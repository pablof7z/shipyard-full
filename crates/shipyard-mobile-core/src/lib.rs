pub use shipyard_core::{
    next_queue_slot, AccountRelationship, Actor, AuthorizedAccount, NostrEvent, Pubkey,
    PublishItem, PublishState, PublishTrigger, Queue,
};

#[no_mangle]
pub extern "C" fn shipyard_mobile_core_version_major() -> u32 {
    0
}
