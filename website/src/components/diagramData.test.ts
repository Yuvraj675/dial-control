import { test } from 'node:test';
import assert from 'node:assert/strict';
import { CALLOUTS, DIAL, STEP_LENGTH, anchorPoint, currentCallout, dialAt, layoutLeaders } from './diagramData.ts';

test('steps come in increasing order, one step apart', () => {
  for (let i = 1; i < CALLOUTS.length; i++) assert.ok(CALLOUTS[i].at - CALLOUTS[i - 1].at >= STEP_LENGTH - 1e-9, CALLOUTS[i].id);
  assert.ok(CALLOUTS.at(-1)!.at + STEP_LENGTH <= 1, 'the last step finishes before the dial unpins');
  assert.equal(new Set(CALLOUTS.map((c) => c.id)).size, CALLOUTS.length);
});

test('edge anchors sit inside the outer rim of their slice, and slice 0 is straight up', () => {
  const top = anchorPoint({ kind: 'edge', i: 0 }, 300, 300, DIAL.natural);
  assert.ok(Math.abs(top.x - 300) < 1e-9);
  assert.ok(Math.abs(top.y - (300 - (DIAL.rOuter - 16))) < 1e-9);
  for (let i = 0; i < DIAL.slices; i++) {
    const p = anchorPoint({ kind: 'edge', i }, 0, 0, DIAL.natural);
    assert.ok(Math.abs(Math.hypot(p.x, p.y) - (DIAL.rOuter - 16)) < 1e-9);
  }
});

test('anchors scale with the dial and the centre anchor is the centre', () => {
  const a = anchorPoint({ kind: 'edge', i: 2 }, 0, 0, DIAL.natural);
  const b = anchorPoint({ kind: 'edge', i: 2 }, 0, 0, DIAL.natural * 2);
  assert.ok(Math.abs(b.x - a.x * 2) < 1e-9 && Math.abs(b.y - a.y * 2) < 1e-9);
  assert.deepEqual(anchorPoint({ kind: 'center' }, 12, 34, 999), { x: 12, y: 34 });
});

test('a label sits on the same side as the thing it points at', () => {
  for (const c of CALLOUTS) {
    if (c.anchor.kind !== 'edge' && c.anchor.kind !== 'ring') continue;
    const p = anchorPoint(c.anchor, 0, 0, DIAL.natural);
    assert.equal(p.x < 0 ? 'l' : 'r', c.side, c.id);
  }
});

test('the current step follows progress', () => {
  assert.equal(currentCallout(0), -1);
  assert.equal(currentCallout(CALLOUTS[0].at), 0);
  assert.equal(currentCallout(1), CALLOUTS.length - 1);
});

const at = (id: string) => CALLOUTS.find((c) => c.id === id)!.at;
const mid = (id: string, p: number) => at(id) + STEP_LENGTH * p; // a moment p of the way through a step

test('the dial does what each label says, in order', () => {
  assert.equal(dialAt(0).index, 0);
  // Turn: the highlight steps on to volume, passing brightness, only in the second half of the step.
  const turned = Array.from({ length: 50 }, (_, i) => dialAt(mid('turn', 0.5 + (0.499 * i) / 49)).index);
  assert.deepEqual(turned, [...turned].sort((a, b) => a - b));
  assert.equal(turned.at(-1), 2);
  assert.ok(turned.includes(1));
  // Click: the volume gauge opens.
  assert.equal(dialAt(mid('click', 0.6)).mode, 'menu');
  assert.equal(dialAt(mid('click', 0.95)).mode, 'volume');
  // Turn again: the level rises and then falls.
  const levels = Array.from({ length: 60 }, (_, i) => dialAt(mid('volume', 0.5 + (0.499 * i) / 59)).gauge);
  assert.ok(Math.max(...levels) >= 80 && levels[0] <= 55 && levels.at(-1)! <= 35);
  assert.ok(levels.indexOf(Math.max(...levels)) > 5 && levels.indexOf(Math.max(...levels)) < 55);
  // Brightness, scroll, home, double press.
  assert.equal(dialAt(mid('brightness', 0.9)).mode, 'brightness');
  assert.equal(dialAt(mid('scroll', 0.95)).mode, 'scroll');
  assert.equal(dialAt(mid('home', 0.66)).index, 7);
  assert.equal(dialAt(mid('home', 0.8)).profileId, 'home');
  assert.equal(dialAt(mid('home', 0.99)).profileId, 'code_editor');
  assert.equal(dialAt(mid('double', 0.7)).open, false);
  assert.equal(dialAt(mid('double', 0.99)).open, true);
  assert.equal(dialAt(mid('double', 0.99)).profileId, 'default');
});

test('the dial waits until the words are fully there: nothing changes in the first half of any step', () => {
  for (const c of CALLOUTS) {
    const start = dialAt(c.at + 0.0005);
    for (const p of [0.1, 0.25, 0.4, 0.49]) assert.deepEqual(dialAt(mid(c.id, p)), start, `${c.id} moved at ${p} of the way through`);
  }
});

