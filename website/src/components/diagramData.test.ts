import { test } from 'node:test';
import assert from 'node:assert/strict';
import { CALLOUTS, DIAL, anchorPoint, currentCallout, dialAt } from './diagramData.ts';

test('steps come in increasing order with room to draw before the next one', () => {
  for (let i = 1; i < CALLOUTS.length; i++) assert.ok(CALLOUTS[i].at - CALLOUTS[i - 1].at >= 0.09, CALLOUTS[i].id);
  assert.ok(CALLOUTS.at(-1)!.at + 0.14 <= 1, 'the last step finishes before the dial unpins');
  assert.equal(new Set(CALLOUTS.map((c) => c.id)).size, CALLOUTS.length);
});

test('slice anchors sit on the slice ring, and slice 0 is straight up', () => {
  const top = anchorPoint({ kind: 'slice', i: 0 }, 300, 300, DIAL.natural);
  assert.ok(Math.abs(top.x - 300) < 1e-9);
  assert.ok(Math.abs(top.y - (300 - (DIAL.rInner + DIAL.rOuter) / 2)) < 1e-9);
  for (let i = 0; i < DIAL.slices; i++) {
    const p = anchorPoint({ kind: 'slice', i }, 0, 0, DIAL.natural);
    assert.ok(Math.abs(Math.hypot(p.x, p.y) - (DIAL.rInner + DIAL.rOuter) / 2) < 1e-9);
  }
});

test('anchors scale with the dial and the centre anchor is the centre', () => {
  const a = anchorPoint({ kind: 'slice', i: 2 }, 0, 0, DIAL.natural);
  const b = anchorPoint({ kind: 'slice', i: 2 }, 0, 0, DIAL.natural * 2);
  assert.ok(Math.abs(b.x - a.x * 2) < 1e-9 && Math.abs(b.y - a.y * 2) < 1e-9);
  assert.deepEqual(anchorPoint({ kind: 'center' }, 12, 34, 999), { x: 12, y: 34 });
});

test('a label sits on the same side as the thing it points at', () => {
  for (const c of CALLOUTS) {
    if (c.anchor.kind === 'center') continue;
    const p = anchorPoint(c.anchor, 0, 0, DIAL.natural);
    assert.equal(p.x < 0 ? 'l' : 'r', c.side, c.id);
  }
});

test('the current step follows progress', () => {
  assert.equal(currentCallout(0), -1);
  assert.equal(currentCallout(CALLOUTS[0].at), 0);
  assert.equal(currentCallout(1), CALLOUTS.length - 1);
});

test('the dial does what each label says, in order', () => {
  assert.equal(dialAt(0).index, 0);
  // Turn: the highlight reaches volume by the end of the step, passing brightness.
  const turned = Array.from({ length: 50 }, (_, i) => dialAt(0.04 + (0.13 * i) / 49).index);
  assert.deepEqual(turned, [...turned].sort((a, b) => a - b));
  assert.equal(turned.at(-1), 2);
  assert.ok(turned.includes(1));
  // Click: the volume gauge opens.
  assert.equal(dialAt(0.2).mode, 'menu');
  assert.equal(dialAt(0.25).mode, 'volume');
  // Turn again: the level rises and then falls.
  const levels = Array.from({ length: 60 }, (_, i) => dialAt(0.26 + (0.1699 * i) / 59).gauge);
  assert.ok(Math.max(...levels) >= 80 && levels[0] <= 55 && levels.at(-1)! <= 35);
  assert.ok(levels.indexOf(Math.max(...levels)) > 5 && levels.indexOf(Math.max(...levels)) < 55);
  // Brightness, scroll, home, double press.
  assert.equal(dialAt(0.54).mode, 'brightness');
  assert.equal(dialAt(0.65).mode, 'scroll');
  assert.equal(dialAt(0.70).index, 7);
  assert.equal(dialAt(0.76).profileId, 'home');
  assert.equal(dialAt(0.795).profileId, 'code_editor');
  assert.equal(dialAt(0.85).open, false);
  assert.equal(dialAt(0.95).open, true);
  assert.equal(dialAt(0.95).profileId, 'default');
});

test('every state stays in range for its dial at any progress', () => {
  const size: Record<string, number> = { default: 9, code_editor: 9, home: 7 };
  for (let i = -2; i <= 102; i++) {
    const d = dialAt(i / 100);
    assert.ok(d.index >= 0 && d.index < size[d.profileId], `q=${i / 100}`);
    assert.ok(d.gauge >= 0 && d.gauge <= 100);
  }
});
