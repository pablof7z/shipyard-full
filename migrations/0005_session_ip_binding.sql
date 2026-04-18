-- Track the IP address from which each session was last used.
-- The existing `created_ip` column records the IP at session creation;
-- `last_ip` is updated on every successful session validation.
-- Both are used by optional strict-IP-binding enforcement.
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS last_ip INET;
