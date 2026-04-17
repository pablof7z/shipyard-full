const MAIN: &str = include_str!("../src/main.rs");
const BACKOFF: &str = include_str!("../src/backoff.rs");
const PUBLISH: &str = include_str!("../src/publish.rs");

fn implementation() -> String {
    let main = MAIN.split("#[cfg(test)]").next().unwrap_or(MAIN);
    let backoff = BACKOFF.split("#[cfg(test)]").next().unwrap_or(BACKOFF);
    let publish = PUBLISH.split("#[cfg(test)]").next().unwrap_or(PUBLISH);
    format!("{main}\n{backoff}\n{publish}")
}

fn section<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    let (_, after_start) = source
        .split_once(start)
        .unwrap_or_else(|| panic!("missing section start: {start}"));
    let (body, _) = after_start
        .split_once(end)
        .unwrap_or_else(|| panic!("missing section end: {end}"));
    body
}

#[test]
fn terminal_publish_state_changes_are_audited() {
    let source = implementation();

    assert!(
        source.contains("INSERT INTO audit_events"),
        "worker must insert audit_events for terminal publish state changes"
    );
    assert!(
        source.contains("PUBLISH_ITEM_STATE_CHANGE"),
        "audit action must identify publish item state changes"
    );
    assert!(
        source.contains("new_state"),
        "audit metadata must record the terminal new_state"
    );
    assert!(
        source.contains("PUBLISHED") && source.contains("FAILED"),
        "audit events must cover both PUBLISHED and FAILED terminal states"
    );
}

#[test]
fn relay_publish_failures_use_job_retry_path() {
    let source = implementation();
    let process_publish_event = section(
        &source,
        "async fn process_publish_event",
        "async fn record_publish_attempt",
    );

    assert!(
        !process_publish_event.contains("fail_publish_item("),
        "process_publish_event must not mark publish_items FAILED directly; it should return an error for mark_job_failed"
    );
    assert!(
        process_publish_event.contains("PublishJobFailure"),
        "relay/no-relay failures should be represented as retryable publish job failures"
    );
}

#[test]
fn publish_items_fail_only_when_job_retries_are_exhausted() {
    let source = implementation();
    let mark_job_failed = section(&source, "async fn mark_job_failed", "#[derive(Debug)]");

    assert!(
        mark_job_failed.contains("fail_publish_item("),
        "mark_job_failed must own the terminal publish_items FAILED transition"
    );
    assert!(
        mark_job_failed.contains("retries_exhausted"),
        "terminal publish item failure must be guarded by exhausted retry checks"
    );
}

#[test]
fn retry_backoff_is_configurable_and_not_linear() {
    let source = implementation();

    assert!(
        source.contains("SHIPYARD_WORKER_BASE_BACKOFF_SECONDS"),
        "worker retry backoff base must be configurable with an environment variable"
    );
    assert!(
        source.contains("retry_backoff_seconds"),
        "worker should calculate retry delay through a tested backoff helper"
    );
    assert!(
        !source.contains("30 * i64::from(job.attempts)"),
        "retry backoff must not remain the hard-coded linear 30 * attempts formula"
    );
}

#[test]
fn worker_handles_sigterm_shutdown() {
    let source = implementation();

    assert!(
        source.contains("SignalKind::terminate()"),
        "worker must install a SIGTERM handler for graceful container shutdown"
    );
    assert!(
        source.contains("tokio::signal::unix::signal"),
        "worker should use Tokio's Unix signal API for SIGTERM"
    );
}
