# Shipyard Voice & Copy Guidelines

## Voice

Shipyard talks like a sharp colleague, not a corporation. Direct, confident, human. We respect the user's time and intelligence. We never pad sentences, never hedge, never use five words when two will do.

Think: the person at work who gives you a straight answer when everyone else is being diplomatic.

### We are:
- **Direct** — Say it plainly. No throat-clearing, no "please note that," no "in order to."
- **Confident** — We know what this product does. No "helps you" or "allows you to" — just say what happens.
- **Human** — Write how people talk. Contractions, short sentences, occasional wit. Never robotic.
- **Honest** — If something failed, say so. If something is limited, say so. Don't sugarcoat.
- **Opinionated** — We make decisions so users don't have to agonize. "We recommend" not "you may want to consider."

### We are not:
- **Cute** — No mascots, no "oopsie," no winking emoji in error messages.
- **Corporate** — No "leverage," "utilize," "streamline," "empower," "seamless."
- **Apologetic** — No "sorry for the inconvenience." Fix the problem, explain what happened.
- **Vague** — No "something went wrong." Say what went wrong.
- **Enthusiastic** — No exclamation points in the UI. Ever. If the product is good, you don't need to shout about it.

## Microcopy Principles

### 1. Say what happened, not what the system did

Bad: "Your event has been successfully added to the publishing queue."
Good: "Queued. Publishing Apr 18 at 10 AM."

Bad: "The signature process has been initiated."
Good: "Waiting for signature."

### 2. Use the user's words, not the system's words

Bad: "PublishItem state transitioned to SCHEDULED."
Good: "Scheduled."

Bad: "Delegate authorization has been created."
Good: "Invited. They can propose posts for your account now."

### 3. Front-load the important word

Bad: "The post has been scheduled for April 18."
Good: "Scheduled for April 18."

Bad: "Your draft has been saved."
Good: "Draft saved."

### 4. Button labels say what will happen

Bad: "Submit" / "Confirm" / "OK"
Good: "Sign" / "Schedule" / "Reject" / "Save Draft"

Bad: "Get Started"
Good: "Write a post"

The button should finish the sentence "I want to..."

### 5. Errors say what went wrong AND what to do

Bad: "An error occurred."
Good: "Relay wss://relay.damus.io didn't respond. Retry or check your relay settings."

Bad: "Invalid input."
Good: "That doesn't look like a pubkey. It should start with npub1 or be 64 hex characters."

Bad: "Unauthorized."
Good: "You don't have permission to sign for this account. Ask the owner to invite you."

### 6. Confirmations are for destructive actions only

Don't confirm: saving, scheduling, sending now, switching accounts.
Do confirm: rejecting a proposal, cancelling a scheduled post, revoking a delegate.

Confirmation copy should state the consequence, not ask a generic question.

Bad: "Are you sure?"
Good: "Cancel this post? It's scheduled to publish in 2 hours."
Good: "Reject this proposal? The delegate will be notified."
Good: "Revoke access? They won't be able to propose posts anymore."

### 7. Empty states tell you what to do, not what's missing

Bad: "No posts found."
Good: "Nothing scheduled yet. Write your first post."

Bad: "No queues."
Good: "No queues set up. Create one to auto-schedule posts at a regular cadence."

Bad: "No proposals."
Good: "No pending proposals. Invite a delegate to start receiving content."

### 8. Loading states are invisible or honest

Prefer skeleton screens over spinners. If you must show text:

Bad: "Loading your content..."
Good: No text. Show the layout skeleton.

If something is taking too long:
Good: "Still connecting to relays..." (after 3+ seconds)
Good: "Waiting for remote signer..." (when NIP-46 is slow)

### 9. Success is quiet

Don't throw a party when something works. The UI should simply update to reflect the new state. A post is scheduled? Show it in the scheduled list. No toast, no confetti, no modal.

Exceptions where a brief inline confirmation helps:
- "Signed" — after signing a proposal (status badge updates)
- "Published to 4 relays" — after immediate publish
- "Draft saved" — brief flash near the save button

