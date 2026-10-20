import { useState, useEffect, useRef } from "react";
import { DonutRadialHUD } from "./components/hud/DonutRadialHUD";
import { SettingsWindow } from "./components/settings/SettingsWindow";
import { DEFAULT_CONFIG } from "./data/defaultProfiles";
import { ProfilesConfig, WheelAction } from "./types/profile";
import { Disc, Layers, Sliders, Eye, EyeOff } from "lucide-react";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

const CURRENT_CONFIG_KEY = "dial_config_v3";

export function App() {
  const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

  const [config, setConfig] = useState<ProfilesConfig>(() => {
    const saved = localStorage.getItem(CURRENT_CONFIG_KEY);
    if (saved) {
      try {
        return JSON.parse(saved);
      } catch (e) {
        return DEFAULT_CONFIG;
      }
    }
    return DEFAULT_CONFIG;
  });

  const [view, setView] = useState<"hud" | "settings">(() => {
    const params = new URLSearchParams(window.location.search);
    return params.get("view") === "settings" ? "settings" : "hud";
  });

  const [activeProfileId, setActiveProfileId] = useState<string>("default");
  const [selectedIndex, setSelectedIndex] = useState<number>(0);
  const [isHudOpen, setIsHudOpen] = useState<boolean>(true);
  const [currentMode, setCurrentMode] = useState<"menu" | "volume" | "brightness">("menu");
  const [gaugeValue, setGaugeValue] = useState<number>(50);

  const activeProfile = config.profiles[activeProfileId] || config.profiles.default;
  const actionsCount = activeProfile.actions.length;

  const handleSaveConfig = (updated: ProfilesConfig) => {
    setConfig(updated);
    localStorage.setItem(CURRENT_CONFIG_KEY, JSON.stringify(updated));
  };

  // Keep references for event listener callbacks without stale closures
  const stateRef = useRef({
    currentMode,
    selectedIndex,
    actionsCount,
    activeProfile,
    isHudOpen,
    gaugeValue,
  });
  useEffect(() => {
    stateRef.current = {
      currentMode,
      selectedIndex,
      actionsCount,
      activeProfile,
      isHudOpen,
      gaugeValue,
    };
  }, [currentMode, selectedIndex, actionsCount, activeProfile, isHudOpen, gaugeValue]);

  // Execute an action (either mode switch or native system command/hotkey)
  const handleExecuteAction = (action: WheelAction) => {
    if (action.type === "mode") {
      const mode = action.id === "volume_mode" ? "volume" : "brightness";
      setCurrentMode(mode);
      if (isTauri) {
        invoke("set_mode", { mode }).catch(console.error);
        if (mode === "brightness") {
          invoke<number>("get_system_brightness")
            .then((b) => setGaugeValue(b))
            .catch(console.error);
        } else {
          invoke<number>("get_system_volume")
            .then((v) => setGaugeValue(v))
            .catch(console.error);
        }
      }
    } else {
      if (isTauri) {
        invoke("execute_action", { action }).catch(console.error);
        invoke("hide_hud").catch(console.error);
      }
      setIsHudOpen(false);
    }
  };

  // Tauri Event Listeners (Hardware Dial & Context Detection)
  useEffect(() => {
    if (!isTauri) return;

    let unlistenList: UnlistenFn[] = [];

    async function setupTauriListeners() {
      // 1. Dial Rotation
      const unlistenRotate = await listen<{ direction: number; delta: number; volume?: number; volume_pct?: number }>("dial:rotate", (event) => {
        const { currentMode, actionsCount, gaugeValue } = stateRef.current;
        const dir = event.payload.direction;

        if (currentMode === "menu") {
          setSelectedIndex((prev) => {
            const next = prev + dir;
            if (next < 0) return actionsCount - 1;
            return next % actionsCount;
          });
        } else if (currentMode === "volume") {
          // Volume mapping: CMF AVRCP 15..79 maps directly to 0..100%
          if (typeof event.payload.volume_pct === "number") {
            setGaugeValue(event.payload.volume_pct);
          } else {
            const next = Math.max(0, Math.min(100, gaugeValue + dir * 5));
            setGaugeValue(next);
            if (isTauri) {
              invoke("set_system_volume", { pct: next }).catch(console.error);
            }
          }
        } else if (currentMode === "brightness") {
          // Step by 5% to cleanly match GNOME's 20-step increments
          const step = dir * 5;
          setGaugeValue((prev) => {
            const next = Math.max(0, Math.min(100, prev + step));
            if (isTauri) {
              invoke("set_system_brightness", { pct: next }).catch(console.error);
            }
            return next;
          });
        }
      });
      unlistenList.push(unlistenRotate);

      // 2. Dial Center Click (Triggered when HUD is already open)
      const unlistenClick = await listen("dial:click", () => {
        const { currentMode, selectedIndex, activeProfile } = stateRef.current;

        // When in active volume/brightness adjustment mode, click confirms and dismisses HUD
        if (currentMode !== "menu") {
          setCurrentMode("menu");
          setIsHudOpen(false);
          if (isTauri) {
            invoke("set_mode", { mode: "menu" }).catch(console.error);
            invoke("hide_hud").catch(console.error);
          }
          return;
        }

        const action = activeProfile.actions[selectedIndex];
        if (action) {
          handleExecuteAction(action);
        }
      });
      unlistenList.push(unlistenClick);

      // 3. Context Aware Profile Change
      const unlistenContext = await listen<{ profile_id: string }>("context:change", (event) => {
        if (config.profiles[event.payload.profile_id]) {
          setActiveProfileId(event.payload.profile_id);
          setSelectedIndex(0);
          setCurrentMode("menu");
        }
      });
      unlistenList.push(unlistenContext);

      // 4. HUD Visibility Sync
      const unlistenVis = await listen<boolean>("hud:visibility", (event) => {
        setIsHudOpen(event.payload);
        if (event.payload) {
          setCurrentMode("menu");
          if (isTauri) {
            invoke("set_mode", { mode: "menu" }).catch(console.error);
            // Pre-fetch actual system levels so gauges are instantly accurate
            invoke<number>("get_system_brightness")
              .then((b) => setGaugeValue(b))
              .catch(console.error);
          }
        }
      });
      unlistenList.push(unlistenVis);
    }

    setupTauriListeners().catch(console.error);

    return () => {
      unlistenList.forEach((unsub) => unsub());
    };
  }, [isTauri, config.profiles]);

  // Keyboard navigation & mouse wheel (for Desktop browser preview or fallback)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (isTauri) {
          invoke("set_mode", { mode: "menu" }).catch(console.error);
          invoke("hide_hud").catch(console.error);
        }
        setCurrentMode("menu");
        setIsHudOpen(false);
      } else if (e.key === " " && view === "hud") {
        e.preventDefault();
        setIsHudOpen((prev) => !prev);
      } else if (e.key === "ArrowRight" || e.key === "ArrowDown") {
        e.preventDefault();
        if (currentMode === "menu") {
          setSelectedIndex((prev) => (prev + 1) % actionsCount);
        } else {
          setGaugeValue((prev) => {
            const next = Math.min(100, prev + 5);
            if (isTauri) {
              if (currentMode === "brightness") {
                invoke("set_system_brightness", { pct: next }).catch(console.error);
              } else if (currentMode === "volume") {
                invoke("set_system_volume", { pct: next }).catch(console.error);
              }
            }
            return next;
          });
        }
      } else if (e.key === "ArrowLeft" || e.key === "ArrowUp") {
        e.preventDefault();
        if (currentMode === "menu") {
          setSelectedIndex((prev) => (prev - 1 + actionsCount) % actionsCount);
        } else {
          setGaugeValue((prev) => {
            const next = Math.max(0, prev - 5);
            if (isTauri) {
              if (currentMode === "brightness") {
                invoke("set_system_brightness", { pct: next }).catch(console.error);
              } else if (currentMode === "volume") {
                invoke("set_system_volume", { pct: next }).catch(console.error);
              }
            }
            return next;
          });
        }
      } else if (e.key === "Enter") {
        e.preventDefault();
        const { currentMode, selectedIndex, activeProfile } = stateRef.current;
        if (currentMode !== "menu") {
          if (isTauri) {
            invoke("set_mode", { mode: "menu" }).catch(console.error);
            invoke("hide_hud").catch(console.error);
          }
          setCurrentMode("menu");
          setIsHudOpen(false);
          return;
        }
        const action = activeProfile.actions[selectedIndex];
        if (action) {
          handleExecuteAction(action);
        }
      }
    };

    const handleWheel = (e: WheelEvent) => {
      if (!isHudOpen) return;
      if (e.deltaY > 0) {
        if (currentMode === "menu") {
          setSelectedIndex((prev) => (prev + 1) % actionsCount);
        } else {
          setGaugeValue((prev) => Math.min(100, prev + 3));
        }
      } else {
        if (currentMode === "menu") {
          setSelectedIndex((prev) => (prev - 1 + actionsCount) % actionsCount);
        } else {
          setGaugeValue((prev) => Math.max(0, prev - 3));
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("wheel", handleWheel);
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("wheel", handleWheel);
    };
  }, [view, isHudOpen, currentMode, actionsCount, selectedIndex, activeProfile, isTauri]);

  return (
    <div className="relative w-screen h-screen overflow-hidden bg-transparent select-none">
      {/* ── Top Industrial Toolbar (shown only in browser preview mode) ─── */}
      {!isTauri && (
        <div className="fixed top-3 left-1/2 -translate-x-1/2 z-50 flex items-center gap-2 p-1.5 rounded-full bg-[#0E1014]/90 backdrop-blur-xl border border-white/10 shadow-2xl">
          <button
            onClick={() => setView("hud")}
            className={`flex items-center gap-1.5 px-3.5 py-1.5 rounded-full transition-all text-xs font-mono uppercase tracking-wider ${
              view === "hud"
                ? "bg-white text-black font-bold shadow-md"
                : "text-neutral-400 hover:text-white"
            }`}
          >
            <Disc className="w-3.5 h-3.5" />
            <span>Donut HUD</span>
          </button>

          <button
            onClick={() => setView("settings")}
            className={`flex items-center gap-1.5 px-3.5 py-1.5 rounded-full transition-all text-xs font-mono uppercase tracking-wider ${
              view === "settings"
                ? "bg-white text-black font-bold shadow-md"
                : "text-neutral-400 hover:text-white"
            }`}
          >
            <Sliders className="w-3.5 h-3.5" />
            <span>Settings</span>
          </button>

          {view === "hud" && (
            <div className="flex items-center gap-2 pl-2 border-l border-white/10">
              <div className="flex items-center gap-1.5 bg-[#14161C] px-2.5 py-1 rounded-full border border-white/10">
                <Layers className="w-3 h-3 text-neutral-400" />
                <select
                  value={activeProfileId}
                  onChange={(e) => {
                    setActiveProfileId(e.target.value);
                    setSelectedIndex(0);
                    setCurrentMode("menu");
                  }}
                  className="bg-transparent text-white text-[11px] font-ndot tracking-wider focus:outline-none uppercase cursor-pointer"
                >
                  {Object.entries(config.profiles).map(([id, p]) => (
                    <option key={id} value={id} className="bg-[#0E1014] text-white">
                      {p.name}
                    </option>
                  ))}
                </select>
              </div>

              <button
                onClick={() => setIsHudOpen((prev) => !prev)}
                className="flex items-center gap-1 px-2.5 py-1 rounded-full bg-white/[0.08] hover:bg-white/15 text-neutral-300 text-[11px] font-mono uppercase tracking-wider"
                title="Toggle HUD (Space)"
              >
                {isHudOpen ? <EyeOff className="w-3 h-3" /> : <Eye className="w-3 h-3" />}
                <span>{isHudOpen ? "HIDE" : "SHOW"}</span>
              </button>
            </div>
          )}
        </div>
      )}

      {/* ── View Router ───────────────────────────────────────────────────── */}
      {view === "hud" ? (
        <DonutRadialHUD
          profile={activeProfile}
          selectedIndex={selectedIndex}
          isOpen={isHudOpen}
          currentMode={currentMode}
          gaugeValue={gaugeValue}
          theme={config.settings.theme}
          onSelectAction={handleExecuteAction}
          onDismiss={() => {
            if (isTauri) {
              invoke("set_mode", { mode: "menu" }).catch(console.error);
              invoke("hide_hud").catch(console.error);
            }
            setCurrentMode("menu");
            setIsHudOpen(false);
          }}
        />
      ) : (
        <SettingsWindow config={config} onSaveConfig={handleSaveConfig} />
      )}
    </div>
  );
}

export default App;
