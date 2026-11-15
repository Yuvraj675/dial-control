import { useEffect, useRef, useState } from 'react';

/** Scale factor that fits a fixed-size element (the wheel is 450px) into its container. */
export function useFit(natural = 450) {
  const ref = useRef<HTMLDivElement>(null);
  const [scale, setScale] = useState(1);
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const measure = () => setScale(el.clientWidth / natural);
    measure();
    const ro = new ResizeObserver(measure);
    ro.observe(el);
    return () => ro.disconnect();
  }, [natural]);
  return { ref, scale };
}
