import { browser } from '$app/environment';
import {
  draftIdFromEvent,
  draftKindFromEvent,
  isDeletedDraftWrap,
  type SignedNostrEvent
} from './drafts';

const localDraftsKey = 'shipyard.local_draft_wraps';

export type LocalDraftWrapRecord = {
  draftId: string;
  ownerPubkey: string;
  targetKind: number;
  updatedAt: string;
  deleted: boolean;
  event: SignedNostrEvent;
};

export function readLocalDraftWraps(ownerPubkey = ''): LocalDraftWrapRecord[] {
  if (!browser) {
    return [];
  }

  const records = parseRecords(localStorage.getItem(localDraftsKey));
  const filtered = ownerPubkey
    ? records.filter((record) => record.ownerPubkey === ownerPubkey)
    : records;

  return filtered.sort((left, right) => right.updatedAt.localeCompare(left.updatedAt));
}

export function upsertLocalDraftWrap(event: SignedNostrEvent): LocalDraftWrapRecord {
  const record = toLocalDraftRecord(event);

  if (!browser) {
    return record;
  }

  const existing = parseRecords(localStorage.getItem(localDraftsKey)).filter(
    (item) => !(item.ownerPubkey === record.ownerPubkey && item.draftId === record.draftId)
  );
  localStorage.setItem(localDraftsKey, JSON.stringify([record, ...existing]));
  window.dispatchEvent(new CustomEvent('shipyard-local-drafts-updated'));

  return record;
}

export function removeLocalDraftWrap(ownerPubkey: string, draftId: string): void {
  if (!browser) {
    return;
  }

  const records = parseRecords(localStorage.getItem(localDraftsKey)).filter(
    (item) => !(item.ownerPubkey === ownerPubkey && item.draftId === draftId)
  );
  localStorage.setItem(localDraftsKey, JSON.stringify(records));
  window.dispatchEvent(new CustomEvent('shipyard-local-drafts-updated'));
}

function toLocalDraftRecord(event: SignedNostrEvent): LocalDraftWrapRecord {
  return {
    draftId: draftIdFromEvent(event),
    ownerPubkey: event.pubkey,
    targetKind: draftKindFromEvent(event),
    updatedAt: new Date(event.created_at * 1000).toISOString(),
    deleted: isDeletedDraftWrap(event),
    event
  };
}

function parseRecords(value: string | null): LocalDraftWrapRecord[] {
  if (!value) {
    return [];
  }

  try {
    const records = JSON.parse(value) as LocalDraftWrapRecord[];
    return Array.isArray(records) ? records.filter(isRecord) : [];
  } catch {
    return [];
  }
}

function isRecord(value: LocalDraftWrapRecord): value is LocalDraftWrapRecord {
  return (
    typeof value?.draftId === 'string' &&
    typeof value.ownerPubkey === 'string' &&
    typeof value.targetKind === 'number' &&
    typeof value.updatedAt === 'string' &&
    typeof value.deleted === 'boolean' &&
    typeof value.event?.id === 'string'
  );
}
