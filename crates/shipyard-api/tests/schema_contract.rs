const MIGRATION: &str = include_str!("../../../migrations/0001_initial.sql");

#[test]
fn publish_attempts_link_to_jobs_for_retry_audit() {
    assert!(
        MIGRATION.contains("job_id UUID NOT NULL REFERENCES jobs(id) ON DELETE CASCADE"),
        "publish_attempts must store the worker job_id that produced each attempt"
    );
    assert!(
        MIGRATION.contains("CREATE INDEX publish_attempts_job_id_idx ON publish_attempts(job_id)"),
        "publish_attempts must index job_id for retry lookups"
    );
}

#[test]
fn jobs_support_ready_queue_claiming_contract() {
    assert!(
        MIGRATION.contains("state job_status NOT NULL DEFAULT 'ready'"),
        "jobs must expose a state column for durable worker state"
    );
    assert!(
        MIGRATION.contains("available_after TIMESTAMPTZ NOT NULL DEFAULT now()"),
        "jobs must expose available_after for delayed retries and claims"
    );
    assert!(
        MIGRATION.contains("CREATE INDEX jobs_ready_idx ON jobs(state, available_after)"),
        "jobs must index (state, available_after) for SKIP LOCKED claiming"
    );
}

#[test]
fn dvm_requests_store_kind_5905_and_7000_contract_fields() {
    for required in [
        "request_event_id TEXT NOT NULL UNIQUE",
        "encrypted_tags JSONB",
        "decrypted_tags JSONB",
        "relays TEXT[] NOT NULL DEFAULT '{}'",
        "status dvm_request_status NOT NULL DEFAULT 'received'",
        "feedback_event_id TEXT",
        "feedback_content TEXT",
        "feedback_pubkey TEXT",
    ] {
        assert!(
            MIGRATION.contains(required),
            "dvm_requests missing required field/constraint: {required}"
        );
    }
}

#[test]
fn sessions_enforce_valid_expiry_window() {
    assert!(
        MIGRATION.contains("CONSTRAINT sessions_expiry_after_issue CHECK (expires_at > issued_at)"),
        "sessions must reject rows that are already expired when issued"
    );
    assert!(
        MIGRATION.contains("CREATE INDEX sessions_expires_at_idx ON sessions(expires_at)"),
        "sessions must index expires_at for expired-session cleanup"
    );
    assert!(
        MIGRATION.contains("CREATE INDEX sessions_active_idx ON sessions(user_pubkey, expires_at) WHERE revoked_at IS NULL"),
        "sessions must index active lookups by user_pubkey and expires_at"
    );
}
