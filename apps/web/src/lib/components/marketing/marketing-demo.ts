export type MarketingDemoRefs = {
  composerBody?: HTMLElement;
  threadLabel?: HTMLElement;
  charCount?: HTMLElement;
  bottomBar?: HTMLElement;
  scheduleBtn?: HTMLElement;
  demoCta?: HTMLElement;
};

const notes = [
  'A quiet space for loud ideas.',
  'Write now, publish later. Set up queues so your posts go out while you sleep. Repost things for people in other timezones.',
  'Simple scheduling for Nostr.'
];

function sleep(ms: number) {
  return new Promise<void>((resolve) => setTimeout(resolve, ms));
}

async function pause(ms: number, isCancelled: () => boolean) {
  await sleep(ms);
  return isCancelled();
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

function typeWords(
  el: HTMLElement,
  text: string,
  speed: number,
  charCount: HTMLElement | undefined,
  isCancelled: () => boolean
): Promise<HTMLElement> {
  return new Promise((resolve) => {
    const words = text.split(' ');
    let index = 0;
    const cursor = document.createElement('span');
    cursor.className = 'cursor';
    el.appendChild(cursor);

    const next = () => {
      if (isCancelled()) {
        cursor.remove();
        resolve(cursor);
        return;
      }
      if (index < words.length) {
        cursor.remove();
        el.appendChild(document.createTextNode((index > 0 ? ' ' : '') + words[index]));
        el.appendChild(cursor);
        if (charCount) {
          charCount.textContent = String(el.textContent?.replace('\u200B', '').length ?? 0);
        }
        index++;
        setTimeout(next, speed + Math.random() * 30);
      } else {
        resolve(cursor);
      }
    };
    next();
  });
}

function noteText(note: HTMLElement) {
  return note.querySelector<HTMLElement>('.note-text');
}

export async function playMarketingDemo(
  refs: MarketingDemoRefs,
  isCancelled: () => boolean = () => false
) {
  const { composerBody, threadLabel, charCount, bottomBar, scheduleBtn, demoCta } = refs;
  if (!composerBody || (await pause(400, isCancelled))) return;

  const first = createNoteEl(0, false);
  composerBody.appendChild(first);
  if (await pause(50, isCancelled)) return;
  first.classList.add('visible');
  if (await pause(200, isCancelled)) return;
  const firstText = noteText(first);
  if (!firstText) return;
  const cursor1 = await typeWords(firstText, notes[0], 55, charCount, isCancelled);
  if (await pause(350, isCancelled)) return;

  first.querySelector('.note-line')?.classList.add('grown');
  threadLabel?.classList.add('visible');
  if (await pause(300, isCancelled)) return;

  cursor1.remove();
  const second = createNoteEl(1, false);
  composerBody.appendChild(second);
  if (await pause(50, isCancelled)) return;
  second.classList.add('visible');
  if (await pause(200, isCancelled)) return;
  const secondText = noteText(second);
  if (!secondText) return;
  const cursor2 = await typeWords(secondText, notes[1], 40, charCount, isCancelled);
  if (await pause(350, isCancelled)) return;

  second.querySelector('.note-line')?.classList.add('grown');
  if (await pause(300, isCancelled)) return;

  cursor2.remove();
  const third = createNoteEl(2, true);
  composerBody.appendChild(third);
  if (await pause(50, isCancelled)) return;
  third.classList.add('visible');
  if (await pause(200, isCancelled)) return;
  const thirdText = noteText(third);
  if (!thirdText) return;
  await typeWords(thirdText, notes[2], 65, charCount, isCancelled);
  if (await pause(500, isCancelled)) return;

  bottomBar?.classList.add('visible');
  if (await pause(300, isCancelled)) return;
  scheduleBtn?.classList.add('pop');
  demoCta?.classList.add('visible');
}
