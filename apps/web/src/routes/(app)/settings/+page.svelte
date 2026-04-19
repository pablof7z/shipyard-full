<script lang="ts">
  import { env } from '$env/dynamic/public';
  import { onMount } from 'svelte';
  import { shipyardApi, shipyardApiBase } from '$lib/api/client';
  import {
    clearShipyardSession,
    compactPubkey,
    readShipyardSession,
    writeShipyardSession
  } from '$lib/api/session';
  import type {
    AccountResponse,
    ApiErrorBody,
    AuthEvent,
    AuthorizedAccount,
    DelegateResponse,
    SessionResponse
  } from '$lib/api/types';
  import { signNostrEventWithNdk } from '$lib/nostr/signing';

  let token = $state('');
  let ownerPubkey = $state('');
  let loginEventText = $state('');
  let delegatePubkey = $state('');
  let relayText = $state('');
  let accounts = $state<AuthorizedAccount[]>([]);
  let delegates = $state<DelegateResponse[]>([]);
  let sessionInfo = $state<SessionResponse | null>(null);
  let loading = $state(false);
  let saving = $state(false);
  let message = $state('');
  let error = $state('');
  let agentPromptCopied = $state(false);
  let skillUrl = $state('');
  let agentPrompt = $state('');
  const authDomain = env.PUBLIC_SHIPYARD_AUTH_DOMAIN ?? 'localhost';
  const authUrl =
    env.PUBLIC_SHIPYARD_AUTH_URL ?? `${shipyardApiBase}/v1/auth/login`;
  const loginPlaceholder = `{"kind":27235,"pubkey":"...","tags":[["domain","${authDomain}"]],"content":"Sign in to Shipyard."}`;

  function setError(err: unknown, fallback: string) {
    error = (err as ApiErrorBody).message ?? fallback;
    message = '';
  }

  function setMessage(value: string) {
    message = value;
    error = '';
  }

  function accountLabel(account: AuthorizedAccount) {
    return `${compactPubkey(account.owner_pubkey)} (${account.relationship})`;
  }

  function parseRelayText() {
    return relayText
      .split(/[\n,]/)
      .map((relay) => relay.trim())
      .filter(Boolean);
  }

  async function loadSettings() {
    const saved = readShipyardSession();
    token = saved.token;
    ownerPubkey = saved.ownerPubkey;

    if (!token) {
      loading = false;
      return;
    }

    loading = true;
    try {
      const [sessionResponse, accountResponse] = await Promise.all([
        shipyardApi.session(token),
        shipyardApi.accounts(token)
      ]);
      sessionInfo = sessionResponse;
      accounts = (accountResponse as AccountResponse).accounts;

      if (!ownerPubkey) {
        ownerPubkey = sessionResponse.user_pubkey;
        writeShipyardSession({ token, ownerPubkey });
      }

      await loadOwnerScopedSettings();
    } catch (err) {
      setError(err, 'Failed to load settings.');
    } finally {
      loading = false;
    }
  }

  async function loadOwnerScopedSettings() {
    if (!token || !ownerPubkey) {
      return;
    }

    const relayResponse = await shipyardApi.relays(token, ownerPubkey);
    relayText = relayResponse.relay_urls.join('\n');

    const activeAccount = accounts.find((account) => account.owner_pubkey === ownerPubkey);
    if (activeAccount?.relationship === 'owner') {
      delegates = await shipyardApi.delegates(token, ownerPubkey);
    } else {
      delegates = [];
    }
  }

  async function saveSession(event: SubmitEvent) {
    event.preventDefault();
    writeShipyardSession({ token, ownerPubkey });
    setMessage('Session settings saved.');
    await loadSettings();
  }

  async function loginWithEvent(event: SubmitEvent) {
    event.preventDefault();
    saving = true;
    try {
      const authEvent = JSON.parse(loginEventText) as AuthEvent;
      const response = await shipyardApi.login(authEvent);
      token = response.session_token;
      ownerPubkey = response.user_pubkey;
      writeShipyardSession({ token, ownerPubkey });
      loginEventText = '';
      setMessage('Signed login event accepted.');
      await loadSettings();
    } catch (err) {
      setError(err, 'Login event was not accepted.');
    } finally {
      saving = false;
    }
  }

  async function loginWithExtension() {
    saving = true;
    try {
      if (!window.nostr?.getPublicKey || !window.nostr?.signEvent) {
        throw { message: 'No NIP-07 signer extension is available.' } as ApiErrorBody;
      }

      const pubkey = await window.nostr.getPublicKey();
      const signedEvent = await signNostrEventWithNdk({
        pubkey,
        created_at: Math.floor(Date.now() / 1000),
        kind: 27235,
        tags: [
          ['domain', authDomain],
          ['method', 'POST'],
          ['u', authUrl]
        ],
        content: 'Sign in to Shipyard.'
      });
      const response = await shipyardApi.login(signedEvent);
      token = response.session_token;
      ownerPubkey = response.user_pubkey;
      writeShipyardSession({ token, ownerPubkey });
      setMessage('Browser signer login accepted.');
      await loadSettings();
    } catch (err) {
      setError(err, 'Browser signer login failed.');
    } finally {
      saving = false;
    }
  }

  async function logout() {
    saving = true;
    try {
      if (token) {
        await shipyardApi.logout(token);
      }
      clearShipyardSession();
      token = '';
      ownerPubkey = '';
      accounts = [];
      delegates = [];
      relayText = '';
      sessionInfo = null;
      setMessage('Session cleared.');
    } catch (err) {
      setError(err, 'Logout failed.');
    } finally {
      saving = false;
    }
  }

  async function changeOwner() {
    writeShipyardSession({ token, ownerPubkey });
    try {
      await loadOwnerScopedSettings();
      setMessage('Active account changed.');
    } catch (err) {
      setError(err, 'Failed to load active account settings.');
    }
  }

  async function saveRelays(event: SubmitEvent) {
    event.preventDefault();
    saving = true;
    try {
      const response = await shipyardApi.updateRelays(token, ownerPubkey, parseRelayText());
      relayText = response.relay_urls.join('\n');
      setMessage('Relay settings saved.');
    } catch (err) {
      setError(err, 'Failed to save relays.');
    } finally {
      saving = false;
    }
  }

  async function inviteDelegate(event: SubmitEvent) {
    event.preventDefault();
    saving = true;
    try {
      await shipyardApi.inviteDelegate(token, ownerPubkey, delegatePubkey);
      delegatePubkey = '';
      delegates = await shipyardApi.delegates(token, ownerPubkey);
      setMessage('Delegate invited.');
    } catch (err) {
      setError(err, 'Failed to invite delegate.');
    } finally {
      saving = false;
    }
  }

  async function revokeDelegate(pubkey: string) {
    saving = true;
    try {
      await shipyardApi.revokeDelegate(token, ownerPubkey, pubkey);
      delegates = await shipyardApi.delegates(token, ownerPubkey);
      setMessage('Delegate revoked.');
    } catch (err) {
      setError(err, 'Failed to revoke delegate.');
    } finally {
      saving = false;
    }
  }

  async function copyAgentPrompt() {
    try {
      await navigator.clipboard.writeText(agentPrompt);
      agentPromptCopied = true;
      setTimeout(() => (agentPromptCopied = false), 1500);
    } catch {
      agentPromptCopied = false;
    }
  }

  onMount(() => {
    skillUrl = `${window.location.origin}/SKILL.md`;
    agentPrompt = `Read ${skillUrl} and follow the instructions.`;
    loadSettings();
  });
