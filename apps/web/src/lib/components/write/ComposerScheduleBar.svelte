<script lang="ts">
  import type { PublishTrigger, Queue } from '$lib/api/types';
  import type { ComposerDrawerState } from './composer';

  let {
    activeDrawer,
    activeQueues,
    disabled,
    publishAt,
    queueId,
    saving,
    submitLabel,
    trigger,
    onPublishAtChange,
    onQueueChange,
    onSubmit,
    onToggleDrawer,
    onTriggerChange
  }: {
    activeDrawer: ComposerDrawerState;
    activeQueues: Queue[];
    disabled: boolean;
    publishAt: string;
    queueId: string;
    saving: boolean;
    submitLabel: string;
    trigger: PublishTrigger;
    onPublishAtChange: (value: string) => void;
    onQueueChange: (value: string) => void;
    onSubmit: () => void;
    onToggleDrawer: (drawer: Exclude<ComposerDrawerState, 'none'>) => void;
    onTriggerChange: (value: PublishTrigger) => void;
  } = $props();

  function selectValue(event: Event) {
    return (event.currentTarget as HTMLSelectElement).value;
  }

  function inputValue(event: Event) {
    return (event.currentTarget as HTMLInputElement).value;
  }
</script>

<form
  class="composer-bottombar"
  onsubmit={(event) => {
    event.preventDefault();
    onSubmit();
  }}
>
  <div class="composer-tools" aria-label="Composer tools">
    <button
      class:active={activeDrawer === 'media'}
      type="button"
      title="Media"
      onclick={() => onToggleDrawer('media')}
    >
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <rect x="1.5" y="2.5" width="13" height="11" rx="1.5" />
        <circle cx="5.5" cy="6.5" r="1.5" />
        <path d="M1.5 11l3.5-3 2.5 2 3-4 4 5" />
      </svg>
      <span>Media</span>
    </button>
    <button
      class:active={activeDrawer === 'drafts'}
      type="button"
      title="Drafts"
      onclick={() => onToggleDrawer('drafts')}
    >
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <path d="M3 2.5h7l3 3v8H3z" />
        <path d="M10 2.5v3h3M5 8h6M5 10.5h4" />
      </svg>
      <span>Drafts</span>
    </button>
    <button
      class:active={activeDrawer === 'advanced'}
      type="button"
      title="Advanced"
      onclick={() => onToggleDrawer('advanced')}
    >
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <path d="M6 4L2.5 8 6 12M10 4l3.5 4L10 12" />
      </svg>
      <span>Advanced</span>
    </button>
  </div>

  <div class="composer-publish-group">
    <div class="schedule-pill">
      <select
        aria-label="Publish mode"
        value={trigger}
        onchange={(event) => onTriggerChange(selectValue(event) as PublishTrigger)}
      >
        <option value="TIME">Schedule</option>
        <option value="QUEUE">Add to queue</option>
      </select>
    </div>

    {#if trigger === 'TIME'}
      <input
        class="schedule-input"
        aria-label="Publish time"
        type="datetime-local"
        value={publishAt}
        onchange={(event) => onPublishAtChange(inputValue(event))}
      />
    {:else if trigger === 'QUEUE'}
      <select
        class="schedule-input"
        aria-label="Queue"
        value={queueId}
        onchange={(event) => onQueueChange(selectValue(event))}
      >
        {#each activeQueues as queue}
          <option value={queue.id}>{queue.name}</option>
        {/each}
      </select>
    {/if}

    <button class="primary-action" type="submit" disabled={saving || disabled}>
      {saving ? 'Saving...' : submitLabel}
    </button>
  </div>
</form>
