// Small interactions that answer the pointer and the keyboard. All of them are off on touch screens and
// under reduced motion, and none of them changes layout.
//   headings          section headings rise in word by word the first time they scroll into view
//   scramble          the wordmark and platform names re-draw their letters once when hovered or focused
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

// Headings: each section heading rises in word by word the first time it scrolls into view. The words start
// where they belong (only a few pixels low and a touch transparent) and the class is added by this script, so
// with scripting off, under reduced motion, or if the observer never fires, the heading simply reads normally.
if (!calm && 'IntersectionObserver' in window) {
  const heads = Array.from(document.querySelectorAll<HTMLElement>('.section__head h2, .dl h2'))
    .filter((h) => !h.closest('[data-diagram]') && h.getBoundingClientRect().top > window.innerHeight);
  for (const h of heads) {
    const words = (h.textContent ?? '').trim().split(/\s+/);
    h.setAttribute('aria-label', words.join(' '));
    h.innerHTML = words.map((w, i) => `<span class="w" aria-hidden="true" style="--i:${i}">${w}</span>`).join(' ');
    h.classList.add('is-split');
  }
  const io = new IntersectionObserver((entries) => {
    for (const e of entries) if (e.isIntersecting) { e.target.classList.add('is-in'); io.unobserve(e.target); }
  }, { rootMargin: '0px 0px -12% 0px' });
  heads.forEach((h) => io.observe(h));
}

// Scramble: letters flick through dots, then settle, once per hover. Doto is a dot font, so dots read as part of it.
if (fine && !calm) {
  document.querySelectorAll<HTMLElement>('[data-scramble]').forEach((el) => {
    const text = el.textContent ?? '';
    let running = false;
    const run = () => {
      if (running) return;
      running = true;
      const glyphs = '.:·+*';
      let step = 0;
      const total = text.length + 6;
      const tick = () => {
        el.textContent = text.split('').map((c, i) => (c === ' ' || i < step - 6 ? c : glyphs[(i + step) % glyphs.length])).join('');
        if (++step <= total) setTimeout(tick, 28);
        else { el.textContent = text; running = false; }
      };
      tick();
    };
    (el.closest('a, button') ?? el).addEventListener('pointerenter', run);
    (el.closest('a, button') ?? el).addEventListener('focus', run);
  });
}
