<script lang="ts">
  import type { ComposerNote } from './composer';

  let {
    notes,
    activeIndex,
    onAddNote,
    onFocusNote,
    onRemoveNote,
    onUpdateNote
  }: {
    notes: ComposerNote[];
    activeIndex: number;
    onAddNote: () => void;
    onFocusNote: (index: number) => void;
    onRemoveNote: (index: number) => void;
    onUpdateNote: (index: number, value: string) => void;
  } = $props();

  const isThread = $derived(notes.length > 1);

  function resize(textarea: HTMLTextAreaElement) {
    textarea.style.height = 'auto';
    textarea.style.height = `${textarea.scrollHeight}px`;
  }

  function updateNote(index: number, event: Event) {
    const textarea = event.currentTarget as HTMLTextAreaElement;
    resize(textarea);
    onUpdateNote(index, textarea.value);
  }
</script>

<div class="composer-thread-body" class:thread-mode={isThread}>
  {#each notes as note, index (note.id)}
    <article class="thread-note" class:active={index === activeIndex}>
      {#if isThread}
        <div class="thread-gutter" aria-hidden="true">
          <div class="thread-number">{index + 1}</div>
          <div class="thread-line"></div>
        </div>
      {/if}

      <div class="thread-note-content">
        <textarea
          class="thread-textarea"
          value={note.content}
          rows={isThread ? 4 : 12}
          placeholder={index === 0 ? "What's on your mind?" : 'Continue the thread...'}
          onfocus={() => onFocusNote(index)}
          oninput={(event) => updateNote(index, event)}
        ></textarea>

        {#if isThread}
          <div class="note-actions">
            <span>{note.content.length} / 280</span>
            {#if notes.length > 1}
              <button type="button" onclick={() => onRemoveNote(index)}>Remove</button>
            {/if}
          </div>
        {/if}
      </div>
    </article>

    {#if isThread && index < notes.length - 1}
      <hr class="thread-divider" />
    {/if}
  {/each}

  <button class="thread-add-note" type="button" onclick={onAddNote}>
    <span class="thread-add-icon" aria-hidden="true">+</span>
    <span>Add another note</span>
  </button>
</div>
