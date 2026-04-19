<script lang="ts">
  interface Props {
    ctaLabel: string;
    onPrimaryCta: (event: MouseEvent) => void;
  }

  let { ctaLabel, onPrimaryCta }: Props = $props();
</script>

<section class="final-cta">
  <h2>Schedule your Nostr posts.</h2>
  <a href="/write" class="final-cta-btn" onclick={onPrimaryCta}>{ctaLabel}</a>
</section>

<footer>
  <div class="footer-brand">Shipyard</div>
  <ul class="footer-links">
    <li><a href="/">Docs</a></li>
    <li><a href="/SKILL.md">Agent skill</a></li>
    <li><a href="/">Source</a></li>
  </ul>
</footer>

<style>
  .final-cta {
    background: var(--accent);
    padding: clamp(80px, 12vw, 140px) clamp(24px, 6vw, 80px);
    text-align: center;
  }

  .final-cta h2 {
    font-size: 3.5rem;
    font-weight: 900;
    letter-spacing: 0;
    line-height: 1;
    color: var(--accent-text-on);
    margin: 0 0 32px;
  }

  .final-cta-btn {
    display: inline-flex;
    background: var(--accent-text-on);
    color: var(--accent);
    border: none;
    border-radius: 7px;
    padding: 14px 28px;
    font-family: inherit;
    font-size: 15px;
    font-weight: 700;
    text-decoration: none;
    transition: opacity 0.15s;
    cursor: pointer;
  }

  .final-cta-btn:hover {
    opacity: 0.9;
  }

  footer {
    padding: 32px clamp(24px, 6vw, 80px);
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .footer-brand {
    color: var(--text-muted);
    font-size: 13px;
    font-weight: 600;
  }

  .footer-links {
    display: flex;
    gap: 24px;
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .footer-links a {
    color: var(--text-tertiary);
    text-decoration: none;
    font-size: 13px;
    font-weight: 500;
    transition: color 0.15s;
  }

  .footer-links a:hover {
    color: var(--text-primary);
  }

  @media (max-width: 600px) {
    .final-cta h2 {
      font-size: 2.25rem;
    }
    footer {
      flex-direction: column;
      gap: 16px;
    }
  }
</style>
