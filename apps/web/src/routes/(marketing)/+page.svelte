<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { readShipyardSession, sessionUpdatedEvent } from '$lib/api/session';
  import { loginModal } from '$lib/components/onboarding/loginState.svelte';

  let authed = $state(false);

  let mockBrowser: HTMLElement | undefined = $state();
  let composerBody: HTMLElement | undefined = $state();
  let threadLabel: HTMLElement | undefined = $state();
  let charCount: HTMLElement | undefined = $state();
  let bottomBar: HTMLElement | undefined = $state();
  let scheduleBtn: HTMLElement | undefined = $state();
  let demoCta: HTMLElement | undefined = $state();

  const notes = [
    'A quiet space for loud ideas.',
    'Write now, publish later. Set up queues so your posts go out while you sleep. Repost things for people in other timezones.',
    'Simple scheduling for Nostr.'
  ];

  function refreshSession() {
    const session = readShipyardSession();
    authed = Boolean(session.token && session.ownerPubkey);
  }

  function handlePrimaryCta(event: MouseEvent) {
    event.preventDefault();
    if (authed) {
      goto('/write');
    } else {
      loginModal.show();
    }
  }

  function toggleTheme() {
    const root = document.documentElement;
    const next = root.getAttribute('data-theme') === 'dark' ? 'light' : 'dark';
    root.setAttribute('data-theme', next);
    localStorage.setItem('theme', next);
  }

  function sleep(ms: number) {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }

  function createNoteEl(index: number, isLast: boolean) {
    const note = document.createElement('div');
    note.className = 'thread-note';

    const gutter = document.createElement('div');
    gutter.className = 'note-gutter';

    const circle = document.createElement('div');
    circle.className = 'note-circle';
    circle.textContent = String(index + 1);
    gutter.appendChild(circle);

    if (!isLast) {
      const line = document.createElement('div');
      line.className = 'note-line';
      gutter.appendChild(line);
    }

    const content = document.createElement('div');
    content.className = 'note-content';

    const text = document.createElement('div');
    text.className = 'note-text';
    content.appendChild(text);

    note.appendChild(gutter);
    note.appendChild(content);
    return note;
  }

  function typeWords(el: HTMLElement, text: string, speed: number): Promise<HTMLElement> {
    return new Promise((resolve) => {
      const words = text.split(' ');
      let i = 0;
      const cursor = document.createElement('span');
      cursor.className = 'cursor';
      el.appendChild(cursor);

      const next = () => {
        if (i < words.length) {
          cursor.remove();
          el.appendChild(document.createTextNode((i > 0 ? ' ' : '') + words[i]));
          el.appendChild(cursor);
          if (charCount) charCount.textContent = String(el.textContent?.replace('\u200B', '').length ?? 0);
          i++;
          setTimeout(next, speed + Math.random() * 30);
        } else {
          resolve(cursor);
        }
      };
      next();
    });
  }

  async function animate() {
    if (!composerBody) return;
    await sleep(400);

    const n1 = createNoteEl(0, false);
    composerBody.appendChild(n1);
    await sleep(50);
    n1.classList.add('visible');
    await sleep(200);
    const cursor1 = await typeWords(n1.querySelector('.note-text') as HTMLElement, notes[0], 55);
    await sleep(350);

    const line1 = n1.querySelector('.note-line');
    line1?.classList.add('grown');
    threadLabel?.classList.add('visible');
    await sleep(300);

    cursor1.remove();
    const n2 = createNoteEl(1, false);
    composerBody.appendChild(n2);
    await sleep(50);
    n2.classList.add('visible');
    await sleep(200);
    const cursor2 = await typeWords(n2.querySelector('.note-text') as HTMLElement, notes[1], 40);
    await sleep(350);

    const line2 = n2.querySelector('.note-line');
    line2?.classList.add('grown');
    await sleep(300);

    cursor2.remove();
    const n3 = createNoteEl(2, true);
    composerBody.appendChild(n3);
    await sleep(50);
    n3.classList.add('visible');
    await sleep(200);
    await typeWords(n3.querySelector('.note-text') as HTMLElement, notes[2], 65);
    await sleep(500);

    bottomBar?.classList.add('visible');
    await sleep(300);
    scheduleBtn?.classList.add('pop');
    demoCta?.classList.add('visible');
  }

  onMount(() => {
    refreshSession();
    window.addEventListener(sessionUpdatedEvent, refreshSession);

    const saved = localStorage.getItem('theme');
    if (saved) {
      document.documentElement.setAttribute('data-theme', saved);
    } else if (!document.documentElement.getAttribute('data-theme')) {
      const prefersLight = window.matchMedia('(prefers-color-scheme: light)').matches;
      document.documentElement.setAttribute('data-theme', prefersLight ? 'light' : 'dark');
    }

    let animated = false;
    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting && !animated && mockBrowser) {
            animated = true;
            mockBrowser.classList.add('visible');
            setTimeout(animate, 500);
            observer.disconnect();
          }
        });
      },
      { threshold: 0.2 }
    );

    if (mockBrowser) observer.observe(mockBrowser);

    return () => {
      observer.disconnect();
      window.removeEventListener(sessionUpdatedEvent, refreshSession);
    };
  });

  const ctaLabel = $derived(authed ? 'Open app' : 'Sign in with Nostr');
