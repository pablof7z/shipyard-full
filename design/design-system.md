# Shipyard Design System

## Brand Identity

**Product character:** Warm creative workspace with a strong, opinionated visual identity. Approachable and enjoyable to use, but unmistakably Shipyard. Not generic, not brutalist.

**Accent color:** Coral `#f06449`
- Hover: `#f4806a`
- Muted: `#dc4a30`
- Subtle (dark): `rgba(240, 100, 73, 0.12)`
- Subtle (light): `rgba(240, 100, 73, 0.08)`
- Text on accent: `#ffffff`

**Logo:** Abstract geometric icon + "Shipyard" wordmark in Inter 700. The mark works independently as favicon and app icon. Not literally nautical — suggests structure, building, direction.

## Color System

### Dark Mode

| Token | Value | Use |
|---|---|---|
| `bg-primary` | `#0c0c0c` | Page background |
| `bg-secondary` | `#111111` | List rows, cards |
| `bg-tertiary` | `#161616` | Stat cards, elevated surfaces |
| `bg-hover` | `#1e1e1e` | Interactive hover states |
| `border-subtle` | `#1e1e1e` | Internal dividers, list separators |
| `border-default` | `#262626` | Component edges, inputs |
| `border-strong` | `#333333` | Checkboxes, strong interactive borders |
| `text-primary` | `#ededec` | Headlines, body text, item titles |
| `text-secondary` | `#a1a1a0` | Nav items, secondary info |
| `text-tertiary` | `#636362` | Metadata, timestamps, icons |
| `text-muted` | `#4a4a49` | Placeholders, disabled, section labels |
| `sidebar-bg` | `#0e0e0e` | Sidebar background |
| `sidebar-hover` | `#181818` | Sidebar item hover |
| `sidebar-active` | `#1a1a1a` | Sidebar active item |
| `input-bg` | `#141414` | Input fields, pills |

### Light Mode

| Token | Value | Use |
|---|---|---|
| `bg-primary` | `#ffffff` | Page background |
| `bg-secondary` | `#fafafa` | List rows, cards |
| `bg-tertiary` | `#f5f5f4` | Stat cards, elevated surfaces |
| `bg-hover` | `#f0f0ef` | Interactive hover states |
| `border-subtle` | `#e8e8e7` | Internal dividers |
| `border-default` | `#e0e0df` | Component edges, inputs |
| `border-strong` | `#d0d0cf` | Strong interactive borders |
| `text-primary` | `#1a1a19` | Headlines, body text |
| `text-secondary` | `#5c5c5b` | Nav items, secondary info |
| `text-tertiary` | `#8a8a89` | Metadata, timestamps |
| `text-muted` | `#b0b0af` | Placeholders, disabled |
| `sidebar-bg` | `#f7f7f6` | Sidebar background |
| `sidebar-hover` | `#eeeeed` | Sidebar item hover |
| `sidebar-active` | `#e8e8e7` | Sidebar active item |
| `input-bg` | `#f5f5f4` | Input fields, pills |

### Semantic Status Colors

| Status | Dark | Light |
|---|---|---|
| Scheduled | `#f06449` (accent) | `#f06449` (accent) |
| Proposed | `#fbbf24` | `#b45309` |
| Published | `#34d399` | `#059669` |
| Failed | `#f87171` | `#dc2626` |
| Needs Signature | `#60a5fa` | `#2563eb` |

Status badge backgrounds use the status color at 12% opacity (dark) or 8% opacity (light).

### Both Modes Equally Designed

Neither mode is primary. Both must feel intentional — light mode is not an inverted dark mode. Each has its own carefully tuned palette above.

## Typography

**Typeface:** Inter — single family throughout. No font mixing.

### Type Scale

| Token | Size | Weight | Use |
|---|---|---|---|
| `display` | 22px | 700 | Stat values, key numbers |
| `title` | 16px | 600 | Page titles |
| `heading` | 14px | 600 | Card titles, queue names |
| `body` | 13px | 500 | List items, nav, buttons |
| `caption` | 12px | 500 | Metadata, timestamps, schedule pills |
| `label` | 11px | 600 | Status badges, section labels |
| `micro` | 10px | 600 | Queue slot status |

