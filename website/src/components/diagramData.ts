/** The anatomy diagram: one callout per part of the dial, shared by the page markup, the flying dial
 *  (which lights the part being explained) and the anime.js script (which draws the leader lines). */

/** Dimensions of the app's wheel, in its own 450px units (DonutRadialHUD defaults). */
export const DIAL = { natural: 450, rOuter: 188, rInner: 120, rBezel: 204, slices: 9 } as const;

export type Anchor = { kind: 'slice'; i: number } | { kind: 'ring'; deg: number } | { kind: 'center' };

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
  { id: 'turn', title: 'Turn', text: 'Each notch moves the highlight.', side: 'l', y: -0.36, anchor: { kind: 'ring', deg: -150 }, at: 0.04 },
  { id: 'click', title: 'Click', text: 'Runs the highlighted option. Volume opens a gauge.', side: 'r', y: -0.26, anchor: { kind: 'center' }, at: 0.17 },
  { id: 'volume', title: 'Set the volume', text: 'Turn again to move the level. Yours returns after.', side: 'l', y: -0.12, anchor: { kind: 'ring', deg: 180 }, at: 0.26 },
  { id: 'brightness', title: 'Brightness', text: 'The same two moves, for your screen.', side: 'r', y: 0, anchor: { kind: 'slice', i: 1 }, at: 0.43 },
  { id: 'scroll', title: 'Scroll', text: 'The dial becomes a mouse wheel.', side: 'r', y: 0.26, anchor: { kind: 'slice', i: 3 }, at: 0.56 },
  { id: 'home', title: 'Home', text: 'Pick another dial, like Code.', side: 'l', y: 0.12, anchor: { kind: 'slice', i: 7 }, at: 0.67 },
  { id: 'double', title: 'Double press', text: 'Closes the wheel, and opens it again.', side: 'l', y: 0.36, anchor: { kind: 'center' }, at: 0.8 },
];

export type DialMode = 'menu' | 'volume' | 'brightness' | 'scroll';
export interface DialState { open: boolean; profileId: string; index: number; mode: DialMode; gauge: number }

const clamp = (x: number, lo = 0, hi = 1) => Math.min(hi, Math.max(lo, x));
const lerp = (a: number, b: number, t: number) => a + (b - a) * clamp(t);

/** What the dial shows at progress `q` through the walkthrough: it actually does the things the labels say.
 *  Slice order in the System dial: 0 play/pause, 1 brightness, 2 volume, 3 scroll, 7 home. */
export function dialAt(progress: number): DialState {
  const q = clamp(progress);
  const seg = (a: number, b: number) => clamp((q - a) / (b - a));
  const base: DialState = { open: true, profileId: 'default', index: 0, mode: 'menu', gauge: 55 };

  if (q < 0.04) return base;
  // Turn: the highlight steps from play/pause to volume.
  if (q < 0.17) return { ...base, index: Math.round(lerp(0, 2, seg(0.04, 0.17))) };
  // Click on volume: its gauge opens.
  if (q < 0.26) return seg(0.17, 0.26) < 0.4 ? { ...base, index: 2 } : { ...base, index: 2, mode: 'volume', gauge: 50 };
  // Turn again: the level rises, then falls.
  if (q < 0.43) {
    const p = seg(0.26, 0.43);
    return { ...base, index: 2, mode: 'volume', gauge: Math.round(p < 0.5 ? lerp(50, 85, p * 2) : lerp(85, 30, (p - 0.5) * 2)) };
  }
  // Brightness: back to the wheel, turn to it, click, and the second gauge falls.
  if (q < 0.56) {
    const p = seg(0.43, 0.56);
    if (p < 0.2) return { ...base, index: 2 };
    if (p < 0.4) return { ...base, index: 1 };
    return { ...base, index: 1, mode: 'brightness', gauge: Math.round(lerp(70, 35, (p - 0.4) / 0.6)) };
  }
  // Scroll: back to the wheel, turn to scroll, click.
  if (q < 0.67) {
    const p = seg(0.56, 0.67);
    if (p < 0.2) return { ...base, index: 1 };
    if (p < 0.5) return { ...base, index: 3 };
    return { ...base, index: 3, mode: 'scroll' };
  }
  // Home: turn to home, click for the list of dials, pick Code.
  if (q < 0.8) {
    const p = seg(0.67, 0.8);
    if (p < 0.2) return { ...base, index: 3 };
    if (p < 0.38) return { ...base, index: 7 };
    if (p < 0.75) return { ...base, profileId: 'home', index: Math.round(lerp(0, 2, (p - 0.38) / 0.37)) };
    return { ...base, profileId: 'code_editor', index: 0 };
  }
  // Double press: the wheel closes, then opens again.
  if (q < 0.92) {
    const p = seg(0.8, 0.92);
    if (p < 0.25) return { ...base, profileId: 'code_editor' };
    if (p < 0.65) return { ...base, open: false };
    return base;
  }
  return base;
}

/** Where a callout's line attaches, for a dial of `size` px centred at (cx, cy). */
export function anchorPoint(anchor: Anchor, cx: number, cy: number, size: number): { x: number; y: number } {
  const k = size / DIAL.natural;
  if (anchor.kind === 'center') return { x: cx, y: cy };
  const theta =
    anchor.kind === 'slice'
      ? -Math.PI / 2 + (anchor.i * 2 * Math.PI) / DIAL.slices
      : (anchor.deg * Math.PI) / 180;
  const r = anchor.kind === 'slice' ? (DIAL.rInner + DIAL.rOuter) / 2 : DIAL.rBezel;
  return { x: cx + r * k * Math.cos(theta), y: cy + r * k * Math.sin(theta) };
}

/** Index of the latest callout that has started, or -1 before the first. */
export function currentCallout(q: number): number {
  let cur = -1;
  CALLOUTS.forEach((c, i) => { if (q >= c.at) cur = i; });
  return cur;
}
