export type ComposerDrawerState = 'none' | 'media' | 'drafts';

export type ComposerNote = {
  id: string;
  content: string;
};

export function createComposerNote(content = ''): ComposerNote {
  const id = globalThis.crypto?.randomUUID?.() ?? `note-${Date.now()}-${Math.random()}`;
  return { id, content };
}

export function notesFromContent(content: string): ComposerNote[] {
  return [createComposerNote(content)];
}

export function contentFromNotes(notes: ComposerNote[]): string {
  return notes
    .map((note) => note.content.trimEnd())
    .filter((content) => content.length > 0)
    .join('\n\n');
}
