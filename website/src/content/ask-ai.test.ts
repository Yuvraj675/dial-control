import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const raw = readFileSync(new URL('./ask-ai.md', import.meta.url), 'utf8');
const filled = raw.replaceAll('{{SITE}}', 'https://example.test/dial-control/').replaceAll('{{REPO}}', 'https://github.com/Yuvraj675/dial-control');

test('the prompt carries the GitHub link, the website and the download links', () => {
  assert.ok(raw.includes('{{REPO}}') && raw.includes('{{SITE}}'));
  assert.ok(filled.includes('https://github.com/Yuvraj675/dial-control/releases/latest'));
  assert.ok(filled.includes('https://example.test/dial-control/'));
  assert.equal(filled.includes('{{'), false, 'every placeholder is filled');
});

test('the prompt tells the assistant what to do and to stay within the notes', () => {
  assert.match(raw, /Read the notes below/);
  assert.match(raw, /Use only these notes/);
  assert.match(raw, /say you do not know/);
});

test('the prompt covers what a newcomer needs: what it is, gestures, keyboard use, install, limits, affiliation', () => {
  for (const needle of ['CMF Buds Pro 2', 'double press', 'triple press', 'Super+Shift+D', 'Win+Shift+D', 'Cmd+Shift+D', 'macro', 'AVRCP', 'install.sh', 'install.ps1', 'glibc 2.39', 'Apple Silicon', 'not code-signed', 'Android', 'MIT', 'not affiliated']) {
    assert.ok(raw.includes(needle), `missing: ${needle}`);
  }
});

test('it is a sensible length and has no em or en dashes', () => {
  const words = raw.split(/\s+/).length;
  assert.ok(words > 250 && words < 700, `${words} words`);
  assert.equal(/[–—]/.test(raw), false);
});

test('claims stay true to the README: no invented numbers or ratings', () => {
  assert.equal(/\b\d+\s?(users|downloads|stars|reviews)\b/i.test(raw), false);
  assert.equal(/\b(best|fastest|#1|award)\b/i.test(raw), false);
});
