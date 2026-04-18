# Shipyard Android

Native Android client target. This directory is intentionally still a scaffold:
there is no complete Kotlin app, Gradle project, or generated FFI binding yet.
It is not a web wrapper.

Current foundation:

- Shared Rust crate exists at `crates/shipyard-mobile-core`.
- The crate exports serializable API payloads for device tokens, queue slot
  previews, signed scheduling, and proposals.
- The crate includes NIP-37 draft metadata/delete marker helpers.
- The crate includes Blossom server selection with the required
  `https://blossom.primal.net` fallback only when no valid server is present.
- The crate exposes FFI-safe version and capability functions for future
  Kotlin bindings.

Next commands:

```sh
cd /tmp/shipyard-mobile-core-foundation
cargo test -p shipyard-mobile-core
cargo build -p shipyard-mobile-core
```

Native implementation standard:

- Kotlin and Jetpack Compose frontend.
- Generated bindings around `crates/shipyard-mobile-core`.
- Android secure storage for local signing secrets.
- Android signer integration where available.
- NIP-37 drafts with offline editing cache.
- Blossom-only media upload through the active signer's server list.
- Delegated proposal mode clearly labeled in composer and review flows.

Native completion criteria:

- Login.
- Save/delete NIP-37 draft.
- Upload media through Blossom.
- Schedule signed note.
- Create and use queues.
- Create delegated proposal.
- Review, reject, sign, and batch sign proposals.
- See publish success/failure.
- Logout and wipe local secrets/cache.
