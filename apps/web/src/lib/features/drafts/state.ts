import type { DraftEntry } from '$lib/ndk/drafts';

// ── Reactive state ──────────────────────────────────────────────────────────
// All state is module-level $state so it persists across route navigations
// within the same browser session.

/** Full list of drafts loaded from the user's private relays. */
export let draftList = $state<DraftEntry[]>([]);

/** The draft currently open in the editor (undefined = none open). */
export let currentDraft = $state<DraftEntry | undefined>(undefined);

/** Whether the current draft has unsaved edits. */
export let isDirty = $state(false);

/** Whether a draft operation (load/save/delete) is in progress. */
export let isLoading = $state(false);

/** Last error message, if any. */
export let draftError = $state<string>('');

// ── Mutations ───────────────────────────────────────────────────────────────

export function setDraftList(entries: DraftEntry[]) {
  draftList = entries;
}

export function openDraft(entry: DraftEntry) {
  currentDraft = entry;
  isDirty = false;
  draftError = '';
}

export function closeDraft() {
  currentDraft = undefined;
  isDirty = false;
}

export function markDirty() {
  isDirty = true;
}

export function removeDraftFromList(id: string) {
  draftList = draftList.filter((d) => d.id !== id);
  if (currentDraft?.id === id) {
    closeDraft();
  }
}

export function upsertDraftInList(entry: DraftEntry) {
  const idx = draftList.findIndex((d) => d.id === entry.id);
  if (idx >= 0) {
    draftList[idx] = entry;
  } else {
    draftList = [entry, ...draftList];
  }
  if (currentDraft?.id === entry.id) {
    currentDraft = entry;
  }
  isDirty = false;
}

export function setLoading(loading: boolean) {
  isLoading = loading;
}

export function setDraftError(msg: string) {
  draftError = msg;
}
