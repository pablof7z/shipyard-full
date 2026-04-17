-- Support recovery of stuck DVM requests left in 'processing' status.
-- The application reclaims rows whose updated_at is older than 5 minutes,
-- so we add a partial index to make that query efficient.
CREATE INDEX IF NOT EXISTS dvm_requests_stale_processing_idx
  ON dvm_requests(dvm_pubkey, updated_at)
  WHERE status = 'processing';

-- Support efficient lookup of legacy pending rows with no dvm_pubkey
-- (rows migrated from pre-0002 schema that were backfilled with empty string).
CREATE INDEX IF NOT EXISTS dvm_requests_legacy_pending_idx
  ON dvm_requests(created_at)
  WHERE status = 'pending' AND dvm_pubkey = '';
