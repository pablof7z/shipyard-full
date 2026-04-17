use shipyard_core::Pubkey;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::routes::models::{require_account_access, ApiState, AppError, AuthenticatedSession};
use crate::routes::publish_items::PublishItemResponse;

pub(super) async fn require_proposal_mutation_access(
    state: &ApiState,
    session: &AuthenticatedSession,
    item: &PublishItemResponse,
    allow_owner_edit: bool,
) -> Result<(), AppError> {
    if item.state != "PROPOSED" {
        return Err(AppError::bad_request(
            "proposal_not_editable",
            "Only proposed items can be changed.",
        ));
    }

    if allow_owner_edit && session.user_pubkey == item.owner_pubkey {
        return Ok(());
    }

    if session.user_pubkey == item.created_by_pubkey {
        require_account_access(state, session, &item.owner_pubkey).await?;
        return Ok(());
    }

    Err(AppError::forbidden(
        "proposal_not_owned",
        "You can only edit your own proposed items.",
    ))
}

pub(super) async fn insert_revision(
    tx: &mut Transaction<'_, Postgres>,
    publish_item_id: Uuid,
    edited_by_pubkey: &Pubkey,
    event_json: &serde_json::Value,
    reason: Option<&str>,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO proposal_revisions
           (publish_item_id, edited_by_pubkey, event_json, reason)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(publish_item_id)
    .bind(edited_by_pubkey.as_str())
    .bind(event_json)
    .bind(reason)
    .execute(&mut **tx)
    .await?;

    Ok(())
}
