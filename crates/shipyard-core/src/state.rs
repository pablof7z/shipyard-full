use crate::model::PublishState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Actor {
    Owner,
    DelegateCreator,
    Worker,
    Dvm,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum StateTransitionError {
    #[error("{actor:?} cannot transition publish item from {from:?} to {to:?}")]
    Forbidden {
        actor: Actor,
        from: PublishState,
        to: PublishState,
    },
}

pub fn can_transition(actor: Actor, from: PublishState, to: PublishState) -> bool {
    use Actor::*;
    use PublishState::*;

    match actor {
        Owner => matches!(
            (from, to),
            (Proposed, Rejected)
                | (Proposed, NeedsSignature)
                | (Proposed, Signed)
                | (Proposed, Scheduled)
                | (Proposed, Cancelled)
                | (NeedsSignature, Signed)
                | (NeedsSignature, Cancelled)
                | (Signed, Cancelled)
                | (Scheduled, NeedsSignature)
                | (Scheduled, Cancelled)
                | (Failed, NeedsSignature)
                | (Failed, Scheduled)
                | (Failed, Publishing)
                | (Failed, Cancelled)
        ),
        DelegateCreator => matches!((from, to), (Proposed, Cancelled) | (Proposed, Proposed)),
        Worker => matches!(
            (from, to),
            (Signed, Scheduled)
                | (Signed, Publishing)
                | (Signed, Failed)
                | (Scheduled, Publishing)
                | (Publishing, Published)
                | (Publishing, Failed)
                | (Failed, Scheduled)
                | (Failed, Publishing)
        ),
        Dvm => matches!((from, to), (Signed, Scheduled) | (Signed, Failed)),
    }
}

pub fn assert_transition(
    actor: Actor,
    from: PublishState,
    to: PublishState,
) -> Result<(), StateTransitionError> {
    if can_transition(actor, from, to) {
        Ok(())
    } else {
        Err(StateTransitionError::Forbidden { actor, from, to })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::PublishState::*;

    #[test]
    fn owner_can_reject_or_sign_proposals() {
        assert!(can_transition(Actor::Owner, Proposed, Rejected));
        assert!(can_transition(Actor::Owner, Proposed, Signed));
        assert!(can_transition(Actor::Owner, Proposed, Scheduled));
    }

    #[test]
    fn delegate_cannot_sign_or_reject() {
        assert!(!can_transition(Actor::DelegateCreator, Proposed, Signed));
        assert!(!can_transition(Actor::DelegateCreator, Proposed, Rejected));
    }

    #[test]
    fn worker_can_publish_but_not_approve() {
        assert!(can_transition(Actor::Worker, Publishing, Published));
        assert!(!can_transition(Actor::Worker, Proposed, Signed));
    }

    #[test]
    fn published_items_are_terminal_for_current_product() {
        for to in [
            Proposed,
            Rejected,
            NeedsSignature,
            Signed,
            Scheduled,
            Publishing,
            Failed,
            Cancelled,
        ] {
            assert!(!can_transition(Actor::Owner, Published, to));
            assert!(!can_transition(Actor::Worker, Published, to));
        }
    }
}
