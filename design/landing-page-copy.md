# Shipyard Landing Page Copy

## Above the Fold

### Headline

> Schedule your Nostr posts.

### Subheadline

> Write now, publish later. Set up queues so your posts go out while you sleep. Repost things for people in other timezones. Simple scheduling for Nostr.

### Primary CTA

> Sign in with Nostr

---

## What you can do

*Short blocks. No selling, just describing.*

### Schedule posts

> Write a post, pick a time. Shipyard publishes it to your relays when the time comes.

### Set up queues

> Create a queue with a cadence — once a day, twice a week, whatever. Drop posts in and they go out on schedule.

### Repost on a delay

> See something good? Schedule a repost for later so your followers in other timezones see it too. Without an algorithm boosting things, timing matters.

### Let others draft for you

> Invite someone to write posts for your account. They propose, you review and sign. Nothing publishes without your key.

---

## How it works

> **Sign in** with your Nostr key — extension, private key, or remote signer.
>
> **Write a post** or pick something to repost.
>
> **Schedule it** for a specific time, add it to a queue, or publish right now.
>
> Shipyard sends it to your relays and tells you which ones got it.

---

## Works everywhere

> Web app, mobile app, and a CLI if you want to script it. Same account, same queues, same schedule.
>
> If you're already scheduling through NDK's DVM, that still works too. Kind 5905, same as before.

---

## Your keys stay with you

> Shipyard never holds your private key. You sign in the browser, on your phone, or through a remote signer. Your drafts are encrypted on your own relays, not stored in our database. Media uploads go through your Blossom servers.

---

## CLI

> Schedule posts, manage queues, and check status from your terminal. Works for scripts, cron jobs, and AI agents.

```
$ shipyard schedule --content "Hello Nostr" --time "tomorrow 9am"
Scheduled. Publishing Apr 18 at 9:00 AM.
```

```
$ shipyard propose --to npub1you... --content "Draft post" --queue daily
Proposed. Waiting for owner signature.
```

---

## Final CTA

> Sign in with Nostr

---

## Footer

- Docs
- CLI
- Source
- Nostr

---

## Meta

**Page title:** Shipyard — Schedule your Nostr posts

**Meta description:** Schedule posts, set up publishing queues, repost for other timezones, and let collaborators draft for your account. Web, mobile, and CLI.

---

## Copy Notes

**What changed from v1:** Dropped the "publishing cockpit" positioning, the competitive framing, and the self-congratulatory rebuild narrative. This version just says what the thing does. The core insight from Pablo: people want to schedule posts, that's it. The timezone/repost angle is the most relatable "why" — without algorithms, posts decay fast, and scheduling lets you reach people when they're online.

**Tone:** Plain. Like a README that happens to be a landing page. No cleverness, no "finally," no positioning against things that don't exist.

**Deliberately short:** The page doesn't need to convince hard. The audience already wants this. Just show them what it does and let them sign in.

**Add when available:**
- Screenshots paired with each section
- Social proof if notable Nostr accounts use it
- Pricing section when the model is decided
