/** The anatomy diagram: one callout per part of the dial, shared by the page markup, the flying dial
 *  (which lights the part being explained) and the anime.js script (which draws the leader lines). */

/** Dimensions of the app's wheel, in its own 450px units (DonutRadialHUD defaults). */
export const DIAL = { natural: 450, rOuter: 188, rInner: 120, rBezel: 204, slices: 9 } as const;

export type Anchor = { kind: 'edge'; i: number } | { kind: 'ring'; deg: number } | { kind: 'center' } | { kind: 'point'; dx: number; dy: number };

export interface Callout {
  id: string;
  title: string;
  text: string;
  /** Which side of the dial the label sits on (the side the thing it points at is on). */
  side: 'l' | 'r';
  /** Label height relative to the dial's centre, as a fraction of the dial's size. */
  y: number;
  anchor: Anchor;
  /** Progress (0 to 1) through the pinned walkthrough at which this step starts and its line draws. */
  at: number;
}

/** The walkthrough, in the order a person would learn the dial. The dial performs each step as you scroll
 *  (see `dialAt`); the label and its leader line say what it is doing. */
export const CALLOUTS: Callout[] = [
  { id: 'turn', title: 'Turn', text: 'Each notch moves the highlight.', side: 'l', y: -0.34, anchor: { kind: 'ring', deg: -150 }, at: 0.03 },
  { id: 'click', title: 'Click', text: 'Runs the highlighted option. Volume opens a gauge.', side: 'l', y: 0.3, anchor: { kind: 'point', dx: -74, dy: 56 }, at: 0.158 },
  { id: 'volume', title: 'Set the volume', text: 'Turn again to move the level. Yours returns after.', side: 'r', y: -0.06, anchor: { kind: 'edge', i: 2 }, at: 0.286 },
  { id: 'brightness', title: 'Brightness', text: 'The same two moves, for your screen.', side: 'r', y: -0.34, anchor: { kind: 'edge', i: 1 }, at: 0.414 },
  { id: 'scroll', title: 'Scroll', text: 'The dial becomes a mouse wheel.', side: 'r', y: 0.19, anchor: { kind: 'edge', i: 3 }, at: 0.542 },
  { id: 'home', title: 'Home', text: 'Pick another dial, like Code.', side: 'l', y: -0.06, anchor: { kind: 'edge', i: 7 }, at: 0.67 },
  { id: 'double', title: 'Double press', text: 'Closes the wheel, and opens it again.', side: 'r', y: 0.4, anchor: { kind: 'point', dx: 74, dy: 56 }, at: 0.798 },
];

export type DialMode = 'menu' | 'volume' | 'brightness' | 'scroll';
export interface DialState { open: boolean; profileId: string; index: number; mode: DialMode; gauge: number }

const clamp = (x: number, lo = 0, hi = 1) => Math.min(hi, Math.max(lo, x));
const lerp = (a: number, b: number, t: number) => a + (b - a) * clamp(t);

/** Each step is `STEP_LENGTH` of the walkthrough long. Its first half draws the label and its line; the dial only
 *  performs the action in the second half, so the words are fully there before anything happens. */
export const STEP_LENGTH = 0.128;

/** What the dial shows at progress `q` through the walkthrough: it actually does the things the labels say.
 *  Slice order in the System dial: 0 play/pause, 1 brightness, 2 volume, 3 scroll, 7 home. */