These are status updates, not celebrations.

## Specific UI Copy

### Navigation & Labels

| Element | Copy | Not this |
|---|---|---|
| Main nav | Dashboard | Home / Overview |
| Main nav | Write | Compose / Create / New Post |
| Main nav | Drafts | My Drafts / Draft Manager |
| Main nav | Scheduled | Upcoming / Planned |
| Main nav | Queues | Publishing Queues |
| Main nav | Proposals | Pending Proposals / Inbox |
| Main nav | Published | History / Past Posts |
| Main nav | Settings | Preferences / Configuration |
| Section label | Publishing | Content Management |
| Account area | Owner / Delegate | Administrator / Contributor |

### Composer

| Element | Copy |
|---|---|
| Textarea placeholder | "What's on your mind?" |
| Thread placeholder (note 2+) | "Continue the thread..." |
| Account selector label | "Publishing as" |
| Media button | "Media" |
| Add thread note | "Add another note" |
| Thread counter | "Thread · 3 notes" |
| Char count | "142 / 280" (no label, just numbers) |
| Save action | "Save Draft" |
| Publish action | "Publish" |
| Schedule action | "Schedule" |
| Queue action | "Add to Queue" |
| Send now action | "Publish Now" |
| Schedule pill (queue) | "Daily Thoughts · Next slot" |
| Schedule pill (time) | "Apr 18, 10:00 AM" |
| Schedule pill (none) | "Schedule for later" |
| Delegate mode banner | "Proposing for pablo — requires their signature" |

### Dashboard

| Element | Copy |
|---|---|
| Stat: scheduled count label | "Scheduled" |
| Stat: scheduled sub | "Next in 2h 15m" |
| Stat: pending label | "Pending Review" |
| Stat: pending sub | "From 2 delegates" |
| Stat: published label | "Published Today" |
| Stat: published sub | "All relays confirmed" |
| Stat: attention label | "Needs Attention" |
| Stat: attention sub | "Relay failure" |
| Section header | "Upcoming" |
| Section link | "View all" |

### Status Badges

| State | Badge text |
|---|---|
| PROPOSED | "Proposed" |
| NEEDS_SIGNATURE | "Needs Signature" |
| SIGNED / SCHEDULED | "Scheduled" |
| PUBLISHING | "Publishing" |
| PUBLISHED | "Published" |
| FAILED | "Failed" |
| REJECTED | "Rejected" |
| CANCELLED | "Cancelled" |

### Proposals

| Element | Copy |
|---|---|
| Tab: pending | "Pending Review" |
| Tab: signed | "Signed" |
| Tab: rejected | "Rejected" |
| Delegate attribution | "From npub1a3k...9f2x" |
| Sign button | "Sign" |
| Reject button | "Reject" |
| Batch action | "Sign 3 proposals" |
| Reject confirmation | "Reject this proposal? The delegate will be notified." |
| Sign confirmation | None. Signing is not destructive. |
| Batch sign confirmation | "Sign 3 proposals? 1 publishes in less than an hour." |

### Queues

| Element | Copy |
|---|---|
| Cadence display | "Every 24h" / "Every 3 days" |
| Empty slot | "Empty slot" |
| Next slot preview | "Next: Apr 19, 10 AM" |
| Create queue CTA | "+ New Queue" |
| Queue name placeholder | "e.g., Daily Thoughts" |
| Cadence label | "Post every" |
| Start time label | "Starting" |

### Settings

| Element | Copy |
|---|---|
| Relay section title | "Relays" |
| Relay empty state | "No relays configured. Add relays where your posts will be published." |
| Relay import prompt | "Import from your Nostr relay list?" |
| Delegate section title | "Delegates" |
| Invite placeholder | "Paste an npub or hex pubkey" |
| Invite button | "Invite" |
| Revoke button | "Revoke" |
| Revoke confirmation | "Revoke access for npub1a3k...? They won't be able to propose posts anymore." |
| Blossom section title | "Media Servers" |
| Blossom empty state | "No Blossom servers found. Using blossom.primal.net as default." |

