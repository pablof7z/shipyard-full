use super::*;
use chrono::TimeZone;
use shipyard_core::Pubkey;

fn request_at(created_at: i64) -> DvmRequestEvent {
    DvmRequestEvent {
        id: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
        pubkey: Pubkey::parse("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
            .unwrap(),
        created_at,
        kind: shipyard_core::dvm::DVM_SCHEDULE_KIND,
        tags: vec![],
        content: String::new(),
        sig: Some("sig".into()),
    }
}

#[test]
fn freshness_validation_rejects_requests_older_than_window() {
    let now = Utc.with_ymd_and_hms(2026, 4, 17, 12, 0, 0).unwrap();
    let stale = request_at(now.timestamp() - 601);

    assert!(validate_request_freshness_at(&stale, now, 10).is_err());
}

#[test]
fn freshness_validation_allows_requests_inside_window() {
    let now = Utc.with_ymd_and_hms(2026, 4, 17, 12, 0, 0).unwrap();
    let fresh = request_at(now.timestamp() - 599);

    assert!(validate_request_freshness_at(&fresh, now, 10).is_ok());
}
