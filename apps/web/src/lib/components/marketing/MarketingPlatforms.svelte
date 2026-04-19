<section class="platforms-section">
  <div class="platforms-label">Works everywhere</div>
  <p class="platforms-sub">
    Web, mobile, and a CLI if you want to script it. Same account, same queues, same schedule.
  </p>
  <div class="platforms-row">
    <span class="plat">Web</span>
    <span class="plat">Mobile</span>
    <span class="plat">CLI</span>
    <span class="plat">Agents</span>
    <span class="plat">DVM</span>
  </div>
</section>

<style>
  .platforms-section {
    padding: clamp(80px, 12vw, 160px) clamp(24px, 6vw, 80px);
    text-align: center;
  }

  .platforms-label {
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-muted);
    margin-bottom: 16px;
  }

  .platforms-sub {
    font-size: 1.0625rem;
    line-height: 1.7;
    color: var(--text-secondary);
    max-width: 480px;
    margin: 0 auto 48px;
  }

  .platforms-row {
    display: flex;
    justify-content: center;
    gap: clamp(24px, 5vw, 64px);
    flex-wrap: wrap;
  }

  .plat {
    font-size: 3.5rem;
    font-weight: 800;
    letter-spacing: 0;
    color: var(--text-muted);
    transition: color 0.2s;
    cursor: default;
  }

  .plat:hover {
    color: var(--text-primary);
  }

  @media (max-width: 600px) {
    .plat {
      font-size: 1.75rem;
    }
  }
</style>
