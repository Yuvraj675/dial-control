import React, { useState, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { Mouse, ChevronUp, ChevronDown } from "lucide-react";
import { listen, UnlistenFn } from "@tauri-apps/api/event";

interface ScrollEdgeOverlayProps {
  theme?: "nothing_red" | "stealth" | "cmf_orange";
}

export const ScrollEdgeOverlay: React.FC<ScrollEdgeOverlayProps> = ({
  theme = "nothing_red",
}) => {
  const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
  const [scrollDir, setScrollDir] = useState<"up" | "down" | null>(null);
  const [isScrolling, setIsScrolling] = useState(false);

  const accentColor =
    theme === "nothing_red"
      ? "#E2201F"
      : theme === "stealth"
      ? "#FFFFFF"
      : "#FF5500";

  const glowColor =
    theme === "nothing_red"
      ? "rgba(226, 32, 31, 0.45)"
      : theme === "stealth"
      ? "rgba(255, 255, 255, 0.35)"
      : "rgba(255, 85, 0, 0.45)";

  const idleGlowColor =
    theme === "nothing_red"
      ? "rgba(226, 32, 31, 0.2)"
      : theme === "stealth"
      ? "rgba(255, 255, 255, 0.15)"
      : "rgba(255, 85, 0, 0.2)";

  const idleBorderColor =
    theme === "nothing_red"
      ? "rgba(226, 32, 31, 0.65)"
      : theme === "stealth"
      ? "rgba(255, 255, 255, 0.55)"
      : "rgba(255, 85, 0, 0.65)";

  // Listen for scroll ticks from dial rotation
  useEffect(() => {
    if (!isTauri) return;

    let timer: NodeJS.Timeout | null = null;
    let unlistenTick: UnlistenFn | undefined;

    listen<{ direction?: number }>("scroll:tick", (event) => {
      const dir = event.payload?.direction ?? 1;
      setScrollDir(dir > 0 ? "down" : "up");
      setIsScrolling(true);

      if (timer) clearTimeout(timer);
      timer = setTimeout(() => {
        setIsScrolling(false);
        setScrollDir(null);
      }, 220);
    }).then((unsub) => {
      unlistenTick = unsub;
    });

    return () => {
      if (unlistenTick) unlistenTick();
      if (timer) clearTimeout(timer);
    };
  }, [isTauri]);

  return (
    <div className="fixed inset-0 pointer-events-none select-none overflow-hidden bg-transparent">
      {/* ── 1. Outer Screen Border & Ambient Vignette ────────────────────── */}
      <motion.div
        className="absolute inset-0 rounded-[14px] border-[3px] transition-colors duration-200"
        style={{
          borderColor: isScrolling ? accentColor : idleBorderColor,
          boxShadow: isScrolling
            ? `inset 0 0 60px ${glowColor}, inset 0 0 20px ${accentColor}, 0 0 25px ${glowColor}`
            : `inset 0 0 35px ${idleGlowColor}`,
        }}
        animate={{
          opacity: isScrolling ? 1 : [0.75, 1, 0.75],
        }}
        transition={{
          repeat: isScrolling ? 0 : Infinity,
          duration: 2.2,
          ease: "easeInOut",
        }}
      />

      {/* ── 2. Industrial Glyph Corner Brackets ─────────────────────────── */}
      <div className="absolute top-2 left-2 w-4 h-4 border-t-2 border-l-2" style={{ borderColor: accentColor }} />
      <div className="absolute top-2 right-2 w-4 h-4 border-t-2 border-r-2" style={{ borderColor: accentColor }} />
      <div className="absolute bottom-2 left-2 w-4 h-4 border-b-2 border-l-2" style={{ borderColor: accentColor }} />
      <div className="absolute bottom-2 right-2 w-4 h-4 border-b-2 border-r-2" style={{ borderColor: accentColor }} />

      {/* ── 3. Subtle Status Badge (Bottom Right Screen Edge) ───────────── */}
      <div className="absolute bottom-4 right-5 flex items-center gap-2.5 px-3 py-1.5 rounded-full bg-[#0C0D12]/92 border border-white/15 backdrop-blur-xl shadow-2xl">
        {/* Pulsing Nothing Accent Dot */}
        <motion.div
          animate={{ scale: [1, 1.4, 1], opacity: [0.7, 1, 0.7] }}
          transition={{ repeat: Infinity, duration: 1.4, ease: "easeInOut" }}
          className="w-2 h-2 rounded-full shrink-0"
          style={{
            backgroundColor: accentColor,
            boxShadow: `0 0 8px ${accentColor}`,
          }}
        />

        <div className="flex items-center gap-1.5">
          <Mouse className="w-3.5 h-3.5" style={{ color: accentColor }} />
          <span className="font-ndot text-[11px] text-white tracking-widest uppercase">
            SCROLL MODE
          </span>
        </div>

        {/* Live Scroll Direction Chevron Indicator */}
        <div className="flex items-center text-neutral-400">
          <AnimatePresence mode="wait">
            {scrollDir === "up" ? (
              <motion.div
                key="up"
                initial={{ y: 2, opacity: 0 }}
                animate={{ y: 0, opacity: 1 }}
                exit={{ y: -2, opacity: 0 }}
                transition={{ duration: 0.12 }}
              >
                <ChevronUp className="w-3.5 h-3.5" style={{ color: accentColor }} />
              </motion.div>
            ) : scrollDir === "down" ? (
              <motion.div
                key="down"
                initial={{ y: -2, opacity: 0 }}
                animate={{ y: 0, opacity: 1 }}
                exit={{ y: 2, opacity: 0 }}
                transition={{ duration: 0.12 }}
              >
                <ChevronDown className="w-3.5 h-3.5" style={{ color: accentColor }} />
              </motion.div>
            ) : (
              <span className="text-[10px] text-neutral-400 font-mono tracking-wider ml-0.5">
                • CLICK DIAL TO EXIT
              </span>
            )}
          </AnimatePresence>
        </div>
      </div>
    </div>
  );
};
