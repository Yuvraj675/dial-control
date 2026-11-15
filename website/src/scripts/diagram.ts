// The anatomy diagram: while the dial is pinned, a label and a leader line draw out of each part of it,
// one after another, as the page scrolls (the "parts of a car" diagram). anime.js builds one timeline from
// the callout data and ties it to scroll; the labels are plain HTML in the page, so with no script, on a
// phone or with reduced motion the same facts are a normal list.
import { createTimeline } from 'animejs/timeline';
import { onScroll } from 'animejs/events';
import { createDrawable } from 'animejs/svg';
import { CALLOUTS, anchorPoint } from '../components/diagramData';

const section = document.querySelector<HTMLElement>('[data-diagram]');
const stage = section?.querySelector<HTMLElement>('.diagram__stage') ?? null;
const dial = section?.querySelector<HTMLElement>('[data-dial-diagram]') ?? null;
const svg = section?.querySelector<SVGSVGElement>('.diagram__lines') ?? null;
const labels = Array.from(section?.querySelectorAll<HTMLElement>('.callout') ?? []);

const SVG_NS = 'http://www.w3.org/2000/svg';
const wide = window.matchMedia('(min-width: 72rem) and (prefers-reduced-motion: no-preference)');
let timeline: ReturnType<typeof createTimeline> | null = null;

function teardown() {
  timeline?.revert();
  timeline = null;
  svg?.replaceChildren();
  labels.forEach((li) => { li.removeAttribute('style'); });
}

function build() {
  if (!section || !stage || !dial || !svg || !labels.length) return;
  teardown();
  if (!wide.matches) return;

  const s = stage.getBoundingClientRect();
  const d = dial.getBoundingClientRect();
  const size = d.width;
  const cx = d.left - s.left + size / 2;
  const cy = d.top - s.top + d.height / 2;
  svg.setAttribute('viewBox', `0 0 ${s.width} ${s.height}`);

  const parts = CALLOUTS.map((c) => {
    const li = labels.find((el) => el.dataset.id === c.id);
    if (!li) return null;
    const gap = size * 0.14;
    const w = li.offsetWidth;
    const h = li.offsetHeight;
    const ly = cy + c.y * size;
    const lx = c.side === 'l' ? cx - size / 2 - gap - w : cx + size / 2 + gap;
    li.style.left = `${lx}px`;
    li.style.top = `${ly - h / 2}px`;

    const from = anchorPoint(c.anchor, cx, cy, size);
    const to = { x: c.side === 'l' ? lx + w + 12 : lx - 12, y: ly };
    const line = document.createElementNS(SVG_NS, 'line');
    line.setAttribute('x1', String(from.x)); line.setAttribute('y1', String(from.y));
    line.setAttribute('x2', String(to.x)); line.setAttribute('y2', String(to.y));
    const dot = document.createElementNS(SVG_NS, 'circle');
    dot.setAttribute('cx', String(from.x)); dot.setAttribute('cy', String(from.y)); dot.setAttribute('r', '0');
    svg.append(line, dot);
    return { c, li, line, dot };
  }).filter((p): p is NonNullable<typeof p> => p !== null);

  // Pinned from the top of the section meeting the top of the screen until its bottom meets the bottom.
  const tl = createTimeline({
    defaults: { ease: 'linear' },
    autoplay: onScroll({ target: section, enter: { target: 'top', container: 'top' }, leave: { target: 'bottom', container: 'bottom' }, sync: true }),
  });
  const T = 1000; // timeline length; callouts start at `at` of it
  for (const { c, li, line, dot } of parts) {
    const t = c.at * T;
    const [drawable] = createDrawable(line);
    tl.add(dot, { r: [0, 4.5], duration: 36 }, t)
      .add(drawable, { draw: ['0 0', '0 1'], duration: 80 }, t + 10)
      .add(li, { opacity: [0, 1], translateX: [c.side === 'l' ? 18 : -18, 0], duration: 80 }, t + 60);
  }
  const rest = { x: 0 };
  tl.add(rest, { x: 1, duration: 1 }, T); // hold the finished diagram until the dial lets go
  timeline = tl;
}

// Which part the dial is explaining right now (the dial tells us; see FlyingDial).
window.addEventListener('diagram:current', (e) => {
  const id = (e as CustomEvent<{ id: string }>).detail?.id;
  labels.forEach((li) => li.classList.toggle('is-current', li.dataset.id === id));
});

let pending = 0;
const rebuild = () => { window.clearTimeout(pending); pending = window.setTimeout(build, 120); };

if (section && svg) {
  try {
    void document.fonts?.ready.then(build);
    window.addEventListener('resize', rebuild, { passive: true });
    wide.addEventListener('change', rebuild);
    build();
  } catch {
    // If anything goes wrong the page falls back to the plain list.
    document.documentElement.classList.remove('js-3d');
  }
}
