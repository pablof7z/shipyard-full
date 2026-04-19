<script lang="ts">
  import { onMount } from 'svelte';
  import { browser } from '$app/environment';
  import { goto } from '$app/navigation';

  const storageKey = 'shipyard.welcome_seen';

  let open = $state(false);

  function dismiss() {
    if (browser) localStorage.setItem(storageKey, '1');
    open = false;
  }

  function goWrite() {
    dismiss();
    goto('/write');
  }

  function onOverlayClick(event: MouseEvent) {
    if (event.target === event.currentTarget) dismiss();
  }

  function onKeyDown(event: KeyboardEvent) {
    if (event.key === 'Escape' && open) dismiss();
  }

  onMount(() => {
    if (!localStorage.getItem(storageKey)) {
      open = true;
    }
  });
</script>

<svelte:window onkeydown={onKeyDown} />

{#if open}
  <div class="overlay" role="presentation" onclick={onOverlayClick}>
    <div class="card" role="dialog" aria-modal="true" aria-label="Welcome to Shipyard">
      <button class="close" type="button" aria-label="Close" onclick={dismiss}>×</button>

      <header>
        <h2>Welcome to Shipyard</h2>
        <p>A quiet space for loud ideas. Here's what you can do:</p>
      </header>

      <ul class="tiles">
        <li>
          <span class="num">01</span>
          <div>
            <strong>Schedule posts</strong>
            <p>Write now, publish later. Pick any time &mdash; Shipyard posts it for you.</p>
          </div>
        </li>
        <li>
          <span class="num">02</span>
          <div>
            <strong>Set up queues</strong>
            <p>Daily, weekly, whatever. Drop drafts in and they go out on schedule.</p>
          </div>
        </li>
        <li>
          <span class="num">03</span>
          <div>
            <strong>Post as a team</strong>
            <p>Invite teammates from Settings. They draft, you review and approve in one tap.</p>
          </div>
        </li>
        <li>
          <span class="num">04</span>
          <div>
            <strong>Let an agent post for you</strong>
            <p>Grab an agent prompt from Settings &rarr; Agents. Your AI drafts, you approve.</p>
          </div>
        </li>
      </ul>

      <div class="actions">
        <button type="button" class="primary" onclick={goWrite}>Write your first post</button>
        <button type="button" class="ghost" onclick={dismiss}>Take a look around</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 1000;
    background: rgba(0, 0, 0, 0.6);
    backdrop-filter: blur(4px);
    -webkit-backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    animation: fade 0.15s ease;
  }

  @keyframes fade {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  .card {
    position: relative;
    width: 100%;
    max-width: 520px;
    background: var(--bg-secondary);
    color: var(--text-primary);
    border: 1px solid var(--border-default);
    border-radius: 14px;
    padding: 32px;
    box-shadow: 0 20px 48px rgba(0, 0, 0, 0.45);
    animation: rise 0.18s ease;
  }

  @keyframes rise {
    from { opacity: 0; transform: translateY(12px) scale(0.98); }
    to { opacity: 1; transform: translateY(0) scale(1); }
  }

  .close {
    position: absolute;
    top: 12px;
    right: 14px;
    background: none;
    border: none;
    color: var(--text-tertiary);
    font-size: 22px;
    line-height: 1;
    width: 28px;
    height: 28px;
    border-radius: 6px;
    cursor: pointer;
    font-family: inherit;
  }

  .close:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  header h2 {
    margin: 0 0 6px;
    font-size: 22px;
    font-weight: 800;
    letter-spacing: 0;
  }

  header p {
    margin: 0 0 20px;
    color: var(--text-secondary);
    font-size: 14px;
    line-height: 1.55;
  }

  .tiles {
    list-style: none;
    padding: 0;
    margin: 0 0 24px;
    display: grid;
    gap: 14px;
  }

  .tiles li {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 14px;
    align-items: start;
    padding: 14px;
    border: 1px solid var(--border-subtle);
    border-radius: 10px;
    background: var(--bg-tertiary);
  }

  .num {
    font-size: 13px;
    font-weight: 800;
    color: var(--accent);
    letter-spacing: 0.02em;
    padding-top: 1px;
    font-variant-numeric: tabular-nums;
  }

  .tiles strong {
    display: block;
    font-size: 14px;
    font-weight: 700;
    margin-bottom: 4px;
  }

  .tiles p {
    margin: 0;
    font-size: 13px;
    color: var(--text-secondary);
    line-height: 1.5;
  }

  .actions {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
  }

  .primary,
  .ghost {
    border-radius: 8px;
    padding: 11px 18px;
    font-family: inherit;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    border: 1px solid transparent;
    transition: background 0.15s;
  }

  .primary {
    background: var(--accent);
    color: #ffffff;
  }

  .primary:hover {
    background: var(--accent-hover, #f4806a);
  }

  .ghost {
    background: transparent;
    color: var(--text-secondary);
    border-color: var(--border-default);
  }

  .ghost:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  @media (max-width: 520px) {
    .card {
      padding: 24px;
    }
  }
</style>