</script>

<svelte:head>
  <title>Settings - Shipyard</title>
</svelte:head>

<header class="page-header">
  <div>
    <p class="eyebrow">Account</p>
    <h1>Settings</h1>
  </div>
  <button class="secondary-action" type="button" onclick={loadSettings} disabled={loading}>
    Refresh
  </button>
</header>

{#if message}
  <section class="notice success">{message}</section>
{/if}
{#if error}
  <section class="notice error">{error}</section>
{/if}

<section class="panel stack">
  <div class="two-column">
    <form class="card-form" onsubmit={saveSession}>
      <div class="section-header">
        <h2>Session</h2>
        <button class="primary-action" type="submit" disabled={saving}>Save</button>
      </div>

      <label class="field">
        <span>Session token</span>
        <input bind:value={token} autocomplete="off" placeholder="UUID from /v1/auth/login" />
      </label>

      <label class="field">
        <span>Active owner pubkey</span>
        <input bind:value={ownerPubkey} autocomplete="off" placeholder="64 hex or npub" />
      </label>

      {#if accounts.length}
        <label class="field">
          <span>Available accounts</span>
          <select bind:value={ownerPubkey} onchange={changeOwner}>
            {#each accounts as account}
              <option value={account.owner_pubkey}>{accountLabel(account)}</option>
            {/each}
          </select>
        </label>
      {/if}

      {#if sessionInfo}
        <p class="meta-line">
          Signed in as {compactPubkey(sessionInfo.user_pubkey)} until {new Date(
            sessionInfo.expires_at
          ).toLocaleString()}.
        </p>
      {/if}

      <div class="inline-actions">
        <button class="secondary-action" type="button" onclick={logout} disabled={saving || !token}>
          Log out
        </button>
      </div>
    </form>

    <form id="login" class="card-form" onsubmit={loginWithEvent}>
      <div class="section-header">
        <h2>Login</h2>
        <div class="inline-actions">
          <button class="secondary-action" type="button" onclick={loginWithExtension} disabled={saving}>
            Browser Signer
          </button>
          <button class="primary-action" type="submit" disabled={saving || !loginEventText.trim()}>
            Login
          </button>
        </div>
      </div>

      <label class="field">
        <span>Nostr auth event JSON</span>
        <textarea
          bind:value={loginEventText}
          rows="9"
          spellcheck="false"
          placeholder={loginPlaceholder}
        ></textarea>
      </label>
    </form>
  </div>

  <div class="two-column">
    <form class="card-form" onsubmit={saveRelays}>
      <div class="section-header">
        <h2>Relays</h2>
        <button class="primary-action" type="submit" disabled={saving || !token || !ownerPubkey}>
          Save Relays
        </button>
      </div>

      <label class="field">
        <span>Relay URLs</span>
        <textarea bind:value={relayText} rows="8" placeholder="wss://relay.example.com"></textarea>
      </label>
    </form>

    <div class="card-form">
      <div class="section-header">
        <h2>Delegates</h2>
      </div>

      <form class="inline-form" onsubmit={inviteDelegate}>
        <input bind:value={delegatePubkey} placeholder="Delegate pubkey" autocomplete="off" />
        <button class="primary-action" type="submit" disabled={saving || !delegatePubkey.trim()}>
          Invite
        </button>
      </form>

      <div class="rows compact">
        {#if !delegates.length}
          <article class="row">
            <p>No delegates for this owner.</p>
          </article>
        {:else}
          {#each delegates as delegate}
            <article class="row">
              <p>{compactPubkey(delegate.delegate_pubkey)}</p>
              <span class="muted-text">{delegate.status}</span>
              <button
                class="danger-action"
                type="button"
                onclick={() => revokeDelegate(delegate.delegate_pubkey)}
                disabled={saving || delegate.status === 'revoked'}
              >
                Revoke
              </button>
            </article>
          {/each}
        {/if}
      </div>
    </div>
  </div>

  <div id="agents" class="card-form agents-card">
    <div class="section-header">
      <h2>Agents</h2>
      <a class="secondary-action" href="/SKILL.md" target="_blank" rel="noopener">View SKILL.md</a>
    </div>

    <p class="agent-copy">
      Shipyard exposes an <a href="https://agentskills.io/specification" target="_blank" rel="noopener">AgentSkills</a>-compliant skill that teaches an AI agent how to
      install the Shipyard CLI and propose posts on your behalf. Paste the prompt below into Claude
      Code, Cursor, or any agent that follows skill URLs.
    </p>

    <label class="field">
      <span>Agent prompt</span>
      <div class="agent-prompt-row">
        <input type="text" readonly value={agentPrompt} />
        <button type="button" class="primary-action" onclick={copyAgentPrompt}>
          {agentPromptCopied ? 'Copied' : 'Copy'}
        </button>
      </div>
    </label>

    <ul class="agent-bullets">
      <li>The agent authenticates with <strong>its own pubkey</strong>, not yours.</li>
      <li>It proposes posts; <strong>you review and sign</strong> from Proposals.</li>
      <li>Invite the agent's pubkey as a delegate above so its proposals reach you.</li>
    </ul>
  </div>
</section>

<style>
  .agents-card {
    margin-top: 24px;
  }

  .agent-copy {
    margin: 8px 0 16px;
    color: var(--text-secondary);
    font-size: 14px;
    line-height: 1.55;
  }

  .agent-copy a {
    color: var(--accent);
  }

  .agent-prompt-row {
    display: flex;
    gap: 8px;
  }

  .agent-prompt-row input {
    flex: 1;
    font-family: 'SF Mono', 'Fira Code', monospace;
    font-size: 13px;
  }

  .agent-bullets {
    margin: 16px 0 0;
    padding: 0 0 0 18px;
    color: var(--text-secondary);
    font-size: 13px;
    line-height: 1.7;
  }
</style>
