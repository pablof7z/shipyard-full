# Out Of Scope And Future

This file records what is intentionally not part of the reimplementation spec. It prevents product drift.

## Explicitly Out Of Scope

### Analytics And Performance Feedback

Shipyard should not become an analytics product.

Do not add:

- Engagement dashboards.
- Best-time analytics.
- Reach scoring.
- Relay performance charts as user-facing product features.
- Growth recommendations.

Operational logs and worker metrics are still required for engineering observability, but they are not product analytics.

### Non-Blossom Media Providers

Media uploads must use Blossom only.

Do not add:

- Satellite upload.
- NIP-96 upload flow.
- Generic media provider abstraction.
- Provider marketplace.
- App-specific media hosting unless it is Blossom-compatible and selected through the same server list rules.

### Backend-Stored Drafts

Drafts must use NIP-37 draft wraps.

Do not store durable draft content in Shipyard's backend. Local ephemeral editing buffers are acceptable only for crash recovery and must not be treated as the canonical draft store.

### Extra Agent Workflows

Agent support is limited to:

- A `SKILL.md` that teaches agents how to install and run `shipyard-cli`.
- CLI commands that expose normal Shipyard capabilities.

Do not add a separate agent orchestration layer, agent inbox, agent planning UI, agent-specific state machine, or privileged agent workflow.

### DVM Protocol Redesign

The DVM request/feedback surface is part of compatibility.

Do not require:

- New version tags.
- New request kinds.
- New input tags.
- HTTP API fallback.
- Custom status lookup protocol.

Internal validation and reliability improvements are allowed as long as the external kind `5905` behavior remains compatible.

### Editorial Credit System

Delegated proposals need internal audit metadata, but the product should not create a visible authorship or credit system for delegates.

The final Nostr event is authored by the signing owner pubkey.

## Future Ideas That Need Separate Approval

These are plausible later additions, but not requirements:

- Rich roles beyond owner/delegate.
- Approval chains with multiple reviewers.
- Auto-signing policies for trusted delegates.
- Recurring publishing campaigns.
- Programmable queue algorithms.
- NWC payment features.
- Paid DVM access.
- Nostr event deletion workflows.
- Team audit export.
- Organization billing.
- Mobile widgets.

Each future item needs its own product decision before implementation.

