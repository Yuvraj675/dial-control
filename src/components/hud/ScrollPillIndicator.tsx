import React, { useState, useEffect } from "react";
import { motion } from "framer-motion";
import { Mouse, ChevronUp, ChevronDown, X } from "lucide-react";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

interface ScrollPillIndicatorProps {
  theme?: "nothing_red" | "stealth" | "cmf_orange";
  onDismiss?: () => void;
}

export const ScrollPillIndicator: React.FC<ScrollPillIndicatorProps> = ({
  theme = "nothing_red",
  onDismiss,
}) => {
  const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
  const [scrollDir, setScrollDir] = useState<"up" | "down" | null>(null);

  const accentColor =
    theme === "nothing_red"
      ? "#E2201F"
      : theme === "stealth"
      ? "#FFFFFF"
      : "#FF5500";

  const glowColor =
    theme === "nothing_red"
      ? "rgba(226, 32, 31, 0.4)"
      : theme === "stealth"
      ? "rgba(255, 255, 255, 0.3)"
      : "rgba(255, 85, 0, 0.4)";

  // Listen for scroll ticks to give live visual feedback on rotary turn
  useEffect(() => {
    if (!isTauri) return;

    let timer: NodeJS.Timeout | null = null;
    let unlistenTick: UnlistenFn | undefined;

    listen<{ direction?: number }>("scroll:tick", (event) => {
      const dir = event.payload?.direction ?? 1;
      setScrollDir(dir > 0 ? "down" : "up");
      if (timer) clearTimeout(timer);
      timer = setTimeout(() => {
        setScrollDir(null);
      }, 240);
    }).then((unsub) => {
      unlistenTick = unsub;
    });

    return () => {
      if (unlistenTick) unlistenTick();
      if (timer) clearTimeout(timer);
    };
  }, [isTauri]);

  const handleStop = () => {
    if (isTauri) {
      invoke("stop_scroll_mode").catch(console.error);
    }
    if (onDismiss) {
      onDismiss();
    }
  };

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        handleStop();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  return (
    <div className="fixed inset-0 flex items-center justify-center bg-transparent select-none p-1">
      <motion.div
        initial={{ y: -20, opacity: 0, scale: 0.9 }}
        animate={{ y: 0, opacity: 1, scale: 1 }}
        exit={{ y: -16, opacity: 0, scale: 0.9 }}
        transition={{ type: "spring", stiffness: 480, damping: 28 }}
        onClick={handleStop}
        className="group relative flex items-center justify-between gap-2.5 px-3.5 py-1.5 rounded-full bg-[#0C0D12]/95 border border-white/15 backdrop-blur-xl shadow-2xl cursor-pointer hover:border-white/30 transition-all duration-200"
        style={{
          boxShadow: `0 0 25px ${glowColor}, 0 6px 20px rgba(0,0,0,0.85)`,
        }}
        title="Click or press dial to stop scroll mode"
      >
        {/* Left: Pulsing Nothing Glyph Accent Dot & Mouse Icon */}
        <div className="flex items-center gap-2">
          <motion.div
            animate={{ scale: [1, 1.35, 1], opacity: [0.75, 1, 0.75] }}
            transition={{ repeat: Infinity, duration: 1.6, ease: "easeInOut" }}
            className="w-2 h-2 rounded-full shrink-0"
            style={{
              backgroundColor: accentColor,
              boxShadow: `0 0 10px ${accentColor}`,
            }}
          />
          <div className="text-white flex items-center justify-center">
            <Mouse className="w-3.5 h-3.5" strokeWidth={2.2} />
          </div>
        </div>

        {/* Center: Ndot Label & Dynamic Directional Indicators */}
        <div className="flex items-center gap-1.5 min-w-0">
          <span className="font-ndot text-[11px] font-bold text-white tracking-widest uppercase">
            SCROLL
          </span>

          <div className="flex flex-col items-center justify-center -space-y-1 text-neutral-400">
            <ChevronUp
              className={`w-3 h-3 transition-colors duration-150 ${
                scrollDir === "up" ? "text-white scale-125" : "text-neutral-500"
              }`}
              strokeWidth={scrollDir === "up" ? 3 : 2}
            />
            <ChevronDown
              className={`w-3 h-3 transition-colors duration-150 ${
                scrollDir === "down" ? "text-white scale-125" : "text-neutral-500"
              }`}
              strokeWidth={scrollDir === "down" ? 3 : 2}
            />
          </div>
        </div>

        {/* Right: Exit / Dismiss Button */}
        <div className="flex items-center justify-center pl-1 border-l border-white/10 text-neutral-400 group-hover:text-white transition-colors">
          <X className="w-3.5 h-3.5" strokeWidth={2} />
        </div>
      </motion.div>
    </div>
  );
};
