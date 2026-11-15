// Small interactions that answer the pointer and the keyboard. All of them are off on touch screens and
// under reduced motion, and none of them changes layout.
//   magnetic buttons  the Download buttons lean toward the pointer when it is near (a pull of about 0.3)
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
  // Magnetic buttons: within reach, shift toward the pointer by 30% of the offset (never more than 8px).
  const magnets = Array.from(document.querySelectorAll<HTMLElement>('[data-magnet]'));
  let px = -9999;
  let py = -9999;
  let frame = 0;
  const clamp = (v: number, m: number) => Math.max(-m, Math.min(m, v));
  const pull = () => {
    frame = 0;
    for (const el of magnets) {
      const r = el.getBoundingClientRect();
      const dx = px - (r.left + r.width / 2);
      const dy = py - (r.top + r.height / 2);
      const reach = Math.max(r.width, r.height) * 0.9;
      el.style.translate = Math.hypot(dx, dy) < reach ? `${clamp(dx * 0.3, 8)}px ${clamp(dy * 0.3, 8)}px` : '';
    }
  };
  if (magnets.length) {
    window.addEventListener('pointermove', (e) => { px = e.clientX; py = e.clientY; if (!frame) frame = requestAnimationFrame(pull); }, { passive: true });
    document.addEventListener('pointerleave', () => { px = py = -9999; pull(); });
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
