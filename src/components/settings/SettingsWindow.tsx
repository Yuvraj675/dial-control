import React, { useState } from "react";
import { ProfilesConfig, WheelAction, ActionType } from "../../types/profile";
import {
  Sliders,
  Palette,
  Layers,
  ArrowUp,
  ArrowDown,
  Plus,
  Trash2,
  Check,
  Disc,
} from "lucide-react";

interface SettingsWindowProps {
  config: ProfilesConfig;
  onSaveConfig: (updated: ProfilesConfig) => void;
  onClose?: () => void;
}

export const SettingsWindow: React.FC<SettingsWindowProps> = ({
  config,
  onSaveConfig,
}) => {
  const [activeTab, setActiveTab] = useState<"profiles" | "appearance" | "hardware">("profiles");
  const [selectedProfileId, setSelectedProfileId] = useState<string>("default");
  const [selectedActionIndex, setSelectedActionIndex] = useState<number>(0);
  const [currentConfig, setCurrentConfig] = useState<ProfilesConfig>(config);
  const [saveSuccess, setSaveSuccess] = useState(false);

  const activeProfile = currentConfig.profiles[selectedProfileId] || currentConfig.profiles.default;
  const actions = activeProfile.actions || [];
  const selectedAction = actions[selectedActionIndex] || actions[0];

  const handleUpdateAction = (updates: Partial<WheelAction>) => {
    const updatedActions = [...actions];
    if (updatedActions[selectedActionIndex]) {
      updatedActions[selectedActionIndex] = {
        ...updatedActions[selectedActionIndex],
        ...updates,
      };
      const updatedProfile = { ...activeProfile, actions: updatedActions };
      setCurrentConfig({
        ...currentConfig,
        profiles: {
          ...currentConfig.profiles,
          [selectedProfileId]: updatedProfile,
        },
      });
    }
  };

  const handleMoveAction = (direction: -1 | 1) => {
    const newIdx = selectedActionIndex + direction;
    if (newIdx >= 0 && newIdx < actions.length) {
      const updated = [...actions];
      const temp = updated[selectedActionIndex];
      updated[selectedActionIndex] = updated[newIdx];
      updated[newIdx] = temp;

      const updatedProfile = { ...activeProfile, actions: updated };
      setCurrentConfig({
        ...currentConfig,
        profiles: {
          ...currentConfig.profiles,
          [selectedProfileId]: updatedProfile,
        },
      });
      setSelectedActionIndex(newIdx);
    }
  };

  const handleAddAction = () => {
    const newAction: WheelAction = {
      id: `custom_${Date.now()}`,
      label: "New Action",
      icon: "⭐",
      type: "hotkey",
      keys: ["KEY_LEFTCTRL", "KEY_S"],
      desc: "Custom shortcut",
    };
    const updatedActions = [...actions, newAction];
    const updatedProfile = { ...activeProfile, actions: updatedActions };
    setCurrentConfig({
      ...currentConfig,
      profiles: {
        ...currentConfig.profiles,
        [selectedProfileId]: updatedProfile,
      },
    });
    setSelectedActionIndex(updatedActions.length - 1);
  };

  const handleRemoveAction = () => {
    if (actions.length <= 2) return;
    const updatedActions = actions.filter((_, idx) => idx !== selectedActionIndex);
    const updatedProfile = { ...activeProfile, actions: updatedActions };
    setCurrentConfig({
      ...currentConfig,
      profiles: {
        ...currentConfig.profiles,
        [selectedProfileId]: updatedProfile,
      },
    });
    setSelectedActionIndex(Math.max(0, selectedActionIndex - 1));
  };

  const handleSave = () => {
    onSaveConfig(currentConfig);
    setSaveSuccess(true);
    setTimeout(() => setSaveSuccess(false), 2200);
  };

  return (
    <div className="flex h-screen w-screen bg-[#08090A] text-white font-sans overflow-hidden select-none">
      {/* ── Left Sidebar Navigation ────────────────────────────────────────── */}
      <div className="w-56 flex flex-col border-r border-white/[0.08] bg-[#0E1014] p-4">
        {/* App Title with Dot-Matrix Accent */}
        <div className="flex items-center gap-2.5 px-2 py-3 mb-6">
          <div className="w-7 h-7 rounded-full bg-cmf-orange flex items-center justify-center shadow-cmf-glow">
            <Disc className="w-4 h-4 text-black animate-spin-slow" />
          </div>
          <div>
            <div className="text-sm font-bold tracking-wider">DIAL CONTROL</div>
            <div className="text-[10px] font-mono text-neutral-400">CMF INDUSTRIAL</div>
          </div>
        </div>

        {/* Tab Buttons */}
        <div className="flex flex-col gap-1.5 flex-1">
          <button
            onClick={() => setActiveTab("profiles")}
            className={`flex items-center gap-3 px-3.5 py-2.5 rounded-lg text-xs font-medium transition-all ${
              activeTab === "profiles"
                ? "bg-white/[0.1] text-white border border-white/10 shadow-sm"
                : "text-neutral-400 hover:text-white hover:bg-white/[0.04]"
            }`}
          >
            <Layers className="w-4 h-4 text-cmf-orange" />
            <span>Wheel Profiles</span>
          </button>

          <button
            onClick={() => setActiveTab("appearance")}
            className={`flex items-center gap-3 px-3.5 py-2.5 rounded-lg text-xs font-medium transition-all ${
              activeTab === "appearance"
                ? "bg-white/[0.1] text-white border border-white/10 shadow-sm"
                : "text-neutral-400 hover:text-white hover:bg-white/[0.04]"
            }`}
          >
            <Palette className="w-4 h-4 text-cmf-cyan" />
            <span>Theme & Dial</span>
          </button>

          <button
            onClick={() => setActiveTab("hardware")}
            className={`flex items-center gap-3 px-3.5 py-2.5 rounded-lg text-xs font-medium transition-all ${
              activeTab === "hardware"
                ? "bg-white/[0.1] text-white border border-white/10 shadow-sm"
                : "text-neutral-400 hover:text-white hover:bg-white/[0.04]"
            }`}
          >
            <Sliders className="w-4 h-4 text-neutral-300" />
            <span>Haptics & Startup</span>
          </button>
        </div>

        {/* Bottom Save Action */}
        <button
          onClick={handleSave}
          className="flex items-center justify-center gap-2 w-full py-2.5 rounded-lg bg-cmf-orange hover:bg-cmf-orange-hover text-black text-xs font-bold transition-all shadow-cmf-glow"
        >
          {saveSuccess ? (
            <>
              <Check className="w-4 h-4" />
              <span>SAVED & SYNCED</span>
            </>
          ) : (
            <span>APPLY & SAVE</span>
          )}
        </button>
      </div>

      {/* ── Main Content Area ──────────────────────────────────────────────── */}
      <div className="flex-1 flex flex-col bg-[#0B0C0E] overflow-hidden">
        {/* Top Header */}
        <div className="h-14 border-b border-white/[0.08] flex items-center justify-between px-8 bg-[#0E1014]/50 backdrop-blur-md">
          <div className="flex items-center gap-2">
            <span className="text-xs font-mono text-neutral-500 uppercase">Config //</span>
            <span className="text-xs font-semibold tracking-wide uppercase text-white">
              {activeTab === "profiles"
                ? "Context-Aware Profiles"
                : activeTab === "appearance"
                ? "Visual Aesthetics & Dimensions"
                : "Hardware Controls"}
            </span>
          </div>
        </div>

        {/* Tab 1: Profiles & Slices Editor */}
        {activeTab === "profiles" && (
          <div className="flex-1 flex overflow-hidden">
            {/* Column 1: Profiles List */}
            <div className="w-64 border-r border-white/[0.08] p-4 flex flex-col gap-2 overflow-y-auto">
              <div className="text-[10px] font-mono uppercase text-neutral-500 tracking-wider mb-2">
                Active Profiles ({Object.keys(currentConfig.profiles).length})
              </div>
              {Object.entries(currentConfig.profiles).map(([id, prof]) => (
                <button
                  key={id}
                  onClick={() => {
                    setSelectedProfileId(id);
                    setSelectedActionIndex(0);
                  }}
                  className={`flex items-center gap-3 p-3 rounded-xl text-left border transition-all ${
                    selectedProfileId === id
                      ? "bg-white/[0.08] border-cmf-orange text-white shadow-sm"
                      : "bg-[#14161B]/60 border-white/[0.04] text-neutral-400 hover:text-white hover:bg-white/[0.04]"
                  }`}
                >
                  <span className="text-lg">{prof.icon}</span>
                  <div className="flex-1 min-w-0">
                    <div className="text-xs font-semibold truncate">{prof.name}</div>
                    <div className="text-[10px] font-mono text-neutral-500 truncate">
                      {prof.actions.length} slices
                    </div>
                  </div>
                </button>
              ))}
            </div>

            {/* Column 2: Slices Reordering & Slices List */}
            <div className="w-72 border-r border-white/[0.08] p-4 flex flex-col overflow-y-auto">
              <div className="flex items-center justify-between mb-3">
                <div className="text-[10px] font-mono uppercase text-neutral-500 tracking-wider">
                  Wheel Slices (Order)
                </div>
                <div className="flex gap-1">
                  <button
                    onClick={() => handleMoveAction(-1)}
                    className="p-1 rounded bg-white/[0.06] hover:bg-white/10 text-neutral-300"
                    title="Move slice earlier"
                  >
                    <ArrowUp className="w-3.5 h-3.5" />
                  </button>
                  <button
                    onClick={() => handleMoveAction(1)}
                    className="p-1 rounded bg-white/[0.06] hover:bg-white/10 text-neutral-300"
                    title="Move slice later"
                  >
                    <ArrowDown className="w-3.5 h-3.5" />
                  </button>
                  <button
                    onClick={handleAddAction}
                    className="p-1 rounded bg-white/[0.06] hover:bg-white/10 text-cmf-orange"
                    title="Add slice"
                  >
                    <Plus className="w-3.5 h-3.5" />
                  </button>
                  <button
                    onClick={handleRemoveAction}
                    className="p-1 rounded bg-white/[0.06] hover:bg-white/10 text-red-400"
                    title="Delete slice"
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                  </button>
                </div>
              </div>

              <div className="flex flex-col gap-2">
                {actions.map((act, idx) => (
                  <button
                    key={act.id + idx}
                    onClick={() => setSelectedActionIndex(idx)}
                    className={`flex items-center gap-3 p-2.5 rounded-lg border text-left transition-all ${
                      selectedActionIndex === idx
                        ? "bg-white/[0.1] border-white/20 text-white"
                        : "bg-[#14161B]/40 border-white/[0.04] text-neutral-400 hover:text-white"
                    }`}
                  >
                    <span className="text-base">{act.icon}</span>
                    <div className="flex-1 min-w-0">
                      <div className="text-xs font-medium truncate">{act.label}</div>
                      <div className="text-[10px] font-mono text-neutral-500 uppercase">
                        {act.type}
                      </div>
                    </div>
                  </button>
                ))}
              </div>
            </div>

            {/* Column 3: Slice Editor Card */}
            <div className="flex-1 p-6 overflow-y-auto">
              {selectedAction && (
                <div className="max-w-md flex flex-col gap-5">
                  <div className="text-xs font-mono uppercase text-neutral-500 tracking-wider">
                    Slice Inspector
                  </div>

                  {/* Label */}
                  <div>
                    <label className="text-xs text-neutral-400 mb-1.5 block">Label</label>
                    <input
                      type="text"
                      value={selectedAction.label}
                      onChange={(e) => handleUpdateAction({ label: e.target.value })}
                      className="w-full px-3.5 py-2 rounded-lg bg-[#14161B] border border-white/10 text-xs text-white focus:outline-none focus:border-cmf-orange"
                    />
                  </div>

                  {/* Icon */}
                  <div>
                    <label className="text-xs text-neutral-400 mb-1.5 block">
                      Icon (Emoji or Glyph)
                    </label>
                    <input
                      type="text"
                      value={selectedAction.icon}
                      onChange={(e) => handleUpdateAction({ icon: e.target.value })}
                      className="w-24 px-3.5 py-2 rounded-lg bg-[#14161B] border border-white/10 text-xs text-white focus:outline-none focus:border-cmf-orange text-center"
                    />
                  </div>

                  {/* Type */}
                  <div>
                    <label className="text-xs text-neutral-400 mb-1.5 block">Action Type</label>
                    <select
                      value={selectedAction.type}
                      onChange={(e) =>
                        handleUpdateAction({ type: e.target.value as ActionType })
                      }
                      className="w-full px-3.5 py-2 rounded-lg bg-[#14161B] border border-white/10 text-xs text-white focus:outline-none focus:border-cmf-orange"
                    >
                      <option value="hotkey">Hotkey Keypress</option>
                      <option value="mode">Interactive Mode (Volume/Brightness)</option>
                      <option value="key">Media Key (Play/Next/Prev)</option>
                      <option value="command">Shell Command</option>
                      <option value="mute">Mute Audio Toggle</option>
                    </select>
                  </div>

                  {/* Target / Keys */}
                  {selectedAction.type === "hotkey" && (
                    <div>
                      <label className="text-xs text-neutral-400 mb-1.5 block">
                        Keystroke Sequence
                      </label>
                      <input
                        type="text"
                        value={selectedAction.keys?.join(" + ") || ""}
                        onChange={(e) =>
                          handleUpdateAction({
                            keys: e.target.value.split("+").map((s) => s.trim().toUpperCase()),
                          })
                        }
                        placeholder="e.g. KEY_LEFTCTRL + KEY_C"
                        className="w-full px-3.5 py-2 rounded-lg bg-[#14161B] border border-white/10 text-xs text-white font-mono focus:outline-none focus:border-cmf-orange"
                      />
                    </div>
                  )}

                  {/* Description */}
                  <div>
                    <label className="text-xs text-neutral-400 mb-1.5 block">
                      Hint / Description
                    </label>
                    <input
                      type="text"
                      value={selectedAction.desc || ""}
                      onChange={(e) => handleUpdateAction({ desc: e.target.value })}
                      placeholder="e.g. Save active document"
                      className="w-full px-3.5 py-2 rounded-lg bg-[#14161B] border border-white/10 text-xs text-white focus:outline-none focus:border-cmf-orange"
                    />
                  </div>
                </div>
              )}
            </div>
          </div>
        )}

        {/* Tab 2: Appearance & Geometry */}
        {activeTab === "appearance" && (
          <div className="p-8 max-w-lg flex flex-col gap-6">
            <div>
              <div className="text-xs font-mono uppercase text-neutral-500 tracking-wider mb-3">
                Theme Color Accent
              </div>
              <div className="grid grid-cols-3 gap-3">
                {[
                  { id: "cmf_orange", name: "CMF Orange", color: "#FF5500" },
                  { id: "cmf_cyan", name: "Electric Cyan", color: "#00E5FF" },
                  { id: "stealth", name: "Monochrome Stealth", color: "#FFFFFF" },
                ].map((th) => (
                  <button
                    key={th.id}
                    onClick={() =>
                      setCurrentConfig({
                        ...currentConfig,
                        settings: {
                          ...currentConfig.settings,
                          theme: th.id as any,
                        },
                      })
                    }
                    className={`flex items-center gap-2.5 p-3 rounded-xl border text-xs font-medium transition-all ${
                      currentConfig.settings.theme === th.id
                        ? "bg-white/10 border-white/30 text-white"
                        : "bg-[#14161B] border-white/5 text-neutral-400 hover:text-white"
                    }`}
                  >
                    <div
                      className="w-4 h-4 rounded-full"
                      style={{ backgroundColor: th.color }}
                    />
                    <span>{th.name}</span>
                  </button>
                ))}
              </div>
            </div>

            <div>
              <div className="flex justify-between text-xs text-neutral-400 mb-2">
                <span>Outer Dial Radius</span>
                <span className="font-mono">{currentConfig.settings.dial_radius}px</span>
              </div>
              <input
                type="range"
                min="140"
                max="240"
                value={currentConfig.settings.dial_radius}
                onChange={(e) =>
                  setCurrentConfig({
                    ...currentConfig,
                    settings: {
                      ...currentConfig.settings,
                      dial_radius: parseInt(e.target.value),
                    },
                  })
                }
                className="w-full accent-cmf-orange"
              />
            </div>

            <div>
              <div className="flex justify-between text-xs text-neutral-400 mb-2">
                <span>Donut Ring Thickness</span>
                <span className="font-mono">{currentConfig.settings.donut_thickness}px</span>
              </div>
              <input
                type="range"
                min="40"
                max="80"
                value={currentConfig.settings.donut_thickness}
                onChange={(e) =>
                  setCurrentConfig({
                    ...currentConfig,
                    settings: {
                      ...currentConfig.settings,
                      donut_thickness: parseInt(e.target.value),
                    },
                  })
                }
                className="w-full accent-cmf-orange"
              />
            </div>
          </div>
        )}

        {/* Tab 3: Hardware & Background Behavior */}
        {activeTab === "hardware" && (
          <div className="p-8 max-w-lg flex flex-col gap-6">
            <div className="flex items-center justify-between p-4 rounded-xl bg-[#14161B] border border-white/10">
              <div>
                <div className="text-xs font-medium text-white">Audio Feedback Beep</div>
                <div className="text-[10px] text-neutral-400">
                  Emit hardware audio tone in earbuds on rotation steps
                </div>
              </div>
              <input
                type="checkbox"
                checked={currentConfig.settings.haptic_beep}
                onChange={(e) =>
                  setCurrentConfig({
                    ...currentConfig,
                    settings: {
                      ...currentConfig.settings,
                      haptic_beep: e.target.checked,
                    },
                  })
                }
                className="accent-cmf-orange w-4 h-4"
              />
            </div>

            <div className="flex items-center justify-between p-4 rounded-xl bg-[#14161B] border border-white/10">
              <div>
                <div className="text-xs font-medium text-white">Start Minimized in System Tray</div>
                <div className="text-[10px] text-neutral-400">
                  Keep running quietly in top panel tray when window closes
                </div>
              </div>
              <input
                type="checkbox"
                checked={currentConfig.settings.start_in_tray}
                onChange={(e) =>
                  setCurrentConfig({
                    ...currentConfig,
                    settings: {
                      ...currentConfig.settings,
                      start_in_tray: e.target.checked,
                    },
                  })
                }
                className="accent-cmf-orange w-4 h-4"
              />
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
