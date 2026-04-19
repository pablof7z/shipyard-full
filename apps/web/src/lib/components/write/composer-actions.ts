import type { ApiErrorBody, PublishTrigger } from '$lib/api/types';

export function toLocalInput(date: Date): string {
  const local = new Date(date.getTime() - date.getTimezoneOffset() * 60_000);
  return local.toISOString().slice(0, 16);
}

export function apiErrorMessage(err: unknown, fallback: string): string {
  const apiError = err as Partial<ApiErrorBody>;
  if (typeof apiError?.message === 'string') return apiError.message;
  return err instanceof Error ? err.message : fallback;
}

export function parseTagsText(tagsText: string): string[][] {
  const tags = JSON.parse(tagsText) as string[][];
  if (!Array.isArray(tags) || tags.some((tag) => !Array.isArray(tag))) {
    throw new Error('Tags JSON must be an array of tag arrays.');
  }
  return tags;
}

export function publishTimeFor(trigger: PublishTrigger, publishAt: string): string | null {
  return trigger === 'TIME' ? new Date(publishAt).toISOString() : null;
}

export function publishTimeFromSignedEvent(
  trigger: PublishTrigger,
  signedEvent: Record<string, unknown>
): string | null {
  if (trigger !== 'TIME') return null;
  const createdAt = signedEvent.created_at;
  if (typeof createdAt !== 'number' || !Number.isFinite(createdAt)) {
    throw new Error('Signed event JSON must include numeric created_at.');
  }
  return new Date(createdAt * 1000).toISOString();
}

export function queueFor(trigger: PublishTrigger, queueId: string): string | null {
  return trigger === 'QUEUE' ? queueId : null;
}
