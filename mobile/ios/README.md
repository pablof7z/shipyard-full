# Shipyard iOS

Native iOS client target.

Implementation standard:

- SwiftUI frontend.
- Shared Rust core from `crates/shipyard-mobile-core` via generated FFI.
- Platform secure storage for local signing secrets.
- Explicit owner signing UX.
- NIP-37 drafts with offline editing cache.
- Blossom-only media upload through the active signer's server list.
- Delegated proposal mode clearly labeled in composer and review flows.

Completion criteria:

- Login.
- Save/delete NIP-37 draft.
- Upload media through Blossom.
- Schedule signed note.
- Create and use queues.
- Create delegated proposal.
- Review, reject, sign, and batch sign proposals.
- See publish success/failure.
- Logout and wipe local secrets/cache.
