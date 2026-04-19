import { env } from '$env/dynamic/public';

import type {
  AccountResponse,
  ApiErrorBody,
  AuthEvent,
  BatchSignProposalItem,
  BatchSignProposalResponse,
  CreateProposalRequest,
  DelegateResponse,
  DvmRequest,
  LoginResponse,
  PublishItem,
  Queue,
  QueueNextSlotResponse,
  RelaySettingsResponse,
  ScheduleSignedEventRequest,
  SessionResponse
} from './types';

export const shipyardApiBase =
  env.PUBLIC_SHIPYARD_API_URL ?? 'http://localhost:8080';

type ApiRequestOptions = Omit<RequestInit, 'body'> & {
  token?: string;
  ownerPubkey?: string;
  body?: unknown;
};

function jsonHeaders(options: ApiRequestOptions): HeadersInit {
  const headers = new Headers(options.headers);
  headers.set('accept', 'application/json');

  if (options.body !== undefined) {
    headers.set('content-type', 'application/json');
  }

  if (options.token) {
    headers.set('authorization', `Bearer ${options.token}`);
  }

  if (options.ownerPubkey) {
    headers.set('x-shipyard-owner-pubkey', options.ownerPubkey);
  }

  return headers;
}

async function request<T>(path: string, options: ApiRequestOptions = {}): Promise<T> {
  const response = await fetch(`${shipyardApiBase}${path}`, {
    ...options,
    body: options.body === undefined ? undefined : JSON.stringify(options.body),
    headers: jsonHeaders(options)
  });

  if (!response.ok) {
    const error = (await response.json().catch(() => ({
      code: 'http_error',
      message: `Request failed with ${response.status}`
    }))) as ApiErrorBody;
    throw error;
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return (await response.json()) as T;
}

export const shipyardApi = {
  status: () => request<Record<string, unknown>>('/v1/status'),
  login: (event: AuthEvent) =>
    request<LoginResponse>('/v1/auth/login', { method: 'POST', body: { event } }),
  logout: (token: string) => request<void>('/v1/auth/logout', { method: 'POST', token }),
  session: (token: string) => request<SessionResponse>('/v1/auth/session', { token }),
  accounts: (token: string) => request<AccountResponse>('/v1/accounts', { token }),
  delegates: (token: string, ownerPubkey: string) =>
    request<DelegateResponse[]>(`/v1/accounts/${ownerPubkey}/delegates`, { token }),
  inviteDelegate: (token: string, ownerPubkey: string, delegatePubkey: string) =>
    request<DelegateResponse>(`/v1/accounts/${ownerPubkey}/delegates`, {
      method: 'POST',
      token,
      body: { delegate_pubkey: delegatePubkey }
    }),
  revokeDelegate: (token: string, ownerPubkey: string, delegatePubkey: string) =>
    request<void>(`/v1/accounts/${ownerPubkey}/delegates/${delegatePubkey}`, {
      method: 'DELETE',
      token
    }),
  queues: (token: string, ownerPubkey: string) =>
    request<Queue[]>('/v1/queues', { token, ownerPubkey }),
  createQueue: (
    token: string,
    ownerPubkey: string,
    input: Pick<Queue, 'name' | 'description' | 'cadence_seconds' | 'start_at'>
  ) =>
    request<Queue>('/v1/queues', {
      method: 'POST',
      token,
      ownerPubkey,
      body: input
    }),
  updateQueue: (
    token: string,
    queueId: string,
    input: Partial<Pick<Queue, 'name' | 'description' | 'cadence_seconds' | 'start_at'>>
  ) =>
    request<Queue>(`/v1/queues/${queueId}`, {
      method: 'PATCH',
      token,
      body: input
    }),
  nextQueueSlot: (token: string, queueId: string) =>
    request<QueueNextSlotResponse>(`/v1/queues/${queueId}/next-slot`, { token }),
  archiveQueue: (token: string, queueId: string) =>
    request<Queue>(`/v1/queues/${queueId}/archive`, { method: 'POST', token }),
  proposals: (token: string, ownerPubkey: string) =>
    request<PublishItem[]>('/v1/proposals', { token, ownerPubkey }),
  createProposal: (token: string, input: CreateProposalRequest) =>
    request<PublishItem>('/v1/proposals', { method: 'POST', token, body: input }),
  deleteProposal: (token: string, proposalId: string) =>
    request<void>(`/v1/proposals/${proposalId}`, { method: 'DELETE', token }),
  rejectProposal: (token: string, proposalId: string, reason: string) =>
    request<PublishItem>(`/v1/proposals/${proposalId}/reject`, {
      method: 'POST',
      token,
      body: { reason: reason || null }
    }),
  signProposal: (token: string, proposalId: string, signedEvent: Record<string, unknown>) =>
    request<PublishItem>(`/v1/proposals/${proposalId}/sign`, {
      method: 'POST',
      token,
      body: { signed_event: signedEvent }
    }),
  batchSignProposals: (token: string, items: BatchSignProposalItem[]) =>
    request<BatchSignProposalResponse>('/v1/proposals/batch-sign', {
      method: 'POST',
      token,
      body: { items }
    }),
  publishItems: (token: string, ownerPubkey: string) =>
    request<PublishItem[]>('/v1/publish-items', { token, ownerPubkey }),
  scheduleSignedEvent: (
    token: string,
    ownerPubkey: string,
    input: ScheduleSignedEventRequest
  ) =>
    request<PublishItem>('/v1/publish-items/schedule', {
      method: 'POST',
      token,
      ownerPubkey,
      body: input
    }),
  cancelPublishItem: (token: string, itemId: string) =>
    request<void>(`/v1/publish-items/${itemId}/cancel`, { method: 'POST', token }),
  retryPublishItem: (token: string, itemId: string) =>
    request<PublishItem>(`/v1/publish-items/${itemId}/retry`, { method: 'POST', token }),
  relays: (token: string, ownerPubkey: string) =>
    request<RelaySettingsResponse>('/v1/relays', { token, ownerPubkey }),
  updateRelays: (token: string, ownerPubkey: string, relayUrls: string[]) =>
    request<RelaySettingsResponse>('/v1/relays', {
      method: 'PUT',
      token,
      ownerPubkey,
      body: { relay_urls: relayUrls }
    }),
  dvmRequests: (token: string, ownerPubkey: string) =>
    request<DvmRequest[]>('/v1/dvm/requests', { token, ownerPubkey })
};
