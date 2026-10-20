import React from "react";
import { motion, AnimatePresence } from "framer-motion";
import { calculateDonutSlice } from "../../utils/donutGeometry";
import { DialProfile, WheelAction } from "../../types/profile";
import { getActionIcon } from "../../utils/iconMap";

interface DonutRadialHUDProps {
  profile: DialProfile;
  selectedIndex: number;
  isOpen: boolean;
  currentMode: "menu" | "volume" | "brightness";
  gaugeValue?: number; // 0-100
  theme?: "nothing_red" | "stealth" | "cmf_orange";
  onSelectAction?: (action: WheelAction) => void;
}

export const DonutRadialHUD: React.FC<DonutRadialHUDProps> = ({
  profile,
  selectedIndex,
  isOpen,
  currentMode,
  gaugeValue = 50,
  theme = "nothing_red",
  onSelectAction,
}) => {
  const size = 450;
  const cx = size / 2;
  const cy = size / 2;
  const rOuter = 188;
  const rInner = 120;

  const actions = profile.actions || [];
  const activeAction = actions[selectedIndex] || actions[0];

  // Authentic Nothing Signature Palette (Matching NThing-UI & Nothing Phone)
  const accentColor =
    theme === "nothing_red"
      ? "#E2201F"
      : theme === "stealth"
      ? "#FFFFFF"
      : "#FF5500";

  const glowColor =
    theme === "nothing_red"
      ? "rgba(226, 32, 31, 0.35)"
      : theme === "stealth"
      ? "rgba(255, 255, 255, 0.2)"
      : "rgba(255, 85, 0, 0.35)";

  // Minimal mechanical tick marks around outer rim
  const tickCount = 36;
  const ticks = Array.from({ length: tickCount }).map((_, i) => {
    const angle = (2 * Math.PI * i) / tickCount - Math.PI / 2;
    const r1 = rOuter + 8;
    const r2 = rOuter + (i % 3 === 0 ? 14 : 11);
    return {
      x1: cx + r1 * Math.cos(angle),
      y1: cy + r1 * Math.sin(angle),
      x2: cx + r2 * Math.cos(angle),
      y2: cy + r2 * Math.sin(angle),
      isMajor: i % 3 === 0,
    };
  });

  return (
    <AnimatePresence>
      {isOpen && (
        <div className="fixed inset-0 flex items-center justify-center bg-transparent pointer-events-auto">
          <motion.div
            initial={{ scale: 0.65, rotate: -20, opacity: 0, filter: "blur(12px)" }}
            animate={{ scale: 1, rotate: 0, opacity: 1, filter: "blur(0px)" }}
            exit={{ scale: 0.8, rotate: 10, opacity: 0, filter: "blur(8px)" }}
            transition={{
              type: "spring",
              stiffness: 420,
              damping: 26,
              mass: 0.85,
            }}
            className="relative flex items-center justify-center"
            style={{ width: size, height: size }}
          >
            {/* Ambient Dark Obsidian Backing */}
            <div
              className="absolute inset-4 rounded-full pointer-events-none transition-all duration-300"
              style={{
                boxShadow: `0 0 50px ${glowColor}, inset 0 0 40px rgba(0,0,0,0.85)`,
              }}
            />

            {/* SVG Layer for Slices, Bezel Ticks, and Gauge */}
            <svg
              width={size}
              height={size}
              viewBox={`0 0 ${size} ${size}`}
              className="absolute inset-0 overflow-visible"
            >
              <defs>
                <filter id="nothing-glow" x="-20%" y="-20%" width="140%" height="140%">
                  <feGaussianBlur stdDeviation="5" result="blur" />
                  <feComposite in="SourceGraphic" in2="blur" operator="over" />
                </filter>
              </defs>

              {/* ── Outer Bezel Ticks ────────────────────────────────────── */}
              <g className="opacity-30">
                {ticks.map((t, i) => (
                  <line
                    key={i}
                    x1={t.x1}
                    y1={t.y1}
                    x2={t.x2}
                    y2={t.y2}
                    stroke={t.isMajor ? "#FFFFFF" : "rgba(255, 255, 255, 0.4)"}
                    strokeWidth={t.isMajor ? 1.5 : 1}
                  />
                ))}
              </g>

              {currentMode === "menu" ? (
                // ── Donut Slices Mode ───────────────────────────────────────
                <g>
                  {actions.map((action, idx) => {
                    const isSelected = idx === selectedIndex;
                    const geom = calculateDonutSlice(
                      idx,
                      actions.length,
                      cx,
                      cy,
                      rInner,
                      rOuter,
                      0.03
                    );

                    return (
                      <motion.g
                        key={action.id}
                        initial={false}
                        animate={{
                          x: isSelected ? geom.activeTranslate.x : 0,
                          y: isSelected ? geom.activeTranslate.y : 0,
                          scale: isSelected ? 1.03 : 1,
                        }}
                        transition={{
                          type: "spring",
                          stiffness: 450,
                          damping: 24,
                        }}
                        className="cursor-pointer"
                        onClick={() => onSelectAction && onSelectAction(action)}
                      >
                        {/* Wedge Polygon */}
                        <path
                          d={geom.path}
                          fill={isSelected ? accentColor : "#15161A"}
                          stroke={isSelected ? "#FFFFFF" : "rgba(255, 255, 255, 0.08)"}
                          strokeWidth={isSelected ? 1.5 : 1}
                          filter={isSelected ? "url(#nothing-glow)" : undefined}
                          className="transition-colors duration-150"
                        />

                        {/* Crisp Lucide Vector Icon via foreignObject */}
                        <foreignObject
                          x={geom.iconPosition.x - 14}
                          y={geom.iconPosition.y - 14}
                          width={28}
                          height={28}
                          className="pointer-events-none select-none"
                        >
                          <div
                            className={`w-full h-full flex items-center justify-center transition-colors ${
                              isSelected
                                ? (theme === "stealth" ? "text-black" : "text-white")
                                : "text-neutral-300"
                            }`}
                          >
                            {getActionIcon(action.icon, 20, "", 2.0)}
                          </div>
                        </foreignObject>
                      </motion.g>
                    );
                  })}
                </g>
              ) : (
                // ── Continuous Circular Gauge Mode (Volume / Brightness) ───
                <g>
                  {/* Gauge Track */}
                  <circle
                    cx={cx}
                    cy={cy}
                    r={(rInner + rOuter) / 2}
                    fill="none"
                    stroke="#16181F"
                    strokeWidth={rOuter - rInner}
                  />

                  {/* Active Gauge Arc */}
                  {(() => {
                    const rMid = (rInner + rOuter) / 2;
                    const circumference = 2 * Math.PI * rMid;
                    const strokeDashoffset =
                      circumference - (gaugeValue / 100) * circumference;

                    return (
                      <motion.circle
                        cx={cx}
                        cy={cy}
                        r={rMid}
                        fill="none"
                        stroke={accentColor}
                        strokeWidth={rOuter - rInner - 6}
                        strokeLinecap="round"
                        strokeDasharray={circumference}
                        animate={{ strokeDashoffset }}
                        transition={{ type: "spring", stiffness: 350, damping: 28 }}
                        transform={`rotate(-90 ${cx} ${cy})`}
                        filter="url(#nothing-glow)"
                      />
                    );
                  })()}
                </g>
              )}
            </svg>

            {/* ── Central Clean Obsidian Disc (No Knurling Pattern) ────────── */}
            <motion.div
              className="relative z-10 flex flex-col items-center justify-center rounded-full bg-[#121316] border border-white/10 shadow-2xl"
              style={{
                width: rInner * 2 - 14,
                height: rInner * 2 - 14,
              }}
            >
              {currentMode === "menu" ? (
                <div className="flex flex-col items-center justify-center text-center px-4">
                  {/* Active Action Lucide Vector Icon with Micro-Spring */}
                  <motion.div
                    key={activeAction?.id}
                    initial={{ scale: 0.75, y: 3, opacity: 0 }}
                    animate={{ scale: 1, y: 0, opacity: 1 }}
                    transition={{ type: "spring", stiffness: 500, damping: 24 }}
                    className="p-3.5 rounded-full bg-white/[0.06] border border-white/10 text-white flex items-center justify-center"
                    style={{
                      boxShadow: `0 0 20px ${glowColor}`,
                    }}
                  >
                    {getActionIcon(activeAction?.icon || "disc", 30, "text-white", 2.0)}
                  </motion.div>

                  {/* ONLY ONE Single Main Text in Ndot57 */}
                  <div className="font-ndot text-sm text-white tracking-widest uppercase mt-2.5">
                    {activeAction?.label}
                  </div>
                </div>
              ) : (
                /* Gauge Center Mode */
                <div className="flex flex-col items-center justify-center text-center">
                  <div className="mb-2 p-2.5 rounded-full bg-white/[0.06] border border-white/10 text-white">
                    {getActionIcon(currentMode === "volume" ? "volume" : "brightness", 24, "text-white", 2.0)}
                  </div>
                  <motion.div
                    key={gaugeValue}
                    initial={{ scale: 0.95 }}
                    animate={{ scale: 1 }}
                    className="font-ndot text-4xl text-white tracking-widest"
                  >
                    {Math.round(gaugeValue)}%
                  </motion.div>
                  <span className="font-ndot text-[11px] uppercase text-neutral-400 tracking-widest mt-1">
                    {currentMode === "volume" ? "VOLUME" : "BRIGHTNESS"}
                  </span>
                </div>
              )}
            </motion.div>
          </motion.div>
        </div>
      )}
    </AnimatePresence>
  );
};
