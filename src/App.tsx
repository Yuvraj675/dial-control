import { useState, useEffect } from "react";
import { DonutRadialHUD } from "./components/hud/DonutRadialHUD";
import { SettingsWindow } from "./components/settings/SettingsWindow";
import { DEFAULT_CONFIG } from "./data/defaultProfiles";
import { ProfilesConfig, WheelAction } from "./types/profile";

export function App() {
  const [config, setConfig] = useState<ProfilesConfig>(() => {
    const saved = localStorage.getItem("dial_config");
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
  const [gaugeValue, setGaugeValue] = useState<number>(68);

  const activeProfile = config.profiles[activeProfileId] || config.profiles.default;
  const actionsCount = activeProfile.actions.length;

  const handleSaveConfig = (updated: ProfilesConfig) => {
    setConfig(updated);
    localStorage.setItem("dial_config", JSON.stringify(updated));
  };

  // Keyboard controls for testing & navigation
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setIsHudOpen(false);
      } else if (e.key === " " && view === "hud") {
        e.preventDefault();
        setIsHudOpen((prev) => !prev);
      } else if (e.key === "ArrowRight" || e.key === "ArrowDown") {
        e.preventDefault();
        if (currentMode === "menu") {
          setSelectedIndex((prev) => (prev + 1) % actionsCount);
        } else {
          setGaugeValue((prev) => Math.min(100, prev + 5));
        }
      } else if (e.key === "ArrowLeft" || e.key === "ArrowUp") {
        e.preventDefault();
        if (currentMode === "menu") {
          setSelectedIndex((prev) => (prev - 1 + actionsCount) % actionsCount);
        } else {
          setGaugeValue((prev) => Math.max(0, prev - 5));
        }
      } else if (e.key === "Enter") {
        e.preventDefault();
        const action = activeProfile.actions[selectedIndex];
        if (action?.type === "mode") {
          setCurrentMode(action.id === "volume_mode" ? "volume" : "brightness");
        } else if (currentMode !== "menu") {
          setCurrentMode("menu");
        } else {
          setIsHudOpen(false);
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
  }, [view, isHudOpen, currentMode, actionsCount, selectedIndex, activeProfile]);

  return (
    <div className="relative w-screen h-screen overflow-hidden bg-transparent">
      {/* Top Testing Switcher Toolbar (hidden in production frameless mode) */}
      <div className="fixed top-3 left-1/2 -translate-x-1/2 z-50 flex items-center gap-2 p-1.5 rounded-full bg-[#14161B]/80 backdrop-blur-md border border-white/10 shadow-lg text-xs">
        <button
          onClick={() => setView("hud")}
          className={`px-3 py-1 rounded-full transition-all ${
            view === "hud" ? "bg-cmf-orange text-black font-bold" : "text-neutral-400 hover:text-white"
          }`}
        >
          Radial Donut HUD
        </button>
        <button
          onClick={() => setView("settings")}
          className={`px-3 py-1 rounded-full transition-all ${
            view === "settings" ? "bg-cmf-orange text-black font-bold" : "text-neutral-400 hover:text-white"
          }`}
        >
          Settings & Customizer
        </button>

        {view === "hud" && (
          <div className="flex items-center gap-1.5 pl-2 border-l border-white/10">
            <select
              value={activeProfileId}
              onChange={(e) => {
                setActiveProfileId(e.target.value);
                setSelectedIndex(0);
                setCurrentMode("menu");
              }}
              className="bg-[#0B0C0E] text-white px-2 py-0.5 rounded text-[11px] border border-white/10 focus:outline-none"
            >
              {Object.entries(config.profiles).map(([id, p]) => (
                <option key={id} value={id}>
                  {p.icon} {p.name}
                </option>
              ))}
            </select>
            <button
              onClick={() => setIsHudOpen((prev) => !prev)}
              className="px-2 py-0.5 rounded bg-white/10 hover:bg-white/20 text-neutral-300 text-[11px]"
            >
              {isHudOpen ? "Hide" : "Show"}
            </button>
          </div>
        )}
      </div>

      {view === "hud" ? (
        <DonutRadialHUD
          profile={activeProfile}
          selectedIndex={selectedIndex}
          isOpen={isHudOpen}
          currentMode={currentMode}
          gaugeValue={gaugeValue}
          theme={config.settings.theme}
          onSelectAction={(action: WheelAction) => {
            if (action.type === "mode") {
              setCurrentMode(action.id === "volume_mode" ? "volume" : "brightness");
            } else {
              setIsHudOpen(false);
            }
          }}
          onClose={() => setIsHudOpen(false)}
        />
      ) : (
        <SettingsWindow
          config={config}
          onSaveConfig={handleSaveConfig}
          onClose={() => setView("hud")}
        />
      )}
    </div>
  );
}
export default App;
