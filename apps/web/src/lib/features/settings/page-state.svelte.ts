import { shipyardApi } from '$lib/api/client';
import {
  clearShipyardSession,
  readShipyardSession,
  writeShipyardSession
} from '$lib/api/session';
import type {
  AccountResponse,
  ApiErrorBody,
  AuthorizedAccount,
  DelegateResponse,
  SessionResponse
} from '$lib/api/types';

type SettingsPageState = {
  token: string;
  ownerPubkey: string;
  delegatePubkey: string;
  accounts: AuthorizedAccount[];
  delegates: DelegateResponse[];
  sessionInfo: SessionResponse | null;
  loading: boolean;
  saving: boolean;
  message: string;
  error: string;
  agentPromptCopied: boolean;
  skillUrl: string;
  agentPrompt: string;
};

export function createSettingsPageState() {
  const state = $state<SettingsPageState>({
    token: '',
    ownerPubkey: '',
    delegatePubkey: '',
    accounts: [],
    delegates: [],
    sessionInfo: null,
    loading: false,
    saving: false,
    message: '',
    error: '',
    agentPromptCopied: false,
    skillUrl: '',
    agentPrompt: ''
  });
  let copyResetTimer: ReturnType<typeof setTimeout> | undefined;

  function setError(err: unknown, fallback: string) {
    state.error = (err as ApiErrorBody).message ?? fallback;
    state.message = '';
  }

  function setMessage(value: string) {
    state.message = value;
    state.error = '';
  }

  async function loadDelegates() {
    if (!state.token || !state.ownerPubkey) {
      state.delegates = [];
      return;
    }

    const activeAccount = state.accounts.find(
      (account) => account.owner_pubkey === state.ownerPubkey
    );
    state.delegates =
      activeAccount?.relationship === 'owner'
        ? await shipyardApi.delegates(state.token, state.ownerPubkey)
        : [];
  }

  async function loadSettings() {
    const saved = readShipyardSession();
    state.token = saved.token;
    state.ownerPubkey = saved.ownerPubkey;

    if (!state.token) {
      state.accounts = [];
      state.delegates = [];
      state.sessionInfo = null;
      state.loading = false;
      return;
    }

    state.loading = true;
    try {
      const [sessionResponse, accountResponse] = await Promise.all([
        shipyardApi.session(state.token),
        shipyardApi.accounts(state.token)
      ]);
      state.sessionInfo = sessionResponse;
      state.accounts = (accountResponse as AccountResponse).accounts;

      if (!state.ownerPubkey) {
        state.ownerPubkey = sessionResponse.user_pubkey;
        writeShipyardSession({ token: state.token, ownerPubkey: state.ownerPubkey });
      }

      await loadDelegates();
    } catch (err) {
      setError(err, "Couldn't load settings.");
    } finally {
      state.loading = false;
    }
  }

  async function logout() {
    state.saving = true;
    try {
      if (state.token) await shipyardApi.logout(state.token);
      clearShipyardSession();
      state.token = '';
      state.ownerPubkey = '';
      state.accounts = [];
      state.delegates = [];
      state.sessionInfo = null;
      setMessage('Signed out.');
    } catch (err) {
      setError(err, "Couldn't sign out.");
    } finally {
      state.saving = false;
    }
  }

  async function changeOwner() {
    writeShipyardSession({ token: state.token, ownerPubkey: state.ownerPubkey });
    try {
      await loadDelegates();
      setMessage('Switched account.');
    } catch (err) {
      setError(err, "Couldn't switch account.");
    }
  }

  async function inviteDelegate(event: SubmitEvent) {
    event.preventDefault();
    state.saving = true;
    try {
      await shipyardApi.inviteDelegate(state.token, state.ownerPubkey, state.delegatePubkey);
      state.delegatePubkey = '';
      state.delegates = await shipyardApi.delegates(state.token, state.ownerPubkey);
      setMessage('Teammate invited.');
    } catch (err) {
      setError(err, "Couldn't invite teammate.");
    } finally {
      state.saving = false;
    }
  }

  async function revokeDelegate(pubkey: string) {
    state.saving = true;
    try {
      await shipyardApi.revokeDelegate(state.token, state.ownerPubkey, pubkey);
      state.delegates = await shipyardApi.delegates(state.token, state.ownerPubkey);
      setMessage('Teammate removed.');
    } catch (err) {
      setError(err, "Couldn't remove teammate.");
    } finally {
      state.saving = false;
    }
  }

  function initializeAgentPrompt(origin: string) {
    state.skillUrl = `${origin}/SKILL.md`;
    state.agentPrompt = `Read ${state.skillUrl} and follow the instructions.`;
  }

  async function copyAgentPrompt() {
    try {
      await navigator.clipboard.writeText(state.agentPrompt);
      state.agentPromptCopied = true;
      if (copyResetTimer) clearTimeout(copyResetTimer);
      copyResetTimer = setTimeout(() => (state.agentPromptCopied = false), 1500);
    } catch {
      state.agentPromptCopied = false;
    }
  }

  function dispose() {
    if (copyResetTimer) clearTimeout(copyResetTimer);
  }

  return {
    state,
    changeOwner,
    copyAgentPrompt,
    dispose,
    initializeAgentPrompt,
    inviteDelegate,
    loadSettings,
    logout,
    revokeDelegate
  };
}
