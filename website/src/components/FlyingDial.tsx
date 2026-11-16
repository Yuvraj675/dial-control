import '../styles/wheel.css';
import { useCallback, useEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { motion, useReducedMotion, useSpring } from 'framer-motion';
import { ChevronLeft, ChevronRight, Pause, Play } from 'lucide-react';
import { DonutRadialHUD } from '../../../src/components/hud/DonutRadialHUD';
import { DEFAULT_CONFIG } from '../../../src/data/defaultProfiles';
import type { DialProfile, WheelAction } from '../../../src/types/profile';
import { CALLOUTS, currentCallout, dialAt } from './diagramData';

type Mode = 'menu' | 'volume' | 'brightness' | 'scroll';

const PROFILES = DEFAULT_CONFIG.profiles;
const ORDER = ['default', 'media_video', 'code_editor', 'file_manager', 'browser_nav', 'creative_design'];

/** The app's Home dial: every profile as a slice, then Close (built the way App.tsx builds it). */
const HOME: DialProfile = {
  id: 'home',
  name: 'Home',
  icon: 'home',
  match_patterns: [],
  actions: [
    ...ORDER.map((id): WheelAction => ({ id: `profile_${id}`, label: PROFILES[id].name.toUpperCase(), icon: PROFILES[id].icon || 'sliders', type: 'profile', action: id })),
    { id: 'close_hud', label: 'CLOSE', icon: 'close', type: 'close' },
  ],
};
const GAUGE_MODES: Mode[] = ['volume', 'brightness', 'scroll'];
const iconSrc = `${import.meta.env.BASE_URL}images/app-icon.webp`;

const label = (s: string) => s.charAt(0) + s.slice(1).toLowerCase();
const clamp = (x: number, lo = 0, hi = 1) => Math.min(hi, Math.max(lo, x));
const lerp = (a: number, b: number, t: number) => a + (b - a) * t;
const easeInOut = (t: number) => (t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2);

type Phase = 'hero' | 'flight' | 'diagram' | 'away';
interface Geo {
  hx: number; hy: number; hw: number;            // the hero placeholder: centre in page coordinates, width
  dx: number; dw: number; dOff: number;          // the diagram placeholder: centre x, width, centre y inside its pinned stage
  D: number; R: number;                          // scroll offsets where the diagram pins and where it lets go
  top: number;                                   // where the pinned stage docks (just under the nav)
}

/** The app's own wheel, flown by the page. In the hero it opens from a simulated double press and turns by
 *  itself; scrolling lifts it out of the hero on a 3D arc, brings it in front and centre, and pins it while
 *  the anatomy diagram draws its labelled parts one by one. Where the effect is off (narrow screens, reduced
 *  motion) it simply stays in the hero. */
export default function FlyingDial() {
  const reduce = useReducedMotion();
  const wrapRef = useRef<HTMLDivElement>(null);
  const geo = useRef<Geo | null>(null);
  const phaseRef = useRef<Phase>('hero');
  const curRef = useRef(-1);
  const enhancedRef = useRef(false);
  const [uiSlot, setUiSlot] = useState<HTMLElement | null>(null);

  const [open, setOpen] = useState(false);
  const [intro, setIntro] = useState(!reduce);
  const [profileId, setProfileId] = useState('default');
  const [index, setIndex] = useState(0);
  const [mode, setMode] = useState<Mode>('menu');
  const [gauge, setGauge] = useState(55);
  const [auto, setAuto] = useState(!reduce);
  const [userPaused, setUserPaused] = useState(false);
  const [hover, setHover] = useState(false);
  const [focus, setFocus] = useState(false);
  const [note, setNote] = useState('');
  const [touched, setTouched] = useState(false);
  const [phase, setPhase] = useState<Phase>('hero');
  const [ripple, setRipple] = useState(0);
  const lastShape = useRef('');
  const resume = useRef<number | undefined>(undefined);
  const userPausedRef = useRef(false);
  userPausedRef.current = userPaused;

  const tiltX = useSpring(0, { stiffness: 110, damping: 18 });
  const tiltY = useSpring(0, { stiffness: 110, damping: 18 });

  const profile = profileId === 'home' ? HOME : PROFILES[profileId];
  const actions = profile.actions;

  // ── Placing the dial: three modes that hand over at exact matches, so there is never a jump ──
  //   before the diagram:  absolute in the page, on a lerp from the hero spot to the diagram spot (scrolls natively)
  //   while pinned:        fixed at the stage's centre (no script movement at all)
  //   after it lets go:    absolute again, at the spot where it was pinned (scrolls away natively)
  useEffect(() => {
    const hero = document.querySelector<HTMLElement>('[data-dial-hero]');
    const stage = document.querySelector<HTMLElement>('.diagram__stage');
    const track = document.querySelector<HTMLElement>('[data-diagram]');
    const dial = document.querySelector<HTMLElement>('[data-dial-diagram]');
    const wrap = wrapRef.current;
    if (!hero || !wrap) return;
    setUiSlot(document.querySelector<HTMLElement>('[data-dial-ui]'));

    const mq = window.matchMedia('(min-width: 72rem) and (prefers-reduced-motion: no-preference)');
    let frame = 0;

    const measure = () => {
      const sy = window.scrollY;
      const h = hero.getBoundingClientRect();
      const g: Geo = { hx: h.left + h.width / 2, hy: h.top + sy + h.height / 2, hw: h.width, dx: 0, dw: 0, dOff: 0, D: 0, R: 0, top: 0 };
      if (stage && track && dial) {
        const t = track.getBoundingClientRect();
        const s = stage.getBoundingClientRect();
        const d = dial.getBoundingClientRect();
        g.dx = d.left + d.width / 2; g.dw = d.width; g.dOff = d.top + d.height / 2 - s.top;
        g.top = parseFloat(getComputedStyle(stage).top) || 0;
        g.D = t.top + sy - g.top; g.R = t.top + sy + t.height - window.innerHeight;
      }
      geo.current = g;
    };

    const place = (mode: 'absolute' | 'fixed', cx: number, cy: number, k: number, z: number, fx: number, fy: number) => {
      wrap.style.position = mode;
      wrap.style.left = `${cx - 225}px`;
      wrap.style.top = `${cy - 225}px`;
      wrap.style.setProperty('--k', String(k));
      wrap.style.setProperty('--z', `${z}px`);
      wrap.style.setProperty('--fx', `${fx}deg`);
      wrap.style.setProperty('--fy', `${fy}deg`);
    };

    const apply = () => {
      frame = 0;
      const g = geo.current;
      if (!g) return;
      const sy = window.scrollY;
      const k0 = g.hw / 450;
      let next: Phase = 'hero';

      if (!enhancedRef.current || !g.D) {
        place('absolute', g.hx, g.hy, k0, 0, 0, 0);
      } else if (sy < g.D) {
        // The flight: from the hero to the diagram's centre, tilted and pushed toward you on the way (a bell
        // curve, so it starts and ends flat).
        const s0 = 0;
        const p = clamp((sy - s0) / (g.D - s0));
        // Part linear so the dial starts to ride along at once and is never left to scroll under the nav.
        const e = 0.45 * p + 0.55 * easeInOut(p);
        const bell = Math.sin(Math.PI * p);
        place('absolute', lerp(g.hx, g.dx, e), lerp(g.hy, g.D + g.top + g.dOff, e), lerp(k0, g.dw / 450, e) * (1 + 0.1 * bell), bell * 170, bell * 14, -bell * 26);
        next = p > 0.002 ? 'flight' : 'hero';
      } else if (sy <= g.R) {
        place('fixed', g.dx, g.top + g.dOff, g.dw / 450, 0, 0, 0);
        next = 'diagram';
        // The dial does what the labels say: turns, clicks into the volume gauge, and so on.
        const q = (sy - g.D) / (g.R - g.D);
        const cur = currentCallout(q);
        if (cur !== curRef.current) {
          curRef.current = cur;
          window.dispatchEvent(new CustomEvent('diagram:current', { detail: { id: cur >= 0 ? CALLOUTS[cur].id : '' } }));
        }
        const d = dialAt(q);
        setOpen(d.open); setProfileId(d.profileId); setIndex(d.index); setMode(d.mode); setGauge(d.gauge);
      } else {
        place('absolute', g.dx, g.R + g.top + g.dOff, g.dw / 450, 0, 0, 0);
        next = 'away';
      }

      if (next !== phaseRef.current) {
        phaseRef.current = next;
        setPhase(next);
        tiltX.set(0); tiltY.set(0);
        if (next !== 'diagram' && curRef.current !== -1) {
          curRef.current = -1;
          window.dispatchEvent(new CustomEvent('diagram:current', { detail: { id: '' } }));
        }
      }
      wrap.style.visibility = 'visible';
    };

    const schedule = () => { if (!frame) frame = requestAnimationFrame(apply); };
    const refresh = () => {
      enhancedRef.current = mq.matches;
      document.documentElement.classList.toggle('js-3d', mq.matches);
      measure(); schedule();
    };

    // The dial follows the pointer a little while it is pinned: a hint of depth.
    const onPointer = (e: PointerEvent) => {
      if (!enhancedRef.current) return;
      const amp = phaseRef.current === 'diagram' ? 1 : phaseRef.current === 'hero' ? 0.55 : 0;
      if (!amp) return;
      tiltX.set(-(e.clientY / window.innerHeight - 0.5) * 9 * amp);
      tiltY.set((e.clientX / window.innerWidth - 0.5) * 13 * amp);
    };
    const offX = tiltX.on('change', (v) => wrap.style.setProperty('--tx', `${v}deg`));
    const offY = tiltY.on('change', (v) => wrap.style.setProperty('--ty', `${v}deg`));

    refresh();
    hero.closest('[data-stage]')?.classList.add('is-live');
    window.addEventListener('scroll', schedule, { passive: true });
    window.addEventListener('resize', refresh, { passive: true });
    window.addEventListener('pointermove', onPointer, { passive: true });
    mq.addEventListener('change', refresh);
    const ro = new ResizeObserver(() => { measure(); schedule(); });
    ro.observe(document.body);
    void document.fonts?.ready.then(() => { measure(); schedule(); });
    return () => {
      window.removeEventListener('scroll', schedule); window.removeEventListener('resize', refresh); window.removeEventListener('pointermove', onPointer);
      mq.removeEventListener('change', refresh); ro.disconnect(); offX(); offY(); if (frame) cancelAnimationFrame(frame);
    };
  }, [tiltX, tiltY]);

  // The simulated double press, then the wheel opens.
  useEffect(() => {
    if (reduce) { setOpen(true); setIntro(false); return; }
    const a = window.setTimeout(() => { setOpen(true); setIntro(false); }, 1050);
    return () => window.clearTimeout(a);
  }, [reduce]);

  // Leaving the hero hands the dial to the diagram; coming back hands it back to the visitor.
  const lastPhase = useRef<Phase>('hero');
  useEffect(() => {
    const prev = lastPhase.current;
    lastPhase.current = phase;
    if (phase === 'diagram') { setAuto(false); setOpen(true); setMode('menu'); setProfileId('default'); }
    else if (phase === 'hero' && prev !== 'hero') { setProfileId('default'); setMode('menu'); setOpen(true); setIndex(0); setNote(''); if (!userPausedRef.current && !reduce) setAuto(true); }
    else if (phase === 'flight') setAuto(false);
  }, [phase, reduce]);

  // Autoplay: only in the hero; pauses on hover and focus, and when the visitor takes over.
  useEffect(() => {
    if (!auto || hover || focus || !open || mode !== 'menu' || reduce || phase !== 'hero') return;
    const t = window.setInterval(() => setIndex((i) => (i + 1) % actions.length), 1700);
    return () => window.clearInterval(t);
  }, [auto, hover, focus, open, mode, reduce, phase, actions.length]);

  // A ring at the centre each time the walkthrough clicks something (opens a gauge, a dial, closes the wheel).
  useEffect(() => {
    if (phase !== 'diagram') { lastShape.current = ''; return; }
    const shape = `${open}|${mode}|${profileId}`;
    if (lastShape.current && shape !== lastShape.current) setRipple((n) => n + 1);
    lastShape.current = shape;
  }, [phase, open, mode, profileId]);

  const takeOver = useCallback(() => {
    setTouched(true);
    setAuto(false);
    window.clearTimeout(resume.current);
    if (!userPaused && !reduce) resume.current = window.setTimeout(() => setAuto(true), 9000);
  }, [userPaused, reduce]);
  useEffect(() => () => window.clearTimeout(resume.current), []);

  const turn = useCallback((dir: 1 | -1) => {
    takeOver();
    if (!open) return;
    if (mode === 'menu') setIndex((i) => (i + dir + actions.length) % actions.length);
    else if (mode !== 'scroll') setGauge((g) => Math.max(0, Math.min(100, g + dir * 5)));
    setNote('');
  }, [mode, open, takeOver, actions.length]);

  const press = useCallback(() => {
    takeOver();
    if (!open) { setOpen(true); return; }
    if (mode !== 'menu') { setMode('menu'); setNote(''); return; }
    const a = actions[index];
    const m = (a.action ?? a.id.replace(/_mode$/, '')) as Mode;
    if (a.type === 'mode' && GAUGE_MODES.includes(m)) { setMode(m); setNote(''); return; }
    if (a.type === 'close') { setOpen(false); setProfileId('default'); setIndex(0); setNote(''); return; }
    if (a.type === 'home') { setProfileId('home'); setIndex(0); setNote('Pick a dial. The real app lists yours here.'); return; }
    if (a.type === 'profile' && a.action && PROFILES[a.action]) { setProfileId(a.action); setIndex(0); setNote(`Switched to ${PROFILES[a.action].name}. Home brings you back.`); return; }
    setNote(`Runs ${label(a.label)}`);
  }, [index, mode, open, takeOver, actions]);

  const onKey = (e: React.KeyboardEvent) => {
    const k = e.key;
    if (k === 'ArrowRight' || k === 'ArrowDown') { e.preventDefault(); turn(1); }
    else if (k === 'ArrowLeft' || k === 'ArrowUp') { e.preventDefault(); turn(-1); }
    else if (k === 'Enter' || k === ' ') { e.preventDefault(); press(); }
    else if (k === 'Escape') { e.preventDefault(); takeOver(); if (mode !== 'menu') setMode('menu'); else setOpen(false); }
  };

  const togglePause = () => {
    const next = !userPaused;
    setUserPaused(next);
    window.clearTimeout(resume.current);
    setAuto(!next && !reduce);
  };

  const active = actions[index];
  const caption = !open
    ? 'Click the dial to open the wheel.'
    : mode === 'volume' ? `Volume ${gauge}%. Turn with the arrow keys, click the centre to go back.`
    : mode === 'brightness' ? `Brightness ${gauge}%. Turn with the arrow keys, click the centre to go back.`
    : mode === 'scroll' ? 'Scroll mode: turning the dial scrolls the page under it. Click the centre to stop.'
    : note || (touched ? `${label(active.label)}. Press Enter to run it.` : 'The app’s own wheel, running here. Use the arrow keys or the buttons.');

  return (
    <>
      <div
        ref={wrapRef}
        className="fly"
        data-phase={phase}
        data-kind="product"
        role="group"
        aria-label="Live demo of the Dial Control wheel. Arrow keys turn it, Enter runs a slice, Escape goes back."
        tabIndex={0}
        onKeyDown={onKey}
        onMouseEnter={() => setHover(true)}
        onMouseLeave={() => setHover(false)}
        onFocus={() => setFocus(true)}
        onBlur={() => setFocus(false)}
        style={{ position: 'absolute', left: 0, top: 0, visibility: 'hidden' }}
      >
        <div className="fly__dial">
          <DonutRadialHUD
            inline
            profile={profile}
            selectedIndex={index}
            isOpen={open}
            currentMode={mode}
            gaugeValue={gauge}
            theme="signal_red"
            onSelectAction={(a) => { const i = actions.findIndex((x) => x.id === a.id); takeOver(); if (i >= 0) setIndex(i); }}
            onDismiss={() => { setMode('menu'); takeOver(); }}
          />

          {phase === 'diagram' && ripple > 0 && !reduce && (
            <div className="wheel__press" aria-hidden="true">
              <motion.span key={`click-${ripple}`} className="wheel__ring" initial={{ scale: 0.9, opacity: 0.95 }} animate={{ scale: 2.4, opacity: 0 }} transition={{ duration: 0.65, ease: [0.16, 1, 0.3, 1] }} />
            </div>
          )}

          {!open && (
            <div className="wheel__press">
              {intro && !reduce && [0, 1].map((n) => (
                <motion.span
                  key={n}
                  className="wheel__ring"
                  initial={{ scale: 0.9, opacity: 0.9 }}
                  animate={{ scale: 2.6, opacity: 0 }}
                  transition={{ duration: 0.55, delay: 0.15 + n * 0.32, ease: [0.16, 1, 0.3, 1] }}
                  aria-hidden="true"
                />
              ))}
              <motion.button
                type="button"
                onClick={press}
                aria-label="Open the wheel"
                style={{ pointerEvents: 'auto', background: 'none', border: 0, padding: 0, cursor: 'pointer', borderRadius: 24 }}
                animate={intro && !reduce ? { scale: [1, 0.9, 1, 0.9, 1] } : { scale: 1 }}
                transition={{ duration: 0.9, times: [0, 0.2, 0.4, 0.6, 0.8], ease: [0.16, 1, 0.3, 1] }}
              >
                <img src={iconSrc} alt="" width={96} height={96} />
              </motion.button>
            </div>
          )}
        </div>
      </div>

      {uiSlot && createPortal(
        <div className="wheel__ui" data-away={phase !== 'hero'}>
          <div className="wheel__controls">
            <button type="button" className="round" onClick={() => turn(-1)} aria-label="Turn the dial left"><ChevronLeft size={20} aria-hidden="true" /></button>
            <button type="button" className="round" onClick={press} aria-label="Click the dial"><span aria-hidden="true" style={{ width: 12, height: 12, borderRadius: '50%', background: 'currentColor' }} /></button>
            <button type="button" className="round" onClick={() => turn(1)} aria-label="Turn the dial right"><ChevronRight size={20} aria-hidden="true" /></button>
            <button type="button" className="round" onClick={togglePause} aria-pressed={userPaused} aria-label={userPaused ? 'Resume the automatic demo' : 'Pause the automatic demo'}>
              {userPaused ? <Play size={18} aria-hidden="true" /> : <Pause size={18} aria-hidden="true" />}
            </button>
          </div>
          <p className="wheel__caption" aria-live={touched ? 'polite' : 'off'}>{caption}</p>
        </div>,
        uiSlot,
      )}
    </>
  );
}
