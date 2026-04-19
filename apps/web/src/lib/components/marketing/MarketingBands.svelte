<section class="band band-alt">
  <div class="band-inner">
    <div class="band-number">01</div>
    <div class="band-content">
      <h2>Schedule posts</h2>
      <p>Write a post, pick a time. Shipyard publishes it for you, right on schedule.</p>
    </div>
  </div>
</section>

<section class="band">
  <div class="band-inner">
    <div class="band-number">02</div>
    <div class="band-content">
      <h2>Set up queues</h2>
      <p>Set how often you want to post &mdash; say three times a day. Drop notes in and Shipyard spaces them out for you.</p>
    </div>
  </div>
</section>

<section class="band band-alt">
  <div class="band-inner">
    <div class="band-number">03</div>
    <div class="band-content">
      <h2>Boost older posts</h2>
      <p>Catch a great note 8 hours too early? Schedule a repost &mdash; followers on the other side of the world will thank you.</p>
    </div>
  </div>
</section>

<section class="band">
  <div class="band-inner">
    <div class="band-number">04</div>
    <div class="band-content">
      <h2>Post as a team</h2>
      <p>Invite teammates to draft for your account. They write, you review and approve in one tap.</p>
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
