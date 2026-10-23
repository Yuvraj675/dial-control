import { useState, useEffect, useRef } from "react";
import { DonutRadialHUD } from "./components/hud/DonutRadialHUD";
import { ScrollPillIndicator } from "./components/hud/ScrollPillIndicator";
import { SettingsWindow } from "./components/settings/SettingsWindow";
import { DEFAULT_CONFIG, sanitizeAndMigrateConfig } from "./data/defaultProfiles";
import { ProfilesConfig, WheelAction, DialProfile, getOrderedProfileIds } from "./types/profile";
import { Disc, Layers, Sliders, Eye, EyeOff, Mouse } from "lucide-react";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

const CURRENT_CONFIG_KEY = "dial_config_v5";

export function App() {
  const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

  const [config, setConfig] = useState<ProfilesConfig>(() => {
    let cfg: ProfilesConfig = DEFAULT_CONFIG;
    const saved = localStorage.getItem(CURRENT_CONFIG_KEY);
    if (saved) {
      try {
        cfg = JSON.parse(saved);
      } catch (e) {
        cfg = DEFAULT_CONFIG;
      }
    }
    return sanitizeAndMigrateConfig(cfg);
  });

  const [view, setView] = useState<"hud" | "settings" | "scroll_pill">(() => {
    const params = new URLSearchParams(window.location.search);
    const v = params.get("view");
    if (v === "settings") return "settings";
    if (v === "scroll_pill") return "scroll_pill";
    return "hud";
  });

  const [activeProfileId, setActiveProfileId] = useState<string>("default");
  const [selectedIndex, setSelectedIndex] = useState<number>(0);
  const [isHudOpen, setIsHudOpen] = useState<boolean>(true);
  const [currentMode, setCurrentMode] = useState<"menu" | "volume" | "brightness" | "scroll">("menu");
  const [gaugeValue, setGaugeValue] = useState<number>(50);
  const [brightnessLevel, setBrightnessLevel] = useState<number>(() => {
    const saved = localStorage.getItem("dial_brightness_level");
    return saved !== null ? Number(saved) : 55;
  });

  // Dynamic Home Dial profile that lists all available profiles in user-defined order
  const activeProfile: DialProfile = (() => {
    if (activeProfileId === "home") {
      const orderedIds = getOrderedProfileIds(config).filter((id) => id !== "home");
      const actions: WheelAction[] = orderedIds
        .map((id) => {
          const prof = config.profiles[id];
          if (!prof) return null;
          return {
            id: `profile_${id}`,
            label: prof.name.toUpperCase(),
            icon: prof.icon || "sliders",
            type: "profile" as const,
            action: id,
            desc: `Switch to ${prof.name} dial`,
          };
        })
        .filter(Boolean) as WheelAction[];

      actions.push({
        id: "close_hud",
        label: "CLOSE",
        icon: "close",
        type: "close" as const,
        desc: "Dismiss HUD",
      });
      return {
        id: "home",
        name: "Home",
        icon: "home",
        match_patterns: [],
        actions,
      };
    }
    return config.profiles[activeProfileId] || config.profiles.default;
  })();

  const actionsCount = activeProfile.actions.length;

  const handleSaveConfig = (updated: ProfilesConfig) => {
    setConfig(updated);
    localStorage.setItem(CURRENT_CONFIG_KEY, JSON.stringify(updated));
    if (isTauri) {
      invoke("save_config", { config: updated }).catch((e) =>
        console.error("[Config] Failed to persist config to backend:", e)
      );
    }
  };

  // Load persistent config from backend on startup & listen for live broadcasts
  useEffect(() => {
    if (!isTauri) return;

    invoke<ProfilesConfig | null>("load_config")
      .then((loaded) => {
        if (loaded && loaded.profiles && loaded.settings) {
          console.log("[Config] Loaded persistent config from backend storage");
          const sanitized = sanitizeAndMigrateConfig(loaded);
          setConfig(sanitized);
          localStorage.setItem(CURRENT_CONFIG_KEY, JSON.stringify(sanitized));
          invoke("save_config", { config: sanitized }).catch((e) =>
            console.error("[Config] Failed to persist migrated config to backend:", e)
          );
        }
      })
      .catch((e) => console.error("[Config] Failed to load config from backend:", e));

    let unlisten: UnlistenFn | undefined;
    listen<ProfilesConfig>("config:update", (event) => {
      console.log("[Config] Live config update broadcast received:", event.payload);
      if (event.payload && event.payload.profiles) {
        const sanitized = sanitizeAndMigrateConfig(event.payload);
        setConfig(sanitized);
        localStorage.setItem(CURRENT_CONFIG_KEY, JSON.stringify(sanitized));
      }
    }).then((unsub) => {
      unlisten = unsub;
    });

    return () => {
      if (unlisten) unlisten();
    };
  }, [isTauri]);

  // Register activation shortcut on startup if running under Tauri
  useEffect(() => {
    if (isTauri && config.settings?.activation_shortcut) {
      invoke("register_activation_shortcut", {
        shortcutStr: config.settings.activation_shortcut,
      }).catch((e) => console.error("[Shortcut] Startup registration failed:", e));
    }
  }, [isTauri, config.settings?.activation_shortcut]);

  // Keep references for event listener callbacks without stale closures
  const stateRef = useRef({
    currentMode,
    selectedIndex,
    actionsCount,
    activeProfile,
    isHudOpen,
    gaugeValue,
    config,
  });
  useEffect(() => {
    stateRef.current = {
      currentMode,
      selectedIndex,
      actionsCount,
      activeProfile,
      isHudOpen,
      gaugeValue,
      config,
    };
  }, [currentMode, selectedIndex, actionsCount, activeProfile, isHudOpen, gaugeValue, config]);

  // Execute an action (either mode switch, profile switch, home dial, or native system command/hotkey)
  const handleExecuteAction = (action: WheelAction) => {
    // 1. Home Dial Selector
    if (action.type === "home" || action.id === "home_dial") {
      setActiveProfileId("home");
      setSelectedIndex(0);
      setCurrentMode("menu");
      return;
    }

    // 2. Switching to a chosen profile from the Home Dial
    if (action.type === "profile" && action.action) {
      if (config.profiles[action.action]) {
        setActiveProfileId(action.action);
        setSelectedIndex(0);
        setCurrentMode("menu");
      }
      return;
    }

    // 3. Close / Dismiss HUD
    if (action.type === "close" || action.id === "close_hud") {
      setCurrentMode("menu");
      setIsHudOpen(false);
      if (isTauri) {
        invoke("hide_hud").catch(console.error);
      }
      return;
    }

    // 4. Continuous Mode switches (Volume / Brightness / Scroll)
    if (action.type === "mode") {
      const isScroll =
        action.action === "scroll" ||
        action.id === "scroll_mode" ||
        action.id?.includes("scroll") ||
        action.label?.toUpperCase().includes("SCROLL");
      const isVolume =
        action.action === "volume" ||
        action.id === "volume_mode" ||
        action.id.startsWith("vol_") ||
        action.label?.toUpperCase().includes("VOL");

      if (isScroll) {
        setCurrentMode("scroll");
        if (isTauri) {
          invoke("set_mode", { mode: "scroll" }).catch(console.error);
        }
        // Show "ROTATE TO SCROLL" briefly, then hide the HUD window so the underlying app receives mouse wheel scrolls
        setTimeout(() => {
          if (stateRef.current.currentMode === "scroll") {
            setIsHudOpen(false);
            if (isTauri) {
              invoke("close_hud_for_scroll").catch(console.error);
            }
          }
        }, 850);
        return;
      }

      const mode = isVolume ? "volume" : "brightness";
      setCurrentMode(mode);
      if (isTauri) {
        invoke("set_mode", { mode }).catch(console.error);
        if (mode === "brightness") {
          invoke<number>("get_system_brightness")
            .then((b) => {
              console.log("[Brightness] Loaded initial system brightness:", b);
              if (typeof b === "number" && !isNaN(b)) {
                setBrightnessLevel(b);
                setGaugeValue(b);
                localStorage.setItem("dial_brightness_level", String(b));
              } else {
                setGaugeValue(brightnessLevel);
              }
            })
            .catch(() => setGaugeValue(brightnessLevel));
        } else {
          invoke<number>("get_system_volume")
            .then((v) => {
              if (typeof v === "number" && !isNaN(v)) {
                setGaugeValue(v);
              }
            })
            .catch(console.error);
        }
      } else {
        if (mode === "brightness") {
          setGaugeValue(brightnessLevel);
        }
      }
    } else {
      if (isTauri) {
        invoke("execute_action", { action }).catch(console.error);
      }
      setIsHudOpen(false);
    }
  };

  const lastDialClickRef = useRef<number>(0);

  // Tauri Event Listeners (Hardware Dial & Context Detection) - Strictly HUD window
  useEffect(() => {
    if (!isTauri || view !== "hud") return;

    let unlistenList: UnlistenFn[] = [];

    async function setupTauriListeners() {
      // 1. Dial Rotation
      const unlistenRotate = await listen<{ direction: number; delta: number; volume?: number; volume_pct?: number }>("dial:rotate", (event) => {
        const { currentMode, actionsCount, isHudOpen } = stateRef.current;
        if (!isHudOpen) return;
        const dir = event.payload.direction ?? event.payload.delta ?? 1;

        if (currentMode === "menu") {
          setSelectedIndex((prev) => {
            const next = prev + dir;
            if (next < 0) return actionsCount - 1;
            return next % actionsCount;
          });
        } else if (currentMode === "volume") {
          // In volume mode, PipeWire/BlueZ on Linux or Core Audio on Windows update hardware volume naturally.
          const vol = event.payload.volume_pct ?? event.payload.volume;
          if (vol !== undefined) {
            setGaugeValue(vol);
          }
        } else if (currentMode === "brightness") {
          // Emulate hardware Fn+F5 / Fn+F6 keys via uinput (5% per step matching GNOME's native scale)
          const step = dir * 5;
          setBrightnessLevel((prev) => {
            const next = Math.max(0, Math.min(100, prev + step));
            setGaugeValue(next);
            localStorage.setItem("dial_brightness_level", String(next));
            return next;
          });

          if (isTauri) {
            invoke("step_brightness", { direction: dir }).catch(console.error);
          }
        }
      });
      unlistenList.push(unlistenRotate);

      // 2. Dial Center Click (Triggered when HUD is already open)
      const unlistenClick = await listen("dial:click", () => {
        const now = Date.now();
        if (now - lastDialClickRef.current < 500) {
          return; // Ignore hardware bounce / duplicate events
        }
        lastDialClickRef.current = now;

        const { currentMode, selectedIndex, activeProfile } = stateRef.current;

        // When in active volume/brightness adjustment mode or scroll mode, click confirms and dismisses HUD / exits scroll
        if (currentMode !== "menu") {
          setCurrentMode("menu");
          setIsHudOpen(false);
          if (isTauri) {
            if (currentMode === "scroll") {
              invoke("stop_scroll_mode").catch(console.error);
            } else {
              invoke("set_mode", { mode: "menu" }).catch(console.error);
              invoke("hide_hud").catch(console.error);
            }
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
      const unlistenContext = await listen<{
        profile_id: string;
        app?: string;
        title?: string;
        wm_class?: string;
      }>("context:change", (event) => {
        const { profile_id, app, title, wm_class } = event.payload;
        console.log(`[Context] Foreground -> App: '${app}', Title: '${title}', Backend matched: '${profile_id}'`);

        const profiles = stateRef.current.config.profiles;
        const query = `${wm_class || ""} ${app || ""} ${title || ""}`.toLowerCase();
        let targetId = profile_id;

        // Check ordered profiles for custom trigger matches
        const orderedProfileIds = getOrderedProfileIds(stateRef.current.config);
        for (const id of orderedProfileIds) {
          if (id === "default" || id === "home") continue;
          const prof = profiles[id];
          if (
            prof?.match_patterns?.some((pat) => {
              const clean = pat.replace(/\*/g, "").toLowerCase().trim();
              return clean.length > 0 && query.includes(clean);
            })
          ) {
            targetId = id;
            break;
          }
        }

        if (targetId && profiles[targetId]) {
          setActiveProfileId(targetId);
          setSelectedIndex(0);
          setCurrentMode("menu");
        } else if (profiles.default) {
          setActiveProfileId("default");
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
          setSelectedIndex(0);
          window.focus();
          if (isTauri) {
            invoke("set_mode", { mode: "menu" }).catch(console.error);

            // Pre-fetch live system volume so dial gauge never snaps
            invoke<number>("get_system_volume")
              .then((v) => {
                if (typeof v === "number" && !isNaN(v)) {
                  setGaugeValue(v);
                }
              })
              .catch(console.error);

            // Pre-fetch actual system brightness
            invoke<number>("get_system_brightness")
              .then((b) => {
                if (typeof b === "number" && !isNaN(b)) {
                  setBrightnessLevel(b);
                  localStorage.setItem("dial_brightness_level", String(b));
                }
              })
              .catch(console.error);
          }
        }
      });
      unlistenList.push(unlistenVis);

      // 5. Backend State Synchronization (Emitted whenever HUD is shown or state re-synced)
      const unlistenSync = await listen<{ volume?: number; brightness?: number }>("sync:state", (event) => {
        const { volume, brightness } = event.payload;
        if (typeof volume === "number" && !isNaN(volume)) {
          setGaugeValue(volume);
        }
        if (typeof brightness === "number" && !isNaN(brightness)) {
          setBrightnessLevel(brightness);
          localStorage.setItem("dial_brightness_level", String(brightness));
        }
      });
      unlistenList.push(unlistenSync);

      // 6. Scroll Mode Exit broadcast from backend
      const unlistenScrollExit = await listen("scroll_mode:exit", () => {
        setCurrentMode("menu");
        setIsHudOpen(false);
      });
      unlistenList.push(unlistenScrollExit);
    }

    setupTauriListeners().catch(console.error);

    return () => {
      unlistenList.forEach((unsub) => unsub());
    };
  }, [isTauri, view]);

  // Keyboard navigation & mouse wheel (strictly for Donut HUD window)
  useEffect(() => {
    if (view !== "hud") return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (!stateRef.current.isHudOpen) return;
      if (e.key === "Escape") {
        if (isTauri) {
          if (currentMode === "scroll") {
            invoke("stop_scroll_mode").catch(console.error);
          } else {
            invoke("set_mode", { mode: "menu" }).catch(console.error);
            invoke("hide_hud").catch(console.error);
          }
        }
        setCurrentMode("menu");
        setIsHudOpen(false);
      } else if (e.key === "ArrowRight" || e.key === "ArrowDown") {
        e.preventDefault();
        if (currentMode === "menu") {
          setSelectedIndex((prev) => (prev + 1) % actionsCount);
        } else if (currentMode === "scroll") {
          if (isTauri) {
            invoke("adjust_gauge", { mode: "scroll", delta: 1 }).catch(console.error);
          }
        } else if (currentMode === "brightness") {
          setBrightnessLevel((prev) => {
            const next = Math.min(100, prev + 5);
            setGaugeValue(next);
            localStorage.setItem("dial_brightness_level", String(next));
            return next;
          });
          if (isTauri) {
            invoke("step_brightness", { direction: 1 }).catch(console.error);
          }
        } else if (currentMode === "volume") {
          setGaugeValue((prev) => {
            const next = Math.min(100, prev + 5);
            if (isTauri) {
              invoke("set_system_volume", { pct: next }).catch(console.error);
            }
            return next;
          });
        }
      } else if (e.key === "ArrowLeft" || e.key === "ArrowUp") {
        e.preventDefault();
        if (currentMode === "menu") {
          setSelectedIndex((prev) => (prev - 1 + actionsCount) % actionsCount);
        } else if (currentMode === "scroll") {
          if (isTauri) {
            invoke("adjust_gauge", { mode: "scroll", delta: -1 }).catch(console.error);
          }
        } else if (currentMode === "brightness") {
          setBrightnessLevel((prev) => {
            const next = Math.max(0, prev - 5);
            setGaugeValue(next);
            localStorage.setItem("dial_brightness_level", String(next));
            return next;
          });
          if (isTauri) {
            invoke("step_brightness", { direction: -1 }).catch(console.error);
          }
        } else if (currentMode === "volume") {
          setGaugeValue((prev) => {
            const next = Math.max(0, prev - 5);
            if (isTauri) {
              invoke("set_system_volume", { pct: next }).catch(console.error);
            }
            return next;
          });
        }
      } else if (e.key === "Enter" || e.key === " " || e.code === "Space") {
        e.preventDefault();
        const { currentMode, selectedIndex, activeProfile } = stateRef.current;
        if (currentMode !== "menu") {
          if (isTauri) {
            if (currentMode === "scroll") {
              invoke("stop_scroll_mode").catch(console.error);
            } else {
              invoke("set_mode", { mode: "menu" }).catch(console.error);
              invoke("hide_hud").catch(console.error);
            }
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
      if (currentMode === "scroll") {
        if (isTauri) {
          invoke("adjust_gauge", { mode: "scroll", delta: e.deltaY > 0 ? 1 : -1 }).catch(console.error);
        }
        return;
      }
      if (e.deltaY > 0) {
        if (currentMode === "menu") {
          setSelectedIndex((prev) => (prev + 1) % actionsCount);
        } else if (currentMode === "brightness") {
          setBrightnessLevel((prev) => {
            const next = Math.min(100, prev + 5);
            setGaugeValue(next);
            localStorage.setItem("dial_brightness_level", String(next));
            return next;
          });
          if (isTauri) invoke("step_brightness", { direction: 1 }).catch(console.error);
        } else {
          setGaugeValue((prev) => Math.min(100, prev + 3));
        }
      } else {
        if (currentMode === "menu") {
          setSelectedIndex((prev) => (prev - 1 + actionsCount) % actionsCount);
        } else if (currentMode === "brightness") {
          setBrightnessLevel((prev) => {
            const next = Math.max(0, prev - 5);
            setGaugeValue(next);
            localStorage.setItem("dial_brightness_level", String(next));
            return next;
          });
          if (isTauri) invoke("step_brightness", { direction: -1 }).catch(console.error);
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

          <button
            onClick={() => setView("scroll_pill")}
            className={`flex items-center gap-1.5 px-3.5 py-1.5 rounded-full transition-all text-xs font-mono uppercase tracking-wider ${
              view === "scroll_pill"
                ? "bg-white text-black font-bold shadow-md"
                : "text-neutral-400 hover:text-white"
            }`}
          >
            <Mouse className="w-3.5 h-3.5" />
            <span>Scroll Pill</span>
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
                  {getOrderedProfileIds(config).map((id) => {
                    const p = config.profiles[id];
                    if (!p) return null;
                    return (
                      <option key={id} value={id} className="bg-[#0E1014] text-white">
                        {p.name}
                      </option>
                    );
                  })}
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
          dialRadius={config.settings.dial_radius}
          donutThickness={config.settings.donut_thickness}
          onSelectAction={handleExecuteAction}
          onDismiss={() => {
            if (isTauri) {
              if (currentMode === "scroll") {
                invoke("stop_scroll_mode").catch(console.error);
              } else {
                invoke("set_mode", { mode: "menu" }).catch(console.error);
                invoke("hide_hud").catch(console.error);
              }
            }
            setCurrentMode("menu");
            setIsHudOpen(false);
          }}
        />
      ) : view === "scroll_pill" ? (
        <ScrollPillIndicator
          theme={config.settings.theme}
          onDismiss={() => {
            if (isTauri) {
              invoke("stop_scroll_mode").catch(console.error);
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
