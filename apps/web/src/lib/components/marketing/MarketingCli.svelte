<section class="cli-section">
  <div class="cli-inner">
    <div class="cli-label">CLI &amp; Agents</div>
    <p class="cli-desc">
      Schedule posts, manage queues, and check status from your terminal. Works for scripts, cron
      jobs, and AI agents.
    </p>
    <div class="terminal">
      <div class="terminal-bar">
        <div class="terminal-dot"></div>
        <div class="terminal-dot"></div>
        <div class="terminal-dot"></div>
        <span class="terminal-title">shipyard</span>
      </div>
      <pre><span class="prompt">$</span> shipyard schedule --content "Hello Nostr" --time "tomorrow 9am"
<span class="output">Scheduled. Publishing Apr 20 at 9:00 AM.</span>
<span class="blank"></span><span class="prompt">$</span> shipyard propose --to npub1you... --content "Draft post" --queue daily
<span class="output">Proposed. Waiting for owner signature.</span></pre>
    </div>
  </div>
</section>

<style>
  .cli-section {
    padding: clamp(64px, 10vw, 120px) clamp(24px, 6vw, 80px);
    background: var(--bg-secondary);
  }

  .cli-inner {
    max-width: 720px;
    margin: 0 auto;
  }

  .cli-label {
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-muted);
    margin-bottom: 8px;
  }

  .cli-desc {
    font-size: 1.0625rem;
    line-height: 1.7;
    color: var(--text-secondary);
    margin-bottom: 32px;
    max-width: 480px;
  }

  .terminal {
    background: var(--bg-primary);
    border: 1px solid var(--border-default);
    border-radius: 10px;
    overflow: hidden;
  }

  .terminal-bar {
    padding: 12px 16px;
    border-bottom: 1px solid var(--border-subtle);
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .terminal-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--border-strong);
  }

  .terminal-title {
    font-size: 12px;
    color: var(--text-muted);
    margin-left: 8px;
    font-weight: 500;
  }

  .terminal pre {
    padding: 24px;
    margin: 0;
    font-family: 'SF Mono', 'Fira Code', 'Fira Mono', 'Roboto Mono', monospace;
    font-size: 13.5px;
    line-height: 1.8;
    overflow-x: auto;
    color: var(--text-primary);
  }

  .terminal .prompt {
    color: var(--accent);
    user-select: none;
  }

  .terminal .output {
    color: var(--text-tertiary);
  }

  .terminal .blank {
    display: block;
    height: 20px;
  }

  @media (max-width: 600px) {
    .terminal pre {
      padding: 16px;
      font-size: 12px;
    }
  }
</style>
