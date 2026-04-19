<script lang="ts">
  import { onMount } from 'svelte';
  import { ndk, ensureClientNdk } from '$lib/ndk/client';
  import { User } from '$lib/components/ui/user';
  import { loginModal } from '$lib/components/onboarding/loginState.svelte';
  import { createSettingsPageState } from '$lib/features/settings/page-state.svelte';

  const model = createSettingsPageState();

  let activeRelationship = $derived(
    model.state.accounts.find((account) => account.owner_pubkey === model.state.ownerPubkey)
      ?.relationship ?? null
  );
  let isOwner = $derived(activeRelationship === 'owner');

  onMount(() => {
    model.initializeAgentPrompt(window.location.origin);
    void ensureClientNdk().catch(() => {});
    model.loadSettings();
    return model.dispose;
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
  <button
    class="secondary-action"
    type="button"
    onclick={model.loadSettings}
    disabled={model.state.loading}
  >
    Refresh
  </button>
</header>

{#if model.state.message}
  <section class="notice success">{model.state.message}</section>
{/if}
{#if model.state.error}
  <section class="notice error">{model.state.error}</section>
{/if}

<section class="panel stack">
  <div class="card-form">
    <div class="section-header">
      <h2>Account</h2>
      {#if model.state.token}
        <button
          class="secondary-action"
          type="button"
          onclick={model.logout}
          disabled={model.state.saving}
        >
          Sign out
        </button>
      {:else}
        <button
          class="primary-action"
          type="button"
          onclick={() => loginModal.show()}
        >
          Sign in
        </button>
      {/if}
    </div>

    {#if model.state.token && model.state.ownerPubkey}
      <div class="account-summary">
        <User.Root {ndk} pubkey={model.state.ownerPubkey} class="account-profile">
          <User.Avatar class="avatar-lg" alt="Account avatar" />
          <span class="account-copy">
            <strong><User.Name /></strong>
            {#if model.state.sessionInfo}
              <small>
                Signed in until {new Date(model.state.sessionInfo.expires_at).toLocaleString()}
              </small>
            {/if}
          </span>
        </User.Root>
      </div>

      {#if model.state.accounts.length > 1}
        <label class="field">
          <span>Posting as</span>
          <select bind:value={model.state.ownerPubkey} onchange={model.changeOwner}>
            {#each model.state.accounts as account}
              <option value={account.owner_pubkey}>
                {account.relationship === 'owner' ? 'My account' : 'Team account'}
              </option>
            {/each}
          </select>
        </label>
      {/if}
    {:else}
      <p class="meta-line">Sign in to manage your account.</p>
    {/if}
  </div>

  {#if model.state.token && isOwner}
    <div class="card-form">
      <div class="section-header">
        <h2>Team</h2>
      </div>

      <p class="meta-line">
        Teammates can draft posts that land in your Review queue for approval.
      </p>

      <form class="inline-form" onsubmit={model.inviteDelegate}>
        <input
          bind:value={model.state.delegatePubkey}
          placeholder="Teammate's npub"
          autocomplete="off"
        />
        <button
          class="primary-action"
          type="submit"
          disabled={model.state.saving || !model.state.delegatePubkey.trim()}
        >
          Invite
        </button>
      </form>

      <div class="rows compact">
        {#if !model.state.delegates.length}
          <article class="row">
            <p>No teammates yet.</p>
          </article>
        {:else}
          {#each model.state.delegates as delegate}
            <article class="row">
              <User.Root {ndk} pubkey={delegate.delegate_pubkey} class="account-profile">
                <User.Avatar class="avatar" alt="Teammate avatar" />
                <span class="account-copy">
                  <strong><User.Name /></strong>
                  <small>{delegate.status}</small>
                </span>
              </User.Root>
              <button
                class="danger-action"
                type="button"
                onclick={() => model.revokeDelegate(delegate.delegate_pubkey)}
                disabled={model.state.saving || delegate.status === 'revoked'}
              >
                Remove
              </button>
            </article>
          {/each}
        {/if}
      </div>
    </div>
  {/if}

  <div id="agents" class="card-form agents-card">
    <div class="section-header">
      <h2>Agents</h2>
      <a class="secondary-action" href="/SKILL.md" target="_blank" rel="noopener">View SKILL.md</a>
    </div>

    <p class="agent-copy">
      Shipyard exposes an <a href="https://agentskills.io/specification" target="_blank" rel="noopener">AgentSkills</a>-compliant skill that teaches an AI agent how to
      install the Shipyard CLI and draft posts on your behalf. Paste the prompt below into Claude
      Code, Cursor, or any agent that follows skill URLs.
    </p>

    <label class="field">
      <span>Agent prompt</span>
      <div class="agent-prompt-row">
        <input type="text" readonly value={model.state.agentPrompt} />
        <button type="button" class="primary-action" onclick={model.copyAgentPrompt}>
          {model.state.agentPromptCopied ? 'Copied' : 'Copy'}
        </button>
      </div>
    </label>

    <ul class="agent-bullets">
      <li>The agent authenticates with <strong>its own identity</strong>, not yours.</li>
      <li>It drafts posts; <strong>you review and approve</strong> them from Review.</li>
      <li>Invite the agent as a teammate above so its drafts reach you.</li>
    </ul>
  </div>
</section>

<style>
  .account-summary {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 4px 0 12px;
  }

  :global(.avatar-lg) {
    width: 56px;
    height: 56px;
    border-radius: 50%;
    border: 2px solid var(--accent);
    object-fit: cover;
    background: var(--bg-tertiary);
  }

  :global(.account-profile) {
    display: flex;
    align-items: center;
    gap: 12px;
    text-align: left;
  }

  .account-copy {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .account-copy strong {
    font-size: 15px;
    color: var(--text-primary);
  }

  .account-copy small {
    font-size: 12px;
    color: var(--text-tertiary);
  }

  .agents-card {
    margin-top: 0;
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
