<script lang="ts">
  import { onMount } from 'svelte';
  import { playMarketingDemo } from './marketing-demo';
  import './marketing-demo.css';
  import './marketing-demo-notes.css';

  interface Props {
    ctaLabel: string;
    onPrimaryCta: (event: MouseEvent) => void;
  }

  let { ctaLabel, onPrimaryCta }: Props = $props();

  let mockBrowser: HTMLElement | undefined = $state();
  let composerBody: HTMLElement | undefined = $state();
  let threadLabel: HTMLElement | undefined = $state();
  let charCount: HTMLElement | undefined = $state();
  let bottomBar: HTMLElement | undefined = $state();
  let scheduleBtn: HTMLElement | undefined = $state();
  let demoCta: HTMLElement | undefined = $state();

  onMount(() => {
    let animated = false;
    let cancelled = false;
    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting && !animated && mockBrowser) {
            animated = true;
            mockBrowser.classList.add('visible');
            setTimeout(() => {
              playMarketingDemo(
                { composerBody, threadLabel, charCount, bottomBar, scheduleBtn, demoCta },
                () => cancelled
              );
            }, 500);
            observer.disconnect();
          }
        });
      },
      { threshold: 0.2 }
    );

    if (mockBrowser) observer.observe(mockBrowser);

    return () => {
      cancelled = true;
      observer.disconnect();
    };
  });
</script>

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
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <rect x="3" y="3" width="18" height="18" rx="2" />
            <circle cx="8.5" cy="8.5" r="1.5" />
            <path d="M21 15l-5-5L5 21" />
          </svg>
          Media
        </span>
        <span class="bottombar-btn">
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
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
    <a href="/write" class="demo-cta" onclick={onPrimaryCta}>{ctaLabel}</a>
    <span class="demo-tagline">Simple scheduling for Nostr.</span>
  </div>
</section>
