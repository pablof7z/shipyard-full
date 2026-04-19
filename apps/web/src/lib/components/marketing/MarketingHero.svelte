<script lang="ts">
  interface Props {
    ctaLabel: string;
    ctaHref: string;
    onPrimaryCta: (event: MouseEvent) => void;
  }

  let { ctaLabel, ctaHref, onPrimaryCta }: Props = $props();
</script>

<section class="splash">
  <div class="splash-logo">
    <svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
      <path d="M3 2h10l-2 6 2 6H3l2-6-2-6z" />
    </svg>
  </div>
  <div class="splash-wordmark">Shipyard</div>
  <h1>A quiet space for loud ideas.</h1>
  <p class="splash-sub">Schedule Nostr posts, run content queues, and review team drafts &mdash; all in one place.</p>
  <a href={ctaHref} class="splash-cta" onclick={onPrimaryCta}>{ctaLabel}</a>
  <div class="splash-scroll">
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="1.5"
      stroke-linecap="round"
      stroke-linejoin="round"
    >
      <path d="M12 5v14M19 12l-7 7-7-7" />
    </svg>
  </div>
</section>

<style>
  .splash {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 80px clamp(24px, 6vw, 80px);
    text-align: center;
    position: relative;
  }

  .splash-logo {
    width: 56px;
    height: 56px;
    background: var(--accent);
    border-radius: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 20px;
  }

  .splash-logo svg {
    width: 30px;
    height: 30px;
    fill: var(--accent-text-on);
  }

  .splash-wordmark {
    font-size: 4rem;
    font-weight: 900;
    letter-spacing: 0;
    color: var(--text-primary);
    margin-bottom: 24px;
  }

  .splash h1 {
    font-size: 2.25rem;
    font-weight: 600;
    letter-spacing: 0;
    line-height: 1.2;
    color: var(--text-secondary);
    max-width: 700px;
    margin: 0;
  }

  .splash-sub {
    margin: 24px 0 0;
    font-size: 1.125rem;
    line-height: 1.7;
    color: var(--text-secondary);
    max-width: 440px;
  }

  .splash-cta {
    margin-top: 36px;
    background: var(--accent);
    color: var(--accent-text-on);
    border: none;
    border-radius: 7px;
    padding: 14px 28px;
    font-family: inherit;
    font-size: 15px;
    font-weight: 600;
    cursor: pointer;
    text-decoration: none;
    transition: background 0.15s;
  }

  .splash-cta:hover {
    background: var(--accent-hover);
  }

  .splash-scroll {
    position: absolute;
    bottom: 32px;
    left: 50%;
    transform: translateX(-50%);
    color: var(--text-muted);
    display: flex;
    animation: nudge 2s ease-in-out infinite;
  }

  .splash-scroll svg {
    width: 20px;
    height: 20px;
  }

  @keyframes nudge {
    0%, 100% {
      transform: translateX(-50%) translateY(0);
    }
    50% {
      transform: translateX(-50%) translateY(4px);
    }
  }

  @media (max-width: 600px) {
    .splash-wordmark {
      font-size: 2.75rem;
    }
    .splash h1 {
      font-size: 1.75rem;
    }
    .splash-scroll {
      display: none;
    }
  }
</style>
