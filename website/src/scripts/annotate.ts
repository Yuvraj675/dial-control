// The Settings window as a 3D object with 2D labels: as it scrolls into view the window tilts back up to flat,
// then a leader line and a label draw out of each area. Scroll-synced anime.js timeline; with no script, on a
// phone or with reduced motion it is a flat screenshot with a list underneath.
import { createTimeline } from 'animejs/timeline';
import { onScroll } from 'animejs/events';
import { createDrawable } from 'animejs/svg';

const fig = document.querySelector<HTMLElement>('[data-annotate]');
const stage = fig?.querySelector<HTMLElement>('.shot__stage') ?? null;
const frame = fig?.querySelector<HTMLElement>('.shot__frame') ?? null;
const svg = fig?.querySelector<SVGSVGElement>('.shot__lines') ?? null;
const notes = Array.from(fig?.querySelectorAll<HTMLElement>('.note') ?? []);
const SVG_NS = 'http://www.w3.org/2000/svg';
const wide = window.matchMedia('(min-width: 72rem) and (prefers-reduced-motion: no-preference)');
let timeline: ReturnType<typeof createTimeline> | null = null;

function teardown() {
  timeline?.revert();
  timeline = null;
  svg?.replaceChildren();
  frame?.removeAttribute('style');
  notes.forEach((n) => n.removeAttribute('style'));
}

function build() {
  if (!fig || !stage || !frame || !svg || !notes.length) return;
  teardown();
  if (!wide.matches) return;

  const f = fig.getBoundingClientRect();
  const s = stage.getBoundingClientRect();
  svg.setAttribute('viewBox', `0 0 ${f.width} ${f.height}`);
  // The frame is tilted while we measure, so use its layout box, not its painted one.
  const originX = s.left - f.left + frame.offsetLeft;
  const originY = s.top - f.top + frame.offsetTop;

  const parts = notes.map((note) => {
    const ax = originX + (Number(note.dataset.x) / 100) * frame.offsetWidth;
    const ay = originY + (Number(note.dataset.y) / 100) * frame.offsetHeight;
    const n = note.getBoundingClientRect();
    const line = document.createElementNS(SVG_NS, 'line');
    line.setAttribute('x1', String(ax)); line.setAttribute('y1', String(ay));
    line.setAttribute('x2', String(n.left - f.left + n.width * 0.18)); line.setAttribute('y2', String(n.top - f.top - 6));
    const dot = document.createElementNS(SVG_NS, 'circle');
    dot.setAttribute('cx', String(ax)); dot.setAttribute('cy', String(ay)); dot.setAttribute('r', '0');
    svg.append(line, dot);
    return { note, line, dot };
  });

  // From the figure's top entering the bottom of the screen until its middle reaches the middle.
  const tl = createTimeline({
    defaults: { ease: 'linear' },
    autoplay: onScroll({ target: fig, enter: { target: 'top', container: 'bottom' }, leave: { target: 'center', container: 'center' }, sync: true }),
  });
  tl.add(frame, { rotateX: [24, 0], rotateY: [-20, 0], translateZ: [-170, 0], duration: 600 }, 0);
  parts.forEach(({ note, line, dot }, i) => {
    const t = 560 + i * 140;
    const [drawable] = createDrawable(line);
    tl.add(dot, { r: [0, 4.5], duration: 40 }, t)
      .add(drawable, { draw: ['0 0', '0 1'], duration: 110 }, t + 10)
      .add(note, { opacity: [0, 1], duration: 90 }, t + 80);
  });
  tl.add({ x: 0 }, { x: 1, duration: 1 }, 1000);
  timeline = tl;
}

let pending = 0;
const rebuild = () => { window.clearTimeout(pending); pending = window.setTimeout(build, 120); };
if (fig && svg) {
  try {
    void document.fonts?.ready.then(build);
    window.addEventListener('resize', rebuild, { passive: true });
    wide.addEventListener('change', rebuild);
    frame?.querySelector('img')?.addEventListener('load', rebuild);
    build();
  } catch {
    teardown();
  }
}
