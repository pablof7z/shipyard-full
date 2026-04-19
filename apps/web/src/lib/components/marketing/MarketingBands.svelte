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
      <p>Create a queue with a cadence. Drop posts in and they go out on schedule.</p>
    </div>
  </div>
</section>

<section class="band band-alt">
  <div class="band-inner">
    <div class="band-number">03</div>
    <div class="band-content">
      <h2>Repost on a delay</h2>
      <p>See something good? Schedule a repost for later so followers in other timezones see it too.</p>
    </div>
  </div>
</section>

<section class="band">
  <div class="band-inner">
    <div class="band-number">04</div>
    <div class="band-content">
      <h2>Post as a team</h2>
      <p>Invite people to draft posts for your account. They propose, you review and sign.</p>
    </div>
  </div>
</section>

<style>
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
    font-size: 10rem;
    font-weight: 900;
    letter-spacing: 0;
    line-height: 0.85;
    color: var(--text-ghost);
    font-variant-numeric: tabular-nums;
    user-select: none;
  }

  .band-content h2 {
    font-size: 2.25rem;
    font-weight: 800;
    letter-spacing: 0;
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
    .band-content h2 {
      font-size: 1.75rem;
    }
  }
</style>
