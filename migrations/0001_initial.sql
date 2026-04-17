CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE users (
  pubkey TEXT PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CONSTRAINT users_pubkey_nonempty CHECK (length(pubkey) > 0)
);

CREATE TABLE accounts (
  pubkey TEXT PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CONSTRAINT accounts_pubkey_nonempty CHECK (length(pubkey) > 0)
);

CREATE TABLE sessions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_pubkey TEXT NOT NULL REFERENCES users(pubkey) ON DELETE CASCADE,
  issued_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  expires_at TIMESTAMPTZ NOT NULL,
  revoked_at TIMESTAMPTZ,
  user_agent TEXT,
  created_ip INET,
  CONSTRAINT sessions_expiry_after_issue CHECK (expires_at > issued_at)
);

CREATE INDEX sessions_user_pubkey_idx ON sessions(user_pubkey);
CREATE INDEX sessions_expires_at_idx ON sessions(expires_at);
CREATE INDEX sessions_active_idx ON sessions(user_pubkey, expires_at) WHERE revoked_at IS NULL;

CREATE TYPE delegate_status AS ENUM ('active', 'revoked');

CREATE TABLE account_delegates (
  owner_pubkey TEXT NOT NULL REFERENCES accounts(pubkey) ON DELETE CASCADE,
  delegate_pubkey TEXT NOT NULL REFERENCES users(pubkey) ON DELETE CASCADE,
  created_by_pubkey TEXT NOT NULL REFERENCES users(pubkey) ON DELETE RESTRICT,
  status delegate_status NOT NULL DEFAULT 'active',
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  revoked_at TIMESTAMPTZ,
  PRIMARY KEY (owner_pubkey, delegate_pubkey),
  CONSTRAINT account_delegates_revoked_at_matches_status CHECK (
    (status = 'active' AND revoked_at IS NULL)
    OR (status = 'revoked' AND revoked_at IS NOT NULL)
  )
);

CREATE INDEX account_delegates_delegate_idx ON account_delegates(delegate_pubkey, status);

CREATE TYPE publish_state AS ENUM (
  'PROPOSED',
  'REJECTED',
  'NEEDS_SIGNATURE',
  'SIGNED',
  'SCHEDULED',
  'PUBLISHING',
  'PUBLISHED',
  'FAILED',
  'CANCELLED'
);

CREATE TYPE publish_trigger AS ENUM ('SEND_NOW', 'TIME', 'QUEUE', 'DVM');

CREATE TABLE queues (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  owner_pubkey TEXT NOT NULL REFERENCES accounts(pubkey) ON DELETE CASCADE,
  name TEXT NOT NULL,
  description TEXT,
  cadence_seconds BIGINT NOT NULL,
  start_at TIMESTAMPTZ NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  archived_at TIMESTAMPTZ,
  CONSTRAINT queues_name_nonempty CHECK (length(trim(name)) > 0),
  CONSTRAINT queues_cadence_positive CHECK (cadence_seconds > 0)
);

CREATE INDEX queues_owner_idx ON queues(owner_pubkey, archived_at);