test('the label is fully drawn before the dial acts: step timing leaves a clear half step', () => {
  for (let i = 1; i < CALLOUTS.length; i++) assert.ok(Math.abs(CALLOUTS[i].at - CALLOUTS[i - 1].at - STEP_LENGTH) < 1e-9, `${CALLOUTS[i].id} starts one step after the last`);
  assert.ok(CALLOUTS.at(-1)!.at + STEP_LENGTH <= 0.94, 'a hold at the end');
});

test('one step hands over to the next without a jump', () => {
  for (let i = 1; i < CALLOUTS.length; i++) {
    const before = dialAt(CALLOUTS[i].at - 0.0005);
    const after = dialAt(CALLOUTS[i].at + 0.0005);
    assert.ok(Math.abs(after.gauge - before.gauge) <= 1, `${CALLOUTS[i].id}: the gauge jumps`);
    assert.deepEqual({ ...after, gauge: 0 }, { ...before, gauge: 0 }, `${CALLOUTS[i - 1].id} to ${CALLOUTS[i].id}`);
  }
});

test('every state stays in range for its dial at any progress', () => {
  const size: Record<string, number> = { default: 9, code_editor: 9, home: 7 };
  for (let i = -2; i <= 102; i++) {
    const d = dialAt(i / 100);
    assert.ok(d.index >= 0 && d.index < size[d.profileId], `q=${i / 100}`);
    assert.ok(d.gauge >= 0 && d.gauge <= 100);
  }
});

// ── the leader lines must be clean: never crossing, only horizontal or 45 degrees, labels not overlapping ──
type Pt = { x: number; y: number };
const cross = (a: Pt, b: Pt, c: Pt, d: Pt) => {
  const o = (p: Pt, q: Pt, r: Pt) => Math.sign((q.x - p.x) * (r.y - p.y) - (q.y - p.y) * (r.x - p.x));
  return o(a, b, c) !== o(a, b, d) && o(c, d, a) !== o(c, d, b);
};
const distToSegment = (p: Pt, a: Pt, b: Pt) => {
  const l2 = (b.x - a.x) ** 2 + (b.y - a.y) ** 2;
  const t = l2 === 0 ? 0 : Math.max(0, Math.min(1, ((p.x - a.x) * (b.x - a.x) + (p.y - a.y) * (b.y - a.y)) / l2));
  return Math.hypot(p.x - (a.x + t * (b.x - a.x)), p.y - (a.y + t * (b.y - a.y)));
};

for (const size of [300, 360, 420, 468, 500, 560]) {
  test(`no two leader lines cross, at a dial of ${size}px`, () => {
    const leaders = layoutLeaders(700, 450, size);
    for (let i = 0; i < leaders.length; i++) for (let j = i + 1; j < leaders.length; j++) {
      const A = leaders[i].points, B = leaders[j].points;
      for (let a = 0; a < A.length - 1; a++) for (let b = 0; b < B.length - 1; b++) {
        assert.equal(cross(A[a], A[a + 1], B[b], B[b + 1]), false, `${leaders[i].id} crosses ${leaders[j].id}`);
      }
    }
  });

  test(`every line is horizontal or at 45 degrees, and keeps clear of other anchors, at ${size}px`, () => {
    const leaders = layoutLeaders(700, 450, size);
    for (const l of leaders) {
      for (let k = 0; k < l.points.length - 1; k++) {
        const dx = Math.abs(l.points[k + 1].x - l.points[k].x), dy = Math.abs(l.points[k + 1].y - l.points[k].y);
        assert.ok(dy < 0.5 || Math.abs(dx - dy) < 0.5, `${l.id} has an odd angle`);
      }
      for (const other of leaders) {
        if (other === l) continue;
        for (let k = 0; k < l.points.length - 1; k++) {
          assert.ok(distToSegment(other.points[0], l.points[k], l.points[k + 1]) > size * 0.03, `${l.id} runs through the anchor of ${other.id}`);
        }
      }
    }
  });

  test(`labels in a column are in the same order as their anchors and do not overlap, at ${size}px`, () => {
    const leaders = layoutLeaders(700, 450, size);
    for (const side of ['l', 'r'] as const) {
      const col = CALLOUTS.map((c, i) => ({ c, l: leaders[i] })).filter((x) => x.c.side === side).sort((a, b) => a.l.labelY - b.l.labelY);
      for (let i = 1; i < col.length; i++) {
        assert.ok(col[i].l.labelY - col[i - 1].l.labelY >= size * 0.17, `${col[i - 1].c.id} and ${col[i].c.id} are too close`);
        const ay = (x: (typeof col)[number]) => x.l.points[0].y;
        const spatial = (x: (typeof col)[number]) => x.c.anchor.kind === 'edge' || x.c.anchor.kind === 'ring';
        if (spatial(col[i]) && spatial(col[i - 1])) assert.ok(ay(col[i]) >= ay(col[i - 1]), `${col[i].c.id} is out of order`);
      }
    }
  });
}
