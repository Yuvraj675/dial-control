import '../styles/wheel.css';
import { useEffect, useRef, useState } from 'react';
import { motion, useReducedMotion, useSpring } from 'framer-motion';
import { Pause, Play } from 'lucide-react';
import { DonutRadialHUD } from '../../../src/components/hud/DonutRadialHUD';
import { DEFAULT_CONFIG } from '../../../src/data/defaultProfiles';
import { useFit } from './useFit';

const ORDER = ['default', 'media_video', 'code_editor', 'file_manager', 'browser_nav', 'creative_design'];
const tabs = ORDER.map((id) => ({ id, profile: DEFAULT_CONFIG.profiles[id] }));
const label = (s: string) => s.charAt(0) + s.slice(1).toLowerCase();

/** The six built-in wheels, rendered by the app's own component from the app's own profile data. */
export default function ProfileSwitcher() {
  const reduce = useReducedMotion();
  const { ref: fitRef, scale } = useFit(450);
  const [tab, setTab] = useState(2); // Code first: the clearest example
  const [index, setIndex] = useState(0);
  const [hover, setHover] = useState(false);
  const [focus, setFocus] = useState(false);
  const [paused, setPaused] = useState(false);
  const tabRefs = useRef<(HTMLButtonElement | null)[]>([]);
  // The dial leans toward the pointer: a little depth next to the flat controls.
  const rx = useSpring(0, { stiffness: 120, damping: 18 });
  const ry = useSpring(0, { stiffness: 120, damping: 18 });
  const lean = (e: React.PointerEvent<HTMLDivElement>) => {
    if (reduce) return;
    const r = e.currentTarget.getBoundingClientRect();
    rx.set(-((e.clientY - r.top) / r.height - 0.5) * 16);
    ry.set(((e.clientX - r.left) / r.width - 0.5) * 22);
  };
  const level = () => { rx.set(0); ry.set(0); };
  // The wheel's tick marks are trigonometry, which Node and the browser may round differently in the last
  // digit, so it is drawn only in the browser. The slice list stays in the server HTML.
  const [mounted, setMounted] = useState(false);
  useEffect(() => { setMounted(true); }, []);

  const { profile } = tabs[tab];
  const n = profile.actions.length;

  useEffect(() => { setIndex(0); }, [tab]);
  useEffect(() => {
    if (reduce || paused || hover || focus) return;
    const t = window.setInterval(() => setIndex((i) => (i + 1) % n), 1800);
    return () => window.clearInterval(t);
  }, [reduce, paused, hover, focus, n]);

  const onTabKey = (e: React.KeyboardEvent, i: number) => {
    let next = i;
    if (e.key === 'ArrowRight' || e.key === 'ArrowDown') next = (i + 1) % tabs.length;
    else if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') next = (i - 1 + tabs.length) % tabs.length;
    else if (e.key === 'Home') next = 0;
    else if (e.key === 'End') next = tabs.length - 1;
    else return;
    e.preventDefault();
    setTab(next);
    tabRefs.current[next]?.focus();
  };

  return (
    <div className="switch" onMouseEnter={() => setHover(true)} onMouseLeave={() => setHover(false)} onFocus={() => setFocus(true)} onBlur={() => setFocus(false)}>
      <div>
        <div className="switch__tabs" role="tablist" aria-label="Wheels">
          {tabs.map((t, i) => (
            <button
              key={t.id}
              ref={(el) => { tabRefs.current[i] = el; }}
              type="button"
              role="tab"
              id={`tab-${t.id}`}
              aria-selected={i === tab}
              aria-controls="wheel-panel"
              tabIndex={i === tab ? 0 : -1}
              className="tab"
              onClick={() => setTab(i)}
              onKeyDown={(e) => onTabKey(e, i)}
            >
              {t.profile.name}
            </button>
          ))}
        </div>
        <ul className="slices" id="wheel-panel" role="tabpanel" aria-labelledby={`tab-${tabs[tab].id}`}>
          {profile.actions.map((a, i) => (
            <li key={a.id} aria-current={i === index}>
              <button type="button" className="slice-btn" onClick={() => setIndex(i)}>{label(a.label)}</button>
            </li>
          ))}
        </ul>
        <p className="switch__note">
          <button type="button" className="round" onClick={() => setPaused((p) => !p)} aria-pressed={paused} aria-label={paused ? 'Resume the automatic demo' : 'Pause the automatic demo'}>
            {paused ? <Play size={18} aria-hidden="true" /> : <Pause size={18} aria-hidden="true" />}
          </button>
        </p>
      </div>
      <motion.div className="switch__stage" data-kind="product" style={{ rotateX: rx, rotateY: ry, transformPerspective: 1100 }} onPointerMove={lean} onPointerLeave={level} role="img" aria-label={`The ${profile.name} wheel with its slices: ${profile.actions.map((a) => label(a.label)).join(', ')}`}>
        <div ref={fitRef} style={{ position: 'absolute', inset: 0, display: 'grid', placeItems: 'center' }}>
          <div className="wheel__inner" style={{ transform: `translate(-50%, -50%) scale(${scale})` }}>
            <div style={{ position: 'absolute', inset: 0, display: 'grid', placeItems: 'center' }}>
              {mounted && <DonutRadialHUD inline profile={profile} selectedIndex={index} isOpen currentMode="menu" theme="signal_red" onSelectAction={(a) => { const i = profile.actions.findIndex((x) => x.id === a.id); if (i >= 0) setIndex(i); }} />}
            </div>
          </div>
        </div>
      </motion.div>
    </div>
  );
}
