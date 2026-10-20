import React from "react";
import { motion, AnimatePresence } from "framer-motion";
import { calculateDonutSlice } from "../../utils/donutGeometry";
import { DialProfile, WheelAction } from "../../types/profile";

interface DonutRadialHUDProps {
  profile: DialProfile;
  selectedIndex: number;
  isOpen: boolean;
  currentMode: "menu" | "volume" | "brightness";
  gaugeValue?: number; // 0-100
  theme?: "cmf_orange" | "cmf_cyan" | "stealth";
  onSelectAction?: (action: WheelAction) => void;
  onClose?: () => void;
}

export const DonutRadialHUD: React.FC<DonutRadialHUDProps> = ({
  profile,
  selectedIndex,
  isOpen,
  currentMode,
  gaugeValue = 50,
  theme = "cmf_orange",
  onSelectAction,
}) => {
  const size = 440;
  const cx = size / 2;
  const cy = size / 2;
  const rOuter = 185;
  const rInner = 118;

  const actions = profile.actions || [];
  const activeAction = actions[selectedIndex] || actions[0];

  const accentColor =
    theme === "cmf_orange"
      ? "#FF5500"
      : theme === "cmf_cyan"
      ? "#00E5FF"
      : "#FFFFFF";

  const glowColor =
    theme === "cmf_orange"
      ? "rgba(255, 85, 0, 0.45)"
      : theme === "cmf_cyan"
      ? "rgba(0, 229, 255, 0.45)"
      : "rgba(255, 255, 255, 0.3)";

  return (
    <AnimatePresence>
      {isOpen && (
        <div className="fixed inset-0 flex items-center justify-center bg-transparent pointer-events-auto">
          <motion.div
            initial={{ scale: 0.65, rotate: -22, opacity: 0, filter: "blur(10px)" }}
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
            {/* Ambient Background Glow Ring */}
            <div
              className="absolute inset-4 rounded-full pointer-events-none transition-all duration-300"
              style={{
                boxShadow: `0 0 60px ${glowColor}, inset 0 0 40px rgba(0,0,0,0.8)`,
              }}
            />

            {/* SVG Canvas for Slices & Gauge */}
            <svg
              width={size}
              height={size}
              viewBox={`0 0 ${size} ${size}`}
              className="absolute inset-0 overflow-visible"
            >
              <defs>
                <filter id="hud-glow" x="-20%" y="-20%" width="140%" height="140%">
                  <feGaussianBlur stdDeviation="6" result="blur" />
                  <feComposite in="SourceGraphic" in2="blur" operator="over" />
                </filter>
              </defs>

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
                      0.035
                    );

                    return (
                      <motion.g
                        key={action.id}
                        initial={false}
                        animate={{
                          x: isSelected ? geom.activeTranslate.x : 0,
                          y: isSelected ? geom.activeTranslate.y : 0,
                          scale: isSelected ? 1.04 : 1,
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
                          fill={isSelected ? accentColor : "#14161B"}
                          stroke={isSelected ? "#FFFFFF" : "rgba(255, 255, 255, 0.09)"}
                          strokeWidth={isSelected ? 2 : 1}
                          filter={isSelected ? "url(#hud-glow)" : undefined}
                          className="transition-colors duration-200"
                        />

                        {/* Slice Icon */}
                        <text
                          x={geom.iconPosition.x}
                          y={geom.iconPosition.y}
                          textAnchor="middle"
                          dominantBaseline="central"
                          fill={isSelected ? (theme === "stealth" ? "#000000" : "#000000") : "#E0E4EC"}
                          fontSize="20"
                          fontWeight="bold"
                          className="select-none pointer-events-none"
                        >
                          {action.icon}
                        </text>
                      </motion.g>
                    );
                  })}
                </g>
              ) : (
                // ── Continuous Circular Gauge Mode (Volume / Brightness) ────
                <g>
                  {/* Track Circle */}
                  <circle
                    cx={cx}
                    cy={cy}
                    r={(rInner + rOuter) / 2}
                    fill="none"
                    stroke="#1A1D24"
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
                        strokeWidth={rOuter - rInner - 8}
                        strokeLinecap="round"
                        strokeDasharray={circumference}
                        animate={{ strokeDashoffset }}
                        transition={{ type: "spring", stiffness: 300, damping: 28 }}
                        transform={`rotate(-90 ${cx} ${cy})`}
                        filter="url(#hud-glow)"
                      />
                    );
                  })()}
                </g>
              )}
            </svg>

            {/* ── Central Glass Hub (Capsule) ────────────────────────────── */}
            <motion.div
              className="relative z-10 flex flex-col items-center justify-center rounded-full glass-obsidian knurling-pattern shadow-2xl border border-white/10"
              style={{
                width: rInner * 2 - 12,
                height: rInner * 2 - 12,
              }}
            >
              {currentMode === "menu" ? (
                <div className="flex flex-col items-center justify-center text-center px-4">
                  {/* Category Pill */}
                  <div className="flex items-center gap-1.5 px-3 py-0.5 rounded-full bg-white/[0.07] border border-white/10 mb-2">
                    <span className="text-[10px]">{profile.icon}</span>
                    <span className="text-[9px] font-mono tracking-wider uppercase text-neutral-400">
                      {profile.name}
                    </span>
                  </div>

                  {/* Active Action Icon with Micro-Spring */}
                  <motion.div
                    key={activeAction?.id}
                    initial={{ scale: 0.8, y: 4, opacity: 0 }}
                    animate={{ scale: 1, y: 0, opacity: 1 }}
                    transition={{ type: "spring", stiffness: 500, damping: 25 }}
                    className="text-4xl my-1"
                  >
                    {activeAction?.icon}
                  </motion.div>

                  {/* Action Label */}
                  <div className="text-sm font-semibold tracking-wide text-white mt-1">
                    {activeAction?.label}
                  </div>

                  {/* Action Shortcut / Description Pill */}
                  <div className="text-[10px] font-mono text-neutral-400 mt-1 max-w-[140px] truncate">
                    {activeAction?.desc || (activeAction?.keys?.join(" + ") ?? "")}
                  </div>
                </div>
              ) : (
                /* Gauge Center Mode */
                <div className="flex flex-col items-center justify-center text-center">
                  <span className="text-2xl mb-1">
                    {currentMode === "volume" ? "🔊" : "☀️"}
                  </span>
                  <motion.div
                    key={gaugeValue}
                    initial={{ scale: 0.95 }}
                    animate={{ scale: 1 }}
                    className="text-3xl font-bold font-mono tracking-tight text-white"
                  >
                    {Math.round(gaugeValue)}%
                  </motion.div>
                  <span className="text-[10px] font-mono uppercase text-neutral-400 tracking-widest mt-1">
                    {currentMode} Mode
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