### Letter Spacing
- Uppercase labels: `0.03–0.05em`
- Display numbers: `-0.02em`
- Everything else: default

### Line Height
- Composer textarea: `1.65`
- UI text: `1.4`
- Labels/badges: `1`

### Numeric Display
Tabular numerals (`font-variant-numeric: tabular-nums`) for all timestamps, counters, and stat values.

## Spacing

Base unit: 4px.

Scale: `4 / 6 / 8 / 10 / 12 / 16 / 20 / 24 / 32 / 48`

| Context | Value |
|---|---|
| Component internal padding | 10–16px |
| Section gaps | 20–24px |
| Main content padding | 24px |
| Sidebar padding | 12–16px |
| Composer top/bottom bars | 10px 16px |

## Component Primitives

### Buttons

| Variant | Background | Border | Text | Radius | Padding |
|---|---|---|---|---|---|
| Primary | `accent` | `accent` | `accent-text-on` | 7px | 7px 14px |
| Secondary | transparent | `border-default` | `text-secondary` | 7px | 7px 14px |
| Ghost | transparent | none | `text-tertiary` | 6px | 6px 10px |
| Small | (any variant) | (any variant) | (any variant) | 6px | 6px 14px, 12px font |

Hover: primary lightens to `accent-hover`, secondary fills with `bg-hover`, ghost fills with `bg-hover`.

### Border Radius Scale

| Token | Value | Use |
|---|---|---|
| `radius-sm` | 4–5px | Badges, cadence pills |
| `radius-md` | 6–7px | Buttons, inputs, sidebar items |
| `radius-lg` | 8px | Cards, list containers |
| `radius-xl` | 12px | Modals, app frame |
| `radius-full` | 20px+ | Account pills, avatars |

### Surfaces

Flat — no box shadows. Hierarchy through background color stepping (`primary` → `secondary` → `tertiary`) and borders. All borders are 1px solid using the three-tier system:
- Subtle: internal dividers, list row separators
- Default: component edges, inputs
- Strong: interactive elements like checkboxes

### Status Badges

- Pill shape, 5px radius, 3px 8px padding
- Tinted background at 12% (dark) / 8% (light) opacity
- Text in status color, 11px weight 600

### Lists

- Container: `border-subtle` border, 8px radius, overflow hidden
- Rows separated by 1px gap in `border-subtle` color
- Row padding: 11px 16px
- Hover: `bg-hover`
- Grid: content (1fr) + status badge (auto) + timestamp (auto)

### Inputs & Pills

- Background: `input-bg`, border: `border-default`, radius: 7px
- Account pills: `border-radius: 20px`
- Schedule picker pills: 7px radius

## Layout

### Sidebar (220px, persistent except in composer)

```
┌──────────────┐
│ [logo] Shipyard │
│              │
│ Dashboard    │
│ Write        │
│ Drafts       │
│              │
│ PUBLISHING   │
│ Scheduled  4 │  ← badge count
│ Queues       │
│ Proposals  3 │
│ Published    │
│              │
│ SETTINGS     │
│ Settings     │
│              │
│ ──────────── │
│ [P] pablo    │  ← account switcher
│     Owner    │
└──────────────┘
```

- Brand: logo mark + wordmark, 16px bottom margin
- Nav items: 7px 8px padding, 6px radius, 13px body weight
- Active: `sidebar-active` bg + accent icon color
- Section labels: 11px uppercase, `text-muted`, 16px top padding
- Badges: accent-filled pill, right-aligned
- Account: pinned bottom, `margin-top: auto`, border-top divider, avatar with accent ring

### Main Content Area

- Header: 16px 24px padding, title left, actions right, `border-subtle` bottom border
- Content: 20–24px padding
- Tab bars: bottom-border style, 2px accent underline on active tab

### Density Modes

**Medium density** (dashboard, queues, proposals): functional and scannable. Stats row, item lists, tab filters. Information-rich without being cramped.

**Low density** (composer): writing dominates. All UI pushed to thin top and bottom bars. Content centered in a 640px column.

## View Patterns

