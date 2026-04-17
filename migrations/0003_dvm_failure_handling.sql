ALTER TABLE dvm_requests
  ADD COLUMN IF NOT EXISTS failure_code TEXT;

ALTER TABLE dvm_requests
  ADD COLUMN IF NOT EXISTS failure_message TEXT;

UPDATE dvm_requests
SET failure_code = COALESCE(failure_code, 'legacy_error'),
    failure_message = COALESCE(failure_message, error)
WHERE status::text = 'failed'
  AND error IS NOT NULL;
