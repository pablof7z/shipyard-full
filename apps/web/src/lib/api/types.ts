export type PublishState =
  | 'PROPOSED'
  | 'REJECTED'
  | 'NEEDS_SIGNATURE'
  | 'SIGNED'
  | 'SCHEDULED'
  | 'PUBLISHING'
  | 'PUBLISHED'
  | 'FAILED'
  | 'CANCELLED';

export type PublishTrigger = 'SEND_NOW' | 'TIME' | 'QUEUE' | 'DVM';

export type AccountRelationship = 'owner' | 'delegate';

export type AuthorizedAccount = {
  owner_pubkey: string;
  relationship: AccountRelationship;
  can_propose: boolean;
  can_sign: boolean;
};

export type AccountResponse = {
  user_pubkey: string;
  accounts: AuthorizedAccount[];
};

export type LoginResponse = {
  session_token: string;
  user_pubkey: string;
  expires_at: string;
};

export type SessionResponse = {
  user_pubkey: string;
  expires_at: string;
};

export type DelegateResponse = {
  delegate_pubkey: string;
  status: string;
  created_at: string;
  revoked_at: string | null;
};

export type Queue = {
  id: string;
  owner_pubkey: string;
  name: string;
  description: string | null;
  cadence_seconds: number;
  start_at: string;
  archived_at: string | null;
};

export type QueueNextSlotResponse = {
  queue_id: string;
  owner_pubkey: string;
  next_slot: string;
  latest_queue_slot: string | null;
  now: string;
};

export type PublishItem = {
  id: string;
  owner_pubkey: string;
  created_by_pubkey: string;
  state: PublishState;
  trigger: PublishTrigger;
  unsigned_event_json: Record<string, unknown> | null;
  signed_event_json: Record<string, unknown> | null;
  event_id: string | null;
  publish_time: string | null;
  queue_id: string | null;
  published_at: string | null;
  published_to: string[];
  failure_code: string | null;
  failure_message: string | null;
  created_at: string;
  updated_at: string;
};

export type RelaySettingsResponse = {
  owner_pubkey: string;
  relay_urls: string[];
};

export type AuthEvent = {
  id?: string | null;
  pubkey: string;
  created_at: number;
  kind: number;
  tags: string[][];
  content: string;
  sig?: string | null;
};

export type CreateProposalRequest = {
  owner_pubkey: string;
  unsigned_event: Record<string, unknown>;
  trigger: PublishTrigger;
  publish_time: string | null;
  queue_id: string | null;
};

export type ScheduleSignedEventRequest = {
  signed_event: Record<string, unknown>;
  trigger: PublishTrigger;
  publish_time: string | null;
  queue_id: string | null;
};

export type BatchSignProposalItem = {
  proposal_id: string;
  signed_event: Record<string, unknown>;
};

export type BatchSignProposalResponse = {
  results: {
    proposal_id: string;
    item: PublishItem | null;
    error: ApiErrorBody | null;
  }[];
};

export type DvmRequest = {
  id: string;
  request_event_id: string;
  request_pubkey: string;
  encrypted: boolean;
  raw_request_event: Record<string, unknown>;
  status: string;
  error: string | null;
  created_at: string;
};

export type ApiErrorBody = {
  code: string;
  message: string;
  details?: unknown;
  request_id?: string;
};
