// Small interactions that answer the pointer and the keyboard. All of them are off on touch screens and
// under reduced motion, and none of them changes layout.
//   stretchy buttons  the Download buttons stretch toward the pointer, but only when it is very close
//   tilt              the Settings window leans toward the pointer, like the app switcher's dial
//   live keys         the keys in the keyboard section light up as you press the real ones
const calm = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
const fine = window.matchMedia('(hover: hover) and (pointer: fine)').matches;

// Live keys work on any keyboard, with or without a mouse.
const keyEls = Array.from(document.querySelectorAll<HTMLElement>('.key[data-key]'));
if (keyEls.length) {
  const norm = (k: string) => (k === 'OS' ? 'Meta' : k.length === 1 ? k.toLowerCase() : k);
  const set = (k: string, down: boolean) => keyEls.forEach((el) => { if (el.dataset.key === norm(k)) el.classList.toggle('is-down', down); });
  window.addEventListener('keydown', (e) => set(e.key, true));
  window.addEventListener('keyup', (e) => set(e.key, false));
  window.addEventListener('blur', () => keyEls.forEach((el) => el.classList.remove('is-down')));
}

if (fine && !calm) {
  // Stretchy buttons: when the pointer is very close (within 22px of the button, or over it), the button stretches
  // toward it, up to 9% along the width and 16% along the height. Anchored on the far side, so the near edge reaches out.
  const stretchy = Array.from(document.querySelectorAll<HTMLElement>('[data-stretch]'));
  const NEAR = 22;
  let px = -9999;
  let py = -9999;
  let frame = 0;
  const clamp = (v: number, lo: number, hi: number) => Math.max(lo, Math.min(hi, v));
  const lean = () => {
    frame = 0;
    for (const el of stretchy) {
      const r = el.getBoundingClientRect();
      const dx = px - (r.left + r.width / 2);
      const dy = py - (r.top + r.height / 2);
      const out = Math.hypot(Math.max(Math.abs(dx) - r.width / 2, 0), Math.max(Math.abs(dy) - r.height / 2, 0));
      if (out >= NEAR) { el.style.scale = ''; el.style.transformOrigin = ''; continue; }
      const near = 1 - out / NEAR; // 0 at the edge of the zone, 1 over the button
      const nx = clamp(dx / (r.width / 2), -1, 1);
      const ny = clamp(dy / (r.height / 2), -1, 1);
      el.style.transformOrigin = `${50 - nx * 50}% ${50 - ny * 50}%`;
      el.style.scale = `${1 + 0.09 * near * Math.abs(nx)} ${1 + 0.16 * near * Math.abs(ny)}`;
    }
  };
  if (stretchy.length) {
    window.addEventListener('pointermove', (e) => { px = e.clientX; py = e.clientY; if (!frame) frame = requestAnimationFrame(lean); }, { passive: true });
    document.addEventListener('pointerleave', () => { px = py = -9999; lean(); });
  }

  // Tilt: lean toward the pointer while it is over the element, settle back when it leaves.
  document.querySelectorAll<HTMLElement>('[data-tilt]').forEach((el) => {
    const max = Number(el.dataset.tilt) || 4;
    el.addEventListener('pointermove', (e) => {
      const r = el.getBoundingClientRect();
      const x = (e.clientX - r.left) / r.width - 0.5;
      const y = (e.clientY - r.top) / r.height - 0.5;
      el.style.transition = 'transform 120ms linear';
      el.style.transform = `perspective(1400px) rotateX(${(-y * 2 * max * 0.7).toFixed(2)}deg) rotateY(${(x * 2 * max).toFixed(2)}deg)`;
    });
    el.addEventListener('pointerleave', () => {
      el.style.transition = 'transform 520ms cubic-bezier(0.16, 1, 0.3, 1)';
      el.style.transform = '';
    });
  });
}