### Dashboard

- Stats row: 4-column grid, 12px gap. Cards show number + sub-label. "Needs Attention" uses `status-error` for the count.
- Upcoming list: mixed-status items sorted by time. Content preview (truncated), status badge, timestamp.
- The "what needs my attention" view.

### Queues

- Tab bar: All / Active / Archived
- Queue cards: name + cadence badge header, horizontal scrolling slot strip
- Slots: 120px wide. Filled = accent border. Empty = dashed border + italic label.
- Each slot shows time, content preview, status.

### Proposals (Owner Inbox)

- Tab bar: Pending Review (count), Signed, Rejected
- Rows add: checkbox (batch signing), delegate npub in accent, inline Sign/Reject buttons
- Batch Sign button in header updates count from selections
- Sign = accent filled, Reject = secondary/ghost (reduces accidental rejection)

### Published / Scheduled

- Same row pattern as dashboard: content preview, status badge, timestamp
- Consistent scanning across all list views

### Settings

- Same card + border + section label patterns
- Covers: relay list, delegate management, Blossom server display, signing connections

## Composer

### Full-Screen, Distraction-Free

When the composer opens, the sidebar disappears. The entire viewport becomes the writing surface.

```
┌─────────────────────────────────────────────────┐
│ [←] [pablo ▾]  Thread · 3 notes    Save Draft 142/280 │  ← thin top bar
│─────────────────────────────────────────────────│
│                                                 │
│         ① The future of social media is...      │
│         │                                       │
│         ─────────────────────────               │
│         ② Three things that make Nostr...       │
│         │  [img]                                │
│         ─────────────────────────               │
│         ③ The tradeoff? We need better...       │
│                                                 │
│         [+] Add another note                    │
│                                                 │
│─────────────────────────────────────────────────│
│ [Media] [Emoji] [Reply]    Daily Thoughts · Next slot [Schedule] │  ← thin bottom bar
└─────────────────────────────────────────────────┘
```

### Thread Structure

- Left gutter: 48px. Numbered circles (24px, accent-filled, white text) connected by 2px vertical line in `border-default`.
- Last note has no line below it.
- Thread content centered at `max-width: 640px`.
- Notes separated by `border-subtle` horizontal rules.
- "Add another note": dashed circle + label, accent on hover.

### Single vs. Multi-Note

- Single note: no gutter, no number, no thread UI. Plain textarea.
- Thread UI appears when user adds a second note.
- Thread counter in top bar hidden for single notes.

### Per-Note Features

- Independent textarea, auto-growing
- Inline media thumbnails (56px) with remove button
- Character count in top bar for active note

### Schedule Integration

- Bottom bar schedule pill: shows queue name + "Next slot" or specific time
- Dropdown to switch: Send Now / Schedule at time / Add to queue
- Button label changes to match: "Publish" / "Schedule" / "Add to Queue"

## Iconography

- Stroke-based, 16px default, 1.5px stroke weight
- Monochrome — uses `text-tertiary` default, `accent` when active
- Consistent set for: dashboard grid, write/pen, document, clock, list/queue, checkmark, gear, media/image, emoji, code brackets, chevrons, plus, close

## Mobile Considerations

The design system applies to mobile with these adaptations:
- Sidebar becomes bottom tab bar or hamburger drawer
- Composer is always full-screen (no sidebar to hide)
- Thread gutter narrows to 36px
- Touch targets: minimum 44px
- Same color tokens, type scale, and component patterns

## Design Principles

1. **Content is king** — The composer strips away everything. Writing is the primary job.
2. **Progressive density** — Airy when creating, dense when managing. Same tokens, different applications.
3. **Status at a glance** — Color-coded badges, counts in the sidebar, attention indicators. Never hunt for what needs action.
4. **Flat and bordered** — No shadows. Hierarchy from background stepping and borders. Clean and modern.
5. **One typeface, well-used** — Inter at different weights and sizes. No font mixing, no decorative type.
6. **Warm but professional** — Coral accent adds character without being playful. This is a tool you take seriously but enjoy using.
7. **Both modes are first-class** — Dark and light each designed intentionally, not derived from each other.
