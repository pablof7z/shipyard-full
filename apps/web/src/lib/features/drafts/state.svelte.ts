import type { DraftEntry } from '$lib/ndk/drafts';

type DraftState = {
  draftList: DraftEntry[];
  currentDraft: DraftEntry | undefined;
  isDirty: boolean;
  isLoading: boolean;
  draftError: string;
};

export const draftsState = $state<DraftState>({
  draftList: [],
  currentDraft: undefined,
  isDirty: false,
  isLoading: false,
  draftError: ''
});

export function setDraftList(entries: DraftEntry[]) {
  draftsState.draftList = entries;
}

export function openDraft(entry: DraftEntry) {
  draftsState.currentDraft = entry;
  draftsState.isDirty = false;
  draftsState.draftError = '';
}

export function closeDraft() {
  draftsState.currentDraft = undefined;
  draftsState.isDirty = false;
}

export function markDirty() {
  draftsState.isDirty = true;
}

export function removeDraftFromList(id: string) {
  draftsState.draftList = draftsState.draftList.filter((draft) => draft.id !== id);
  if (draftsState.currentDraft?.id === id) {
    closeDraft();
  }
}

export function upsertDraftInList(entry: DraftEntry) {
  const index = draftsState.draftList.findIndex((draft) => draft.id === entry.id);
  if (index >= 0) {
    draftsState.draftList[index] = entry;
  } else {
    draftsState.draftList = [entry, ...draftsState.draftList];
  }
  if (draftsState.currentDraft?.id === entry.id) {
    draftsState.currentDraft = entry;
  }
  draftsState.isDirty = false;
}

export function setLoading(loading: boolean) {
  draftsState.isLoading = loading;
}

export function setDraftError(message: string) {
  draftsState.draftError = message;
}
