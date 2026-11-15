// The anatomy diagram: while the dial is pinned, a label and a leader line draw out of each part of it,
// one after another, as the page scrolls (the "parts of a car" diagram). anime.js builds one timeline from
// the callout data and ties it to scroll; the labels are plain HTML in the page, so with no script, on a
// phone or with reduced motion the same facts are a normal list.
import { createTimeline } from 'animejs/timeline';
import { onScroll } from 'animejs/events';
import { createDrawable } from 'animejs/svg';
import { CALLOUTS, layoutLeaders } from '../components/diagramData';

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

  // The tested layout: every line leaves its anchor at 45 degrees, then runs level into its label, and no two cross.
  const leaders = layoutLeaders(cx, cy, size);
  const parts = CALLOUTS.map((c, i) => {
    const li = labels.find((el) => el.dataset.id === c.id);
    if (!li) return null;
    const L = leaders[i];
    const w = li.offsetWidth;
    const h = li.offsetHeight;
    li.style.left = `${c.side === 'l' ? L.labelEdgeX - w : L.labelEdgeX}px`;
    li.style.top = `${L.labelY - h / 2}px`;

    const line = document.createElementNS(SVG_NS, 'polyline');
    line.setAttribute('points', L.points.map((p) => `${p.x},${p.y}`).join(' '));
    line.dataset.id = c.id;
    const dot = document.createElementNS(SVG_NS, 'circle');
    dot.setAttribute('cx', String(L.points[0].x)); dot.setAttribute('cy', String(L.points[0].y)); dot.setAttribute('r', '0');
    dot.dataset.id = c.id;
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
  svg?.querySelectorAll<SVGElement>('[data-id]').forEach((el) => el.classList.toggle('is-current', el.dataset.id === id));
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
