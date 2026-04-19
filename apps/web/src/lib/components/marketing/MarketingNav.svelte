<script lang="ts">
  interface Props {
    ctaLabel: string;
    onPrimaryCta: (event: MouseEvent) => void;
    onToggleTheme: () => void;
  }

  let { ctaLabel, onPrimaryCta, onToggleTheme }: Props = $props();
</script>

<nav>
  <a href="/" class="nav-brand">
    <div class="nav-logo">
      <svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
        <path d="M3 2h10l-2 6 2 6H3l2-6-2-6z" />
      </svg>
    </div>
    <span class="nav-wordmark">Shipyard</span>
  </a>
  <div class="nav-right">
    <button class="theme-toggle" aria-label="Toggle theme" onclick={onToggleTheme}>
      <svg
        class="icon-moon"
        viewBox="0 0 16 16"
        fill="none"
        stroke="currentColor"
        stroke-width="1.5"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <path d="M14 8.5A6 6 0 0 1 7.5 2a6 6 0 1 0 6.5 6.5z" />
      </svg>
      <svg
        class="icon-sun"
        viewBox="0 0 16 16"
        fill="none"
        stroke="currentColor"
        stroke-width="1.5"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <circle cx="8" cy="8" r="3" />
        <path
          d="M8 1v2M8 13v2M1 8h2M13 8h2M3.05 3.05l1.41 1.41M11.54 11.54l1.41 1.41M3.05 12.95l1.41-1.41M11.54 4.46l1.41-1.41"
        />
      </svg>
    </button>
    <a href="/write" class="nav-cta" onclick={onPrimaryCta}>{ctaLabel}</a>
  </div>
</nav>

<style>
  nav {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    z-index: 100;
    padding: 16px 32px;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .nav-brand {
    display: flex;
    align-items: center;
    gap: 10px;
    text-decoration: none;
    color: var(--text-primary);
  }

  .nav-logo {
    width: 28px;
    height: 28px;
    background: var(--accent);
    border-radius: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .nav-logo svg {
    width: 16px;
    height: 16px;
    fill: var(--accent-text-on);
  }

  .nav-wordmark {
    font-size: 16px;
    font-weight: 700;
    letter-spacing: 0;
  }

  .nav-right {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .theme-toggle {
    background: none;
    border: none;
    padding: 4px;
    cursor: pointer;
    color: var(--text-muted);
    display: flex;
    transition: color 0.15s;
  }

  .theme-toggle:hover {
    color: var(--text-secondary);
  }

  .theme-toggle svg {
    width: 16px;
    height: 16px;
  }

  .theme-toggle .icon-sun {
    display: none;
  }

  :global([data-theme='light']) .theme-toggle .icon-sun {
    display: block;
  }

  :global([data-theme='light']) .theme-toggle .icon-moon {
    display: none;
  }

  .nav-cta {
    background: var(--accent);
    color: var(--accent-text-on);
    border: none;
    border-radius: 7px;
    padding: 8px 18px;
    font-family: inherit;
    font-size: 13px;
    font-weight: 600;
    text-decoration: none;
    transition: background 0.15s;
    cursor: pointer;
  }

  .nav-cta:hover {
    background: var(--accent-hover);
  }

  @media (max-width: 600px) {
    .nav-cta {
      display: none;
    }
  }
</style>
