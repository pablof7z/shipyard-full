ALTER TABLE dvm_requests
  ADD COLUMN IF NOT EXISTS dvm_pubkey TEXT NOT NULL DEFAULT '';

ALTER TABLE dvm_requests
  ADD COLUMN IF NOT EXISTS input_event_id TEXT;

UPDATE dvm_requests
SET input_event_id = request_event_id
WHERE input_event_id IS NULL;

ALTER TABLE dvm_requests
  ALTER COLUMN input_event_id SET NOT NULL;

ALTER TABLE dvm_requests
  ALTER COLUMN dvm_pubkey DROP DEFAULT;

ALTER TABLE dvm_requests
  DROP CONSTRAINT IF EXISTS dvm_requests_request_event_id_key;

CREATE INDEX IF NOT EXISTS dvm_requests_request_event_id_idx
  ON dvm_requests(request_event_id);

ALTER TABLE dvm_requests
  ALTER COLUMN status DROP DEFAULT;

ALTER TYPE dvm_request_status RENAME TO dvm_request_status_old;

CREATE TYPE dvm_request_status AS ENUM ('pending', 'processing', 'succeeded', 'failed');

ALTER TABLE dvm_requests
  ALTER COLUMN status TYPE dvm_request_status
  USING (
    CASE status::text
      WHEN 'received' THEN 'pending'
      WHEN 'scheduled' THEN 'succeeded'
      WHEN 'error' THEN 'failed'
      ELSE status::text
    END
  )::dvm_request_status;

ALTER TABLE dvm_requests
  ALTER COLUMN status SET DEFAULT 'pending';

DROP TYPE dvm_request_status_old;

ALTER TABLE dvm_requests
  ADD CONSTRAINT dvm_requests_input_event_dvm_pubkey_key UNIQUE (input_event_id, dvm_pubkey);

CREATE INDEX IF NOT EXISTS dvm_requests_dvm_status_idx
  ON dvm_requests(dvm_pubkey, status, created_at);
