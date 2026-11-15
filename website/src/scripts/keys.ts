// The keyboard section: the keys start tilted back in 3D and fall flat into place as the section scrolls in
// (anime.js, scroll-synced, one after another). Without script, on a phone or with reduced motion they are just flat.
import { createTimeline } from 'animejs/timeline';
import { onScroll } from 'animejs/events';
import { stagger } from 'animejs/utils';

const section = document.getElementById('keyboard');
const keys = Array.from(section?.querySelectorAll<HTMLElement>('.key') ?? []);
const wide = window.matchMedia('(min-width: 72rem) and (prefers-reduced-motion: no-preference)');
let timeline: ReturnType<typeof createTimeline> | null = null;

function build() {
  timeline?.revert();
  timeline = null;
  keys.forEach((k) => k.removeAttribute('style'));
  if (!section || !keys.length || !wide.matches) return;
  const tl = createTimeline({
    defaults: { ease: 'outQuad' },
    autoplay: onScroll({ target: section, enter: { target: 'top', container: 'bottom' }, leave: { target: 'top', container: 'center' }, sync: true }),
  });
  tl.add(keys, { rotateX: [64, 0], translateZ: [-90, 0], translateY: [26, 0], duration: 600, delay: stagger(70) }, 0);
  timeline = tl;
}

if (section) {
  try {
    wide.addEventListener('change', build);
    build();
  } catch {
    keys.forEach((k) => k.removeAttribute('style'));
  }
}