</script>

<svelte:head>
  <title>Shipyard — Schedule your Nostr posts</title>
  <meta
    name="description"
    content="Schedule posts, set up publishing queues, repost for other timezones, and let collaborators draft for your account. Web, mobile, and CLI."
  />
</svelte:head>

<div class="marketing">
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
      <button class="theme-toggle" aria-label="Toggle theme" onclick={toggleTheme}>
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
      <a href="/write" class="nav-cta" onclick={handlePrimaryCta}>{ctaLabel}</a>
    </div>
  </nav>

  <section class="splash">
    <div class="splash-logo">
      <svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
        <path d="M3 2h10l-2 6 2 6H3l2-6-2-6z" />
      </svg>
    </div>
    <div class="splash-wordmark">Shipyard</div>
    <h1>A quiet space for loud ideas.</h1>
    <p class="splash-sub">Write, schedule, and boost your notes.</p>
    <a href="/write" class="splash-cta" onclick={handlePrimaryCta}>{ctaLabel}</a>
    <div class="splash-scroll">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
        <path d="M12 5v14M19 12l-7 7-7-7" />
      </svg>
    </div>
  </section>

  <section class="demo">
    <div class="mock-browser" bind:this={mockBrowser}>
      <div class="browser-chrome">
        <div class="browser-dots"><span></span><span></span><span></span></div>
        <div class="browser-url">shipyard.app/write</div>
      </div>
      <div class="composer">
        <div class="composer-topbar">
          <span class="topbar-back">&larr;</span>
          <div class="topbar-account">
            <div class="topbar-avatar"></div>
            you
          </div>
          <span class="topbar-thread" bind:this={threadLabel}>Thread &middot; 3 notes</span>
          <div class="topbar-right">
            <span class="topbar-draft">Save draft</span>
            <span class="topbar-count" bind:this={charCount}></span>
          </div>
        </div>
        <div class="composer-body" bind:this={composerBody}></div>
        <div class="composer-bottombar" bind:this={bottomBar}>
          <span class="bottombar-btn">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
              <rect x="3" y="3" width="18" height="18" rx="2" />
              <circle cx="8.5" cy="8.5" r="1.5" />
              <path d="M21 15l-5-5L5 21" />
            </svg>
            Media
          </span>
          <span class="bottombar-btn">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="12" cy="12" r="10" />
              <path d="M8 14s1.5 2 4 2 4-2 4-2" />
              <line x1="9" y1="9" x2="9.01" y2="9" />
              <line x1="15" y1="9" x2="15.01" y2="9" />
            </svg>
          </span>
          <div class="bottombar-spacer"></div>
          <div class="schedule-pill">
            <span class="dot"></span>
            Daily queue &middot; Next slot
          </div>
          <button class="schedule-btn" bind:this={scheduleBtn}>Schedule</button>
        </div>
      </div>
    </div>
    <div class="demo-cta-row" bind:this={demoCta}>
      <a href="/write" class="demo-cta" onclick={handlePrimaryCta}>{ctaLabel}</a>
      <span class="demo-tagline">Simple scheduling for Nostr.</span>
    </div>
  </section>

  <section class="band band-alt">
    <div class="band-inner">
      <div class="band-number">01</div>
      <div class="band-content">
        <h2>Schedule posts</h2>
        <p>Write a post, pick a time. Shipyard publishes it to your relays when the time comes.</p>
      </div>
    </div>
  </section>

  <section class="band">
    <div class="band-inner">
      <div class="band-number">02</div>
      <div class="band-content">
        <h2>Set up queues</h2>
        <p>Create a queue with a cadence — once a day, twice a week, whatever. Drop posts in and they go out on schedule.</p>
      </div>
    </div>
  </section>

  <section class="band band-alt">
    <div class="band-inner">
      <div class="band-number">03</div>
      <div class="band-content">
        <h2>Repost on a delay</h2>
        <p>See something good? Schedule a repost for later so your followers in other timezones see it too. Without an algorithm boosting things, timing matters.</p>
      </div>
    </div>
  </section>

  <section class="band">
    <div class="band-inner">
      <div class="band-number">04</div>
      <div class="band-content">
        <h2>Post as a team</h2>
        <p>Invite people to draft posts for your account. They propose, you review and sign. Nothing publishes without your key.</p>
      </div>
    </div>
  </section>

  <section class="platforms-section">
    <div class="platforms-label">Works everywhere</div>
    <p class="platforms-sub">Web app, mobile app, and a CLI if you want to script it. Same account, same queues, same schedule.</p>
    <div class="platforms-row">
      <span class="plat">Web</span>
      <span class="plat">Mobile</span>
      <span class="plat">CLI</span>
      <span class="plat">Agents</span>
    </div>
  </section>

  <section class="cli-section">
    <div class="cli-inner">
      <div class="cli-label">CLI &amp; Agents</div>
      <p class="cli-desc">Schedule posts, manage queues, and check status from your terminal. Works for scripts, cron jobs, and AI agents.</p>
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

  <section class="final-cta">
    <h2>Schedule your Nostr posts.</h2>
    <a href="/write" class="final-cta-btn" onclick={handlePrimaryCta}>{ctaLabel}</a>
  </section>

  <footer>
    <div class="footer-brand">Shipyard</div>
    <ul class="footer-links">
      <li><a href="/">Docs</a></li>
      <li><a href="/SKILL.md">Agent skill</a></li>
      <li><a href="/">Source</a></li>
    </ul>
  </footer>
