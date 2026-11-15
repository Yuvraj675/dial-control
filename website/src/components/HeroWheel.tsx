import '../styles/wheel.css';
import { useCallback, useEffect, useRef, useState } from 'react';
import { motion, useReducedMotion } from 'framer-motion';
import { ChevronLeft, ChevronRight, Pause, Play } from 'lucide-react';
import { DonutRadialHUD } from '../../../src/components/hud/DonutRadialHUD';
import { DEFAULT_CONFIG } from '../../../src/data/defaultProfiles';
import type { DialProfile, WheelAction } from '../../../src/types/profile';
import { useFit } from './useFit';

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

/** The app's own wheel, running on the page. It opens from a simulated double press, turns by itself
 *  until a visitor takes over, and answers the arrow keys, Enter, Escape, the buttons and clicks. */
export default function HeroWheel() {
  const reduce = useReducedMotion();
  const { ref: fitRef, scale } = useFit(450);
  const rootRef = useRef<HTMLDivElement>(null);

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
  const resume = useRef<number | undefined>(undefined);

  const profile = profileId === 'home' ? HOME : PROFILES[profileId];
  const actions = profile.actions;

  // Tell the page the live wheel is here, so the still image can step aside.
  useEffect(() => {
    rootRef.current?.closest('[data-stage]')?.classList.add('is-live');
  }, []);

  // The simulated double press, then the wheel opens.
  useEffect(() => {
    if (reduce) { setOpen(true); setIntro(false); return; }
    const a = window.setTimeout(() => { setOpen(true); setIntro(false); }, 1050);
    return () => window.clearTimeout(a);
  }, [reduce]);

  // Autoplay: pauses on hover and focus, and when the visitor takes over.
  useEffect(() => {
    if (!auto || hover || focus || !open || mode !== 'menu' || reduce) return;
    const t = window.setInterval(() => setIndex((i) => (i + 1) % actions.length), 1700);
    return () => window.clearInterval(t);
  }, [auto, hover, focus, open, mode, reduce, actions.length]);

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
    <div
      ref={rootRef}
      className="wheel"
      role="group"
      aria-label="Live demo of the Dial Control wheel. Arrow keys turn it, Enter runs a slice, Escape goes back."
      tabIndex={0}
      onKeyDown={onKey}
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
      onFocus={() => setFocus(true)}
      onBlur={() => setFocus(false)}
      data-kind="product"
    >
      <div ref={fitRef} style={{ position: 'absolute', inset: 0, display: 'grid', placeItems: 'center' }}>
        <div className="wheel__inner" style={{ transform: `translate(-50%, -50%) scale(${scale})` }}>
          <div style={{ position: 'absolute', inset: 0, display: 'grid', placeItems: 'center' }}>
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
          </div>
        </div>

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

      <div className="wheel__ui">
        <div className="wheel__controls">
          <button type="button" className="round" onClick={() => turn(-1)} aria-label="Turn the dial left"><ChevronLeft size={20} aria-hidden="true" /></button>
          <button type="button" className="round" onClick={press} aria-label="Click the dial"><span aria-hidden="true" style={{ width: 12, height: 12, borderRadius: '50%', background: 'currentColor' }} /></button>
          <button type="button" className="round" onClick={() => turn(1)} aria-label="Turn the dial right"><ChevronRight size={20} aria-hidden="true" /></button>
          <button type="button" className="round" onClick={togglePause} aria-pressed={userPaused} aria-label={userPaused ? 'Resume the automatic demo' : 'Pause the automatic demo'}>
            {userPaused ? <Play size={18} aria-hidden="true" /> : <Pause size={18} aria-hidden="true" />}
          </button>
        </div>
        <p className="wheel__caption" aria-live={touched ? 'polite' : 'off'}>{caption}</p>
      </div>
    </div>
  );
}