### Errors

| Situation | Copy |
|---|---|
| Relay publish failed | "Couldn't reach wss://relay.damus.io. Retry?" |
| All relays failed | "None of your relays accepted this post. Check relay settings." |
| Signature invalid | "Signature doesn't match. The event may have been modified. Re-sign?" |
| Signer timeout | "Remote signer didn't respond. Check your signer app and try again." |
| No relays configured | "No relays set up. Add at least one in Settings before publishing." |
| Delegate not authorized | "You're not authorized to propose for this account." |
| Event validation failed | "This event can't be published — missing required fields." |
| Blossom upload failed | "Upload failed. Your Blossom server returned an error." |
| Network error | "Can't reach the server. Check your connection." |

### Login & Auth

| Element | Copy |
|---|---|
| Login heading | "Sign in to Shipyard" |
| Extension option | "Browser Extension" |
| Private key option | "Private Key" |
| Remote signer option | "Remote Signer" |
| Private key warning | "Your key stays in your browser. We never see it." |
| Remote signer prompt | "Paste your connection string" |
| Logged out state | "Signed out." |
| Session expired | "Session expired. Sign in again." |

### Notifications (Mobile)

| Event | Copy |
|---|---|
| New proposal | "New proposal from npub1a3k... for your review" |
| Proposal signed | "Your proposal was signed and scheduled" |
| Proposal rejected | "Your proposal was rejected" |
| Publish failed | "Post failed to publish — relay error" |
| Signer timeout | "Signing request timed out" |

Keep notifications factual. No "Great news!" or "Uh oh!" prefixes.

## Formatting Rules

1. **Sentence case everywhere.** "Pending review" not "Pending Review." Exception: product name "Shipyard" and proper nouns.
2. **No periods in buttons, labels, badges, or nav items.** Periods only in full sentences (descriptions, empty states, errors).
3. **No exclamation points.** Ever. In any UI element.
4. **Use numerals.** "3 proposals" not "three proposals." "2h 15m" not "two hours and fifteen minutes."
5. **Truncate with ellipsis.** Long content previews get `...` via CSS. Never truncate status labels or action buttons.
6. **Time formatting.** "Today, 4:00 PM" / "Tomorrow, 9 AM" / "Apr 18, 10:00 AM" — relative when close, absolute when far.
7. **Pubkey display.** `npub1a3k...9f2x` — first 9 chars + last 4. Never show full hex in the UI unless the user explicitly requests it.
8. **Relay display.** `wss://relay.damus.io` — full URL, no trimming. Relays are technical; show them accurately.

## Words We Use / Words We Don't

| Use | Don't use |
|---|---|
| Post | Content, item, object |
| Write | Compose, create, author |
| Schedule | Plan, queue up (when talking about time-based) |
| Queue | Pipeline, workflow |
| Sign | Approve, authorize (in signing context) |
| Reject | Decline, deny |
| Propose | Submit, send for review |
| Delegate | Contributor, collaborator, team member |
| Owner | Admin, manager, account holder |
| Relay | Server (in Nostr context) |
| Failed | Error occurred, something went wrong |
| Cancel | Remove, delete (for unpublished items) |
| Invite | Add, grant access |
| Revoke | Remove access, uninvite |

## Tone by Context

| Context | Tone | Example |
|---|---|---|
| Writing/composing | Quiet, invisible | Placeholder: "What's on your mind?" then get out of the way |
| Managing/reviewing | Factual, efficient | "3 proposals pending. 1 publishes tomorrow." |
| Errors | Direct, helpful | "Relay didn't respond. Retry or check settings." |
| Destructive actions | Clear, consequential | "Cancel this post? It's scheduled for 2 hours from now." |
| Empty states | Helpful, forward-looking | "Nothing scheduled yet. Write your first post." |
| Settings | Matter-of-fact | "Add relays where your posts will be published." |
| Onboarding | Welcoming but brief | "Sign in to Shipyard" — then let them explore |