</div>

<style>
  :global(html) {
    font-family: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
  }

  .marketing {
    --accent: #f06449;
    --accent-hover: #f4806a;
    --accent-text-on: #ffffff;
    --bg-primary: #0c0c0c;
    --bg-secondary: #111111;
    --bg-tertiary: #161616;
    --border-subtle: #1e1e1e;
    --border-default: #262626;
    --border-strong: #333333;
    --text-primary: #ededec;
    --text-secondary: #a1a1a0;
    --text-tertiary: #636362;
    --text-muted: #4a4a49;
    --text-ghost: #1e1e1e;
    background: var(--bg-primary);
    color: var(--text-primary);
    min-height: 100vh;
    overflow-x: hidden;
  }

  :global([data-theme='light']) .marketing {
    --bg-primary: #ffffff;
    --bg-secondary: #fafafa;
    --bg-tertiary: #f5f5f4;
    --border-subtle: #e8e8e7;
    --border-default: #e0e0df;
    --border-strong: #d0d0cf;
    --text-primary: #1a1a19;
    --text-secondary: #5c5c5b;
    --text-tertiary: #8a8a89;
    --text-muted: #b0b0af;
    --text-ghost: #f0f0ef;
  }

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

  .nav-logo :global(svg) {
    width: 16px;
    height: 16px;
    fill: var(--accent-text-on);
  }

  .nav-wordmark {
    font-size: 16px;
    font-weight: 700;
    letter-spacing: -0.01em;
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

  .theme-toggle :global(svg) {
    width: 16px;
    height: 16px;
  }

  .theme-toggle :global(.icon-sun) {
    display: none;
  }

  .theme-toggle :global(.icon-moon) {
    display: block;
  }

  :global([data-theme='light']) .theme-toggle :global(.icon-sun) {
    display: block;
  }

  :global([data-theme='light']) .theme-toggle :global(.icon-moon) {
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

  .splash-logo :global(svg) {
    width: 30px;
    height: 30px;
    fill: var(--accent-text-on);
  }

  .splash-wordmark {
    font-size: clamp(2.5rem, 6vw, 4rem);
    font-weight: 900;
    letter-spacing: -0.04em;
    color: var(--text-primary);
    margin-bottom: 24px;
  }

  .splash h1 {
    font-size: clamp(1.5rem, 3.5vw, 2.25rem);
    font-weight: 600;
    letter-spacing: -0.02em;
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
    flex-direction: column;
    align-items: center;
    gap: 8px;
    animation: nudge 2s ease-in-out infinite;
  }

  .splash-scroll :global(svg) {
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

  .demo {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 80px clamp(16px, 4vw, 48px) 120px;
  }

  .demo-cta-row {
    margin-top: 32px;
    display: flex;
    align-items: center;
    gap: 16px;
    opacity: 0;
    transform: translateY(8px);
    transition: opacity 0.5s ease, transform 0.5s ease;
  }

  :global(.demo-cta-row.visible) {
    opacity: 1 !important;
    transform: translateY(0) !important;
  }

  .demo-cta {
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

  .demo-cta:hover {
    background: var(--accent-hover);
  }

  .demo-tagline {
    font-size: 14px;
    color: var(--text-tertiary);
    font-weight: 500;
  }

  .mock-browser {
    width: 100%;
    max-width: 740px;
    border: 1px solid var(--border-default);
    border-radius: 12px;
    overflow: hidden;
    background: var(--bg-primary);
    opacity: 0;
    transform: translateY(30px) scale(0.96);
    transition: opacity 0.7s ease, transform 0.7s ease;
  }

  :global(.mock-browser.visible) {
    opacity: 1 !important;
    transform: translateY(0) scale(1) !important;
  }

  .browser-chrome {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 16px;
    background: var(--bg-tertiary);
    border-bottom: 1px solid var(--border-default);
  }

  .browser-dots {
    display: flex;
    gap: 6px;
  }

  .browser-dots span {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--border-strong);
  }

  .browser-url {
    flex: 1;
    margin-left: 8px;
    background: var(--bg-primary);
    border: 1px solid var(--border-subtle);
    border-radius: 6px;
    padding: 5px 12px;
    font-size: 12px;
    color: var(--text-tertiary);
    font-weight: 500;
  }

  .composer {
    display: flex;
    flex-direction: column;
    min-height: 420px;
  }

  .composer-topbar {
    display: flex;
    align-items: center;
    padding: 10px 20px;
    border-bottom: 1px solid var(--border-subtle);
    gap: 12px;
    font-size: 13px;
  }

  .topbar-back {
    color: var(--text-tertiary);
    font-size: 16px;
  }

  .topbar-account {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 3px 10px 3px 3px;
    border: 1px solid var(--border-default);
    border-radius: 20px;
    font-weight: 500;
    color: var(--text-secondary);
    font-size: 12px;
  }

  .topbar-avatar {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: linear-gradient(135deg, var(--accent), var(--accent-hover));
  }

  .topbar-thread {
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 500;
    opacity: 0;
    transition: opacity 0.4s ease;
  }

  :global(.topbar-thread.visible) {
    opacity: 1 !important;
  }

  .topbar-right {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .topbar-draft {
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 500;
  }

  .topbar-count {
    font-size: 12px;
    color: var(--text-tertiary);
    font-variant-numeric: tabular-nums;
    font-weight: 500;
  }

  .composer-body {
    flex: 1;
    padding: 28px 20px 20px 20px;
    display: flex;
    flex-direction: column;
  }

  :global(.thread-note) {
    display: flex;
    gap: 14px;
    opacity: 0;
    transform: translateY(6px);
    transition: opacity 0.35s ease, transform 0.35s ease;
  }

  :global(.thread-note.visible) {
    opacity: 1;
    transform: translateY(0);
  }

  :global(.thread-note + .thread-note) {
    margin-top: 0;
    padding-top: 20px;
    border-top: 1px solid var(--border-subtle);
  }

  :global(.note-gutter) {
    display: flex;
    flex-direction: column;
    align-items: center;
    flex-shrink: 0;
    width: 24px;
    padding-top: 2px;
  }

  :global(.note-circle) {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: var(--accent);
    color: var(--accent-text-on);
    font-size: 11px;
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  :global(.note-line) {
    width: 2px;
    background: var(--border-default);
    flex: 1;
    margin-top: 4px;
    margin-bottom: -16px;
    transform-origin: top;
    transform: scaleY(0);
    transition: transform 0.4s ease;
  }

  :global(.note-line.grown) {
    transform: scaleY(1);
  }

  :global(.note-content) {
    flex: 1;
    min-height: 40px;
    padding-top: 2px;
  }

  :global(.note-text) {
    font-size: 15px;
    line-height: 1.65;
    color: var(--text-primary);
    white-space: pre-wrap;
  }

  :global(.note-text .cursor) {
    display: inline-block;
    width: 2px;
    height: 1.1em;
    background: var(--accent);
    margin-left: 1px;
    vertical-align: text-bottom;
    animation: blink 0.6s step-end infinite;
  }

  @keyframes blink {
    50% { opacity: 0; }
  }

  .composer-bottombar {
    display: flex;
    align-items: center;
    padding: 10px 20px;
    border-top: 1px solid var(--border-subtle);
    gap: 10px;
    opacity: 0;
    transition: opacity 0.5s ease;
  }

  :global(.composer-bottombar.visible) {
    opacity: 1 !important;
  }

  .bottombar-btn {
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 500;
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .bottombar-btn :global(svg) {
    width: 14px;
    height: 14px;
  }

  .bottombar-spacer {
    flex: 1;
  }

  .schedule-pill {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    border: 1px solid var(--border-default);
    border-radius: 7px;
    font-size: 11px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .schedule-pill .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
  }

  .schedule-btn {
    background: var(--accent);
    color: var(--accent-text-on);
    border: none;
    border-radius: 6px;
    padding: 6px 14px;
    font-size: 12px;
    font-weight: 600;
    font-family: inherit;
    cursor: pointer;
    transition: transform 0.15s ease;
  }

  :global(.schedule-btn.pop) {
    animation: popIn 0.3s ease;
  }

  @keyframes popIn {
    0% { transform: scale(1); }
    50% { transform: scale(1.08); }
    100% { transform: scale(1); }
  }

  .band {
    padding: clamp(64px, 10vw, 120px) clamp(24px, 6vw, 80px);
  }

  .band-alt {
    background: var(--bg-secondary);
  }

  .band-inner {
    max-width: 1100px;
    margin: 0 auto;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 48px;
    align-items: center;
  }

  .band-number {
    font-size: clamp(6rem, 14vw, 12rem);
    font-weight: 900;
    letter-spacing: -0.05em;
    line-height: 0.85;
    color: var(--text-ghost);
    font-variant-numeric: tabular-nums;
    user-select: none;
  }

  .band-content h2 {
    font-size: clamp(1.5rem, 3vw, 2.25rem);
    font-weight: 800;
    letter-spacing: -0.03em;
    line-height: 1.1;
    margin: 0 0 16px;
  }

  .band-content p {
    font-size: 1.0625rem;
    line-height: 1.75;
    color: var(--text-secondary);
    max-width: 420px;
    margin: 0;
  }

  .band:nth-of-type(even) .band-inner {
    direction: rtl;
  }

  .band:nth-of-type(even) .band-inner > * {
    direction: ltr;
  }

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
    font-size: clamp(2rem, 5vw, 3.5rem);
    font-weight: 800;
    letter-spacing: -0.03em;
    color: var(--text-muted);
    transition: color 0.2s;
    cursor: default;
  }

  .plat:hover {
    color: var(--text-primary);
  }

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

  .final-cta {
    background: var(--accent);
    padding: clamp(80px, 12vw, 140px) clamp(24px, 6vw, 80px);
    text-align: center;
  }

  .final-cta h2 {
    font-size: clamp(2rem, 5vw, 3.5rem);
    font-weight: 900;
    letter-spacing: -0.035em;
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

  @media (max-width: 840px) {
    .band-inner {
      grid-template-columns: 1fr;
      gap: 24px;
    }
    .band-number {
      font-size: 5rem;
    }
    .band:nth-of-type(even) .band-inner {
      direction: ltr;
    }
  }

  @media (max-width: 600px) {
    .splash h1 {
      font-size: 2.5rem;
    }
    .splash-scroll {
      display: none;
    }
    .mock-browser {
      border-radius: 10px;
    }
    .browser-url {
      display: none;
    }
    .composer {
      min-height: 360px;
    }
    .demo-cta-row {
      flex-direction: column;
    }
    .plat {
      font-size: 1.75rem;
    }
    .nav-cta {
      display: none;
    }
    .terminal pre {
      padding: 16px;
      font-size: 12px;
    }
    footer {
      flex-direction: column;
      gap: 16px;
    }
  }

  @media (max-width: 420px) {
    .composer-body {
      padding: 20px 14px 14px;
    }
    .composer-topbar {
      padding: 8px 14px;
    }
    .composer-bottombar {
      padding: 8px 14px;
    }
    .topbar-draft {
      display: none;
    }
  }
</style>