CREATE TABLE relay_settings (
  owner_pubkey TEXT PRIMARY KEY REFERENCES accounts(pubkey) ON DELETE CASCADE,
  relay_urls TEXT[] NOT NULL DEFAULT '{}',
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE publish_items (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  owner_pubkey TEXT NOT NULL REFERENCES accounts(pubkey) ON DELETE CASCADE,
  created_by_pubkey TEXT NOT NULL REFERENCES users(pubkey) ON DELETE RESTRICT,
  state publish_state NOT NULL,
  trigger publish_trigger NOT NULL,
  unsigned_event_json JSONB,
  signed_event_json JSONB,
  event_id TEXT,
  publish_time TIMESTAMPTZ,
  queue_id UUID REFERENCES queues(id) ON DELETE SET NULL,
  published_at TIMESTAMPTZ,
  published_to TEXT[] NOT NULL DEFAULT '{}',
  failure_code TEXT,
  failure_message TEXT,
  failed_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CONSTRAINT publish_items_event_when_published CHECK (
    state NOT IN ('SCHEDULED', 'PUBLISHING', 'PUBLISHED') OR signed_event_json IS NOT NULL
  ),
  CONSTRAINT publish_items_publish_time_when_scheduled CHECK (
    state NOT IN ('SCHEDULED', 'PUBLISHING', 'PUBLISHED') OR publish_time IS NOT NULL
  ),
  CONSTRAINT publish_items_failure_fields CHECK (
    state <> 'FAILED' OR (failure_code IS NOT NULL AND failure_message IS NOT NULL)
  ),
  CONSTRAINT publish_items_no_cancel_published CHECK (
    NOT (state = 'CANCELLED' AND published_at IS NOT NULL)
  )
);

CREATE INDEX publish_items_owner_state_idx ON publish_items(owner_pubkey, state, publish_time);
CREATE INDEX publish_items_state_idx ON publish_items(state);
CREATE INDEX publish_items_due_idx ON publish_items(publish_time) WHERE state = 'SCHEDULED';
CREATE INDEX publish_items_queue_idx ON publish_items(queue_id, publish_time);
CREATE UNIQUE INDEX publish_items_event_id_idx ON publish_items(event_id) WHERE event_id IS NOT NULL;

CREATE TABLE proposal_revisions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  publish_item_id UUID NOT NULL REFERENCES publish_items(id) ON DELETE CASCADE,
  edited_by_pubkey TEXT NOT NULL REFERENCES users(pubkey) ON DELETE RESTRICT,
  event_json JSONB NOT NULL,
  reason TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX proposal_revisions_item_idx ON proposal_revisions(publish_item_id, created_at DESC);

CREATE TYPE job_status AS ENUM ('ready', 'running', 'succeeded', 'failed', 'cancelled');
CREATE TYPE job_type AS ENUM (
  'publish_event',
  'retry_publish_event',
  'expire_signature_request',
  'process_dvm_request',
  'send_notification'
);

CREATE TABLE jobs (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  kind job_type NOT NULL,
  state job_status NOT NULL DEFAULT 'ready',
  available_after TIMESTAMPTZ NOT NULL DEFAULT now(),
  locked_at TIMESTAMPTZ,
  locked_by TEXT,
  attempts INTEGER NOT NULL DEFAULT 0,
  max_attempts INTEGER NOT NULL DEFAULT 5,
  payload JSONB NOT NULL DEFAULT '{}',
  last_error TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CONSTRAINT jobs_attempt_bounds CHECK (attempts >= 0 AND max_attempts > 0)
);

CREATE INDEX jobs_ready_idx ON jobs(state, available_after) WHERE state = 'ready';

CREATE TABLE publish_attempts (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  publish_item_id UUID NOT NULL REFERENCES publish_items(id) ON DELETE CASCADE,
  job_id UUID NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
  attempt INTEGER NOT NULL,
  relay_url TEXT NOT NULL,
  status TEXT NOT NULL,
  error TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  CONSTRAINT publish_attempts_attempt_positive CHECK (attempt > 0),
  CONSTRAINT publish_attempts_relay_nonempty CHECK (length(trim(relay_url)) > 0)
);

CREATE INDEX publish_attempts_item_idx ON publish_attempts(publish_item_id, attempt, relay_url);
CREATE INDEX publish_attempts_job_id_idx ON publish_attempts(job_id);

CREATE TYPE dvm_request_status AS ENUM ('received', 'scheduled', 'error');

CREATE TABLE dvm_requests (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  request_event_id TEXT NOT NULL UNIQUE,
  request_pubkey TEXT NOT NULL,
  encrypted BOOLEAN NOT NULL DEFAULT false,
  encrypted_tags JSONB,
  decrypted_tags JSONB,
  relays TEXT[] NOT NULL DEFAULT '{}',
  raw_request_event JSONB NOT NULL,
  status dvm_request_status NOT NULL DEFAULT 'received',
  error TEXT,
  feedback_event_id TEXT,
  feedback_content TEXT,
  feedback_pubkey TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX dvm_requests_pubkey_idx ON dvm_requests(request_pubkey, created_at DESC);

CREATE TABLE audit_events (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  actor_pubkey TEXT,
  owner_pubkey TEXT,
  action TEXT NOT NULL,
  resource_type TEXT NOT NULL,
  resource_id TEXT,
  metadata JSONB NOT NULL DEFAULT '{}',
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX audit_events_owner_idx ON audit_events(owner_pubkey, created_at DESC);
CREATE INDEX audit_events_actor_idx ON audit_events(actor_pubkey, created_at DESC);

CREATE TABLE device_tokens (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_pubkey TEXT NOT NULL REFERENCES users(pubkey) ON DELETE CASCADE,
  owner_pubkey TEXT REFERENCES accounts(pubkey) ON DELETE CASCADE,
  platform TEXT NOT NULL,
  token TEXT NOT NULL,
  enabled BOOLEAN NOT NULL DEFAULT true,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (platform, token),
  CONSTRAINT device_tokens_platform CHECK (platform IN ('ios', 'android'))
);

CREATE INDEX device_tokens_user_idx ON device_tokens(user_pubkey, enabled);