export function dialAt(progress: number): DialState {
  const q = clamp(progress);
  const base: DialState = { open: true, profileId: 'default', index: 0, mode: 'menu', gauge: 55 };
  const starts = CALLOUTS.map((c) => c.at);
  if (q < starts[0] || q >= starts[starts.length - 1] + STEP_LENGTH) return base;

  let i = starts.findIndex((st) => q < st + STEP_LENGTH);
  const p = clamp((q - starts[i]) / STEP_LENGTH); // 0..1 through the step
  const a = clamp((p - 0.5) / 0.5); // 0..1 through the action, which only starts once the label is fully drawn
  const waiting = p < 0.5;

  switch (CALLOUTS[i].id) {
    // Turn: the highlight steps from play/pause to volume.
    case 'turn':
      return { ...base, index: Math.round(lerp(0, 2, a)) };
    // Click on volume: its gauge opens.
    case 'click':
      return a < 0.5 ? { ...base, index: 2 } : { ...base, index: 2, mode: 'volume', gauge: 50 };
    // Turn again: the level rises, then falls.
    case 'volume':
      return { ...base, index: 2, mode: 'volume', gauge: waiting ? 50 : Math.round(a < 0.5 ? lerp(50, 85, a * 2) : lerp(85, 30, (a - 0.5) * 2)) };
    // Brightness: back to the wheel, turn to it, click, and the second gauge falls.
    case 'brightness':
      if (waiting) return { ...base, index: 2, mode: 'volume', gauge: 30 };
      if (a < 0.2) return { ...base, index: 2 };
      if (a < 0.4) return { ...base, index: 1 };
      return { ...base, index: 1, mode: 'brightness', gauge: Math.round(lerp(70, 35, (a - 0.4) / 0.6)) };
    // Scroll: back to the wheel, turn to scroll, click.
    case 'scroll':
      if (waiting) return { ...base, index: 1, mode: 'brightness', gauge: 35 };
      if (a < 0.25) return { ...base, index: 1 };
      if (a < 0.6) return { ...base, index: 3 };
      return { ...base, index: 3, mode: 'scroll' };
    // Home: turn to home, click for the list of dials, pick Code.
    case 'home':
      if (waiting) return { ...base, index: 3, mode: 'scroll' };
      if (a < 0.15) return { ...base, index: 3 };
      if (a < 0.35) return { ...base, index: 7 };
      if (a < 0.8) return { ...base, profileId: 'home', index: Math.round(lerp(0, 2, (a - 0.35) / 0.45)) };
      return { ...base, profileId: 'code_editor' };
    // Double press: the wheel closes, then opens again.
    default:
      if (waiting || a < 0.15) return { ...base, profileId: 'code_editor' };
      if (a < 0.6) return { ...base, open: false };
      return base;
  }
}

/** Where a callout's line attaches, for a dial of `size` px centred at (cx, cy). An edge anchor sits just inside the
 *  outer rim of its slice, so a line to it never has to cross the other slices. */
export function anchorPoint(anchor: Anchor, cx: number, cy: number, size: number): { x: number; y: number } {
  const k = size / DIAL.natural;
  if (anchor.kind === 'center') return { x: cx, y: cy };
  // A point is given in the wheel's own 450px units from its centre: the click lines start at either end of the
  // label under the centre button, so they leave clear of it.
  if (anchor.kind === 'point') return { x: cx + anchor.dx * k, y: cy + anchor.dy * k };
  const theta = anchor.kind === 'edge' ? -Math.PI / 2 + (anchor.i * 2 * Math.PI) / DIAL.slices : (anchor.deg * Math.PI) / 180;
  const r = anchor.kind === 'edge' ? DIAL.rOuter - 16 : DIAL.rBezel;
  return { x: cx + r * k * Math.cos(theta), y: cy + r * k * Math.sin(theta) };
}

export interface Leader {
  id: string;
  /** The line: the anchor on the dial, an optional 45 degree bend, then a horizontal run to the label. */
  points: { x: number; y: number }[];
  /** Where the label's near edge sits and the height of its centre. */
  labelEdgeX: number;
  labelY: number;
}

/** Lay out every leader line for a dial of `size` px at (cx, cy). Each line leaves its anchor at 45 degrees away
 *  from the dial, then runs horizontally into its label; labels in a column are ordered like their anchors, so the
 *  lines are parallel diagonals and horizontals that never cross (tested). */
export function layoutLeaders(cx: number, cy: number, size: number): Leader[] {
  const gap = size * 0.18;
  const shoulder = 10;
  return CALLOUTS.map((c) => {
    const a = anchorPoint(c.anchor, cx, cy, size);
    const dir = c.side === 'l' ? -1 : 1;
    const labelEdgeX = cx + dir * (size / 2 + gap);
    // A label that is almost level with its anchor is snapped level, so the line is a clean horizontal.
    const wanted = cy + c.y * size;
    const labelY = Math.abs(wanted - a.y) < size * 0.03 ? a.y : wanted;
    const end = { x: labelEdgeX - dir * shoulder, y: labelY };
    const dy = Math.abs(labelY - a.y);
    const knee = { x: a.x + dir * dy, y: labelY };
    const bends = dy > 1.5 && (dir === 1 ? knee.x < end.x : knee.x > end.x);
    return { id: c.id, points: bends ? [a, knee, end] : [a, end], labelEdgeX, labelY };
  });
}

/** Index of the latest callout that has started, or -1 before the first. */
export function currentCallout(q: number): number {
  let cur = -1;
  CALLOUTS.forEach((c, i) => { if (q >= c.at) cur = i; });
  return cur;
}
