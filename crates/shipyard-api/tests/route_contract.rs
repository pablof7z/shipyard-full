const PROPOSALS: &str = include_str!("../src/routes/proposals.rs");
const PUBLISH_ITEM_MODEL: &str = include_str!("../src/routes/publish_items/model.rs");

fn implementation(source: &str) -> &str {
    source.split("#[cfg(test)]").next().unwrap_or(source)
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
fn proposal_create_enqueues_owner_pending_notification() {
    let create = section(
        implementation(PROPOSALS),
        "async fn create_proposal",
        "async fn edit_proposal",
    );

    assert!(
        create.contains("enqueue_owner_pending_proposal_notification"),
        "delegate-created proposals must enqueue a durable owner pending-proposal notification"
    );
    assert!(
        create.contains("session.user_pubkey != owner_pubkey"),
        "owner self-created proposals should not enqueue a pending-proposal notification"
    );
}

#[test]
fn proposal_terminal_mutations_use_state_machine_guard() {
    let proposals = implementation(PROPOSALS);
    let cancel = section(
        proposals,
        "async fn cancel_proposal",
        "async fn reject_proposal",
    );
    let reject = section(
        proposals,
        "async fn reject_proposal",
        "#[derive(Debug, Deserialize)]",
    );

    assert!(
        cancel.contains("assert_publish_transition(actor, &item.state, PublishState::Cancelled)"),
        "proposal cancel must go through the Rust state-machine guard"
    );
    assert!(
        reject.contains(
            "assert_publish_transition(Actor::Owner, &item.state, PublishState::Rejected)"
        ),
        "proposal reject must go through the Rust state-machine guard"
    );
}

#[test]
fn queue_publish_time_uses_core_slot_logic_and_latest_assignment() {
    let publish_model = implementation(PUBLISH_ITEM_MODEL);

    assert!(
        publish_model.contains("next_queue_slot(&queue, Utc::now(), latest_queue_slot)"),
        "QUEUE scheduling must use shipyard-core next_queue_slot"
    );
    assert!(
        publish_model.contains("SELECT max(publish_time)")
            && publish_model.contains("state NOT IN ('REJECTED', 'CANCELLED', 'FAILED')"),
        "QUEUE scheduling must consider the latest active assigned queue slot"
    );
}

#[test]
fn route_mutations_can_call_state_machine_guard() {
    let publish_model = implementation(PUBLISH_ITEM_MODEL);

    assert!(
        publish_model.contains("assert_transition(actor, from, to)"),
        "API publish mutations must share the Rust state-machine guard"
    );
}
