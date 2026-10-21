import React, { useState } from "react";
import { ProfilesConfig, WheelAction, ActionType } from "../../types/profile";
import { getActionIcon, ICON_MAP } from "../../utils/iconMap";
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
      label: "NEW ACTION",
      icon: "disc",
      type: "hotkey",
      keys: ["KEY_LEFTCTRL", "KEY_S"],
      desc: "CUSTOM SHORTCUT",
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

  const availableIcons = Object.keys(ICON_MAP);

  return (
    <div className="flex h-screen w-screen bg-[#07080A] text-white font-sans overflow-hidden select-none">
      {/* ── Left Sidebar Navigation ────────────────────────────────────────── */}
      <div className="w-52 shrink-0 flex flex-col border-r border-white/[0.08] bg-[#0C0D11] p-4">
        {/* App Title in Ndot57 */}
        <div className="flex items-center gap-2.5 px-2 py-3 mb-6 border-b border-white/[0.06] pb-5">
          <div className="w-8 h-8 rounded-full bg-white flex items-center justify-center text-black shadow-md">
            <Disc className="w-5 h-5 text-black animate-spin-slow" />
          </div>
          <div>
            <div className="font-ndot text-sm text-white tracking-widest">DIAL (1)</div>
            <div className="text-[10px] font-mono text-neutral-400 uppercase tracking-wider">
              NOTHING OS // OS_50
            </div>
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
            <Layers className="w-4 h-4 text-white" />
            <span className="font-mono tracking-wider text-[11px] uppercase">Profiles</span>
          </button>

          <button
            onClick={() => setActiveTab("appearance")}
            className={`flex items-center gap-3 px-3.5 py-2.5 rounded-lg text-xs font-medium transition-all ${
              activeTab === "appearance"
                ? "bg-white/[0.1] text-white border border-white/10 shadow-sm"
                : "text-neutral-400 hover:text-white hover:bg-white/[0.04]"
            }`}
          >
            <Palette className="w-4 h-4 text-white" />
            <span className="font-mono tracking-wider text-[11px] uppercase">Aesthetics</span>
          </button>

          <button
            onClick={() => setActiveTab("hardware")}
            className={`flex items-center gap-3 px-3.5 py-2.5 rounded-lg text-xs font-medium transition-all ${
              activeTab === "hardware"
                ? "bg-white/[0.1] text-white border border-white/10 shadow-sm"
                : "text-neutral-400 hover:text-white hover:bg-white/[0.04]"
            }`}
          >
            <Sliders className="w-4 h-4 text-white" />
            <span className="font-mono tracking-wider text-[11px] uppercase">Hardware</span>
          </button>
        </div>

        {/* Bottom Save Action */}
        <button
          onClick={handleSave}
          className="flex items-center justify-center gap-2 w-full py-2.5 rounded-lg bg-white hover:bg-neutral-200 text-black text-xs font-bold transition-all shadow-md font-mono uppercase tracking-wider"
        >
          {saveSuccess ? (
            <>
              <Check className="w-4 h-4" />
              <span>SYNCED</span>
            </>
          ) : (
            <span>APPLY & SAVE</span>
          )}
        </button>
      </div>

      {/* ── Main Content Area ──────────────────────────────────────────────── */}
      <div className="flex-1 flex flex-col bg-[#0A0B0E] overflow-hidden">
        {/* Top Header */}
        <div className="h-14 border-b border-white/[0.08] flex items-center justify-between px-8 bg-[#0C0D11]/60 backdrop-blur-md">
          <div className="flex items-center gap-2">
            <div className="w-2 h-2 rounded-full red-led mr-1" />
            <span className="font-ndot text-xs text-neutral-400 uppercase tracking-widest">
              TELEMETRY //
            </span>
            <span className="font-ndot text-xs tracking-widest uppercase text-white">
              {activeTab === "profiles"
                ? "WHEEL CONFIGURATION"
                : activeTab === "appearance"
                ? "DIAL GEOMETRY"
                : "HARDWARE BUS"}
            </span>
          </div>
        </div>

        {/* Tab 1: Profiles & Slices Editor */}
        {activeTab === "profiles" && (
          <div className="flex-1 flex overflow-x-auto overflow-y-hidden">
            {/* Column 1: Profiles List */}
            <div className="w-60 shrink-0 border-r border-white/[0.08] p-4 flex flex-col gap-2 overflow-y-auto bg-[#0C0D11]/40">
              <div className="font-ndot text-[10px] uppercase text-neutral-500 tracking-widest mb-2">
                ACTIVE PROFILES ({Object.keys(currentConfig.profiles).length})
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
                      ? "bg-white/[0.08] border-white/20 text-white shadow-sm"
                      : "bg-[#101217]/60 border-white/[0.03] text-neutral-400 hover:text-white hover:bg-white/[0.04]"
                  }`}
                >
                  <div className="p-2 rounded-lg bg-white/[0.05] border border-white/10 text-white">
                    {getActionIcon(prof.icon, 16)}
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="font-ndot text-xs tracking-wider truncate">{prof.name}</div>
                    <div className="text-[10px] font-mono text-neutral-500 truncate">
                      {prof.actions.length} SLICES
                    </div>
                  </div>
                </button>
              ))}
            </div>

            {/* Column 2: Slices List */}
            <div className="w-64 shrink-0 border-r border-white/[0.08] p-4 flex flex-col overflow-y-auto">
              <div className="flex items-center justify-between mb-3">
                <div className="font-ndot text-[10px] uppercase text-neutral-500 tracking-widest">
                  WHEEL ORDER
                </div>
                <div className="flex gap-1">
                  <button
                    onClick={() => handleMoveAction(-1)}
                    className="p-1.5 rounded bg-white/[0.06] hover:bg-white/10 text-neutral-300"
                    title="Move slice earlier"
                  >
                    <ArrowUp className="w-3.5 h-3.5" />
                  </button>
                  <button
                    onClick={() => handleMoveAction(1)}
                    className="p-1.5 rounded bg-white/[0.06] hover:bg-white/10 text-neutral-300"
                    title="Move slice later"
                  >
                    <ArrowDown className="w-3.5 h-3.5" />
                  </button>
                  <button
                    onClick={handleAddAction}
                    className="p-1.5 rounded bg-white/[0.06] hover:bg-white/10 text-white"
                    title="Add slice"
                  >
                    <Plus className="w-3.5 h-3.5" />
                  </button>
                  <button
                    onClick={handleRemoveAction}
                    className="p-1.5 rounded bg-white/[0.06] hover:bg-white/10 text-red-400"
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
                        : "bg-[#101217]/40 border-white/[0.04] text-neutral-400 hover:text-white"
                    }`}
                  >
                    <div className="p-1.5 rounded bg-white/[0.05] text-white">
                      {getActionIcon(act.icon, 14)}
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className="font-ntype-headline text-xs font-bold tracking-wider truncate">
                        {act.label}
                      </div>
                      <div className="text-[10px] font-mono text-neutral-500 uppercase">
                        {act.type}
                      </div>
                    </div>
                  </button>
                ))}
              </div>
            </div>

            {/* Column 3: Slice Inspector */}
            <div className="flex-1 min-w-[300px] p-6 overflow-y-auto">
              {selectedAction && (
                <div className="max-w-md flex flex-col gap-5">
                  <div className="font-ndot text-xs uppercase text-neutral-500 tracking-widest">
                    SLICE INSPECTOR // {selectedActionIndex + 1}
                  </div>

                  {/* Label */}
                  <div>
                    <label className="font-mono text-xs text-neutral-400 mb-1.5 block uppercase">
                      Label
                    </label>
                    <input
                      type="text"
                      value={selectedAction.label}
                      onChange={(e) => handleUpdateAction({ label: e.target.value })}
                      className="w-full px-3.5 py-2 rounded-lg bg-[#111317] border border-white/10 text-xs text-white focus:outline-none focus:border-white font-mono"
                    />
                  </div>

                  {/* Icon Selector */}
                  <div>
                    <label className="font-mono text-xs text-neutral-400 mb-1.5 block uppercase">
                      Vector Icon
                    </label>
                    <div className="flex items-center gap-3">
                      <div className="p-2.5 rounded-lg bg-white/[0.08] border border-white/10 text-white">
                        {getActionIcon(selectedAction.icon, 20)}
                      </div>
                      <select
                        value={selectedAction.icon}
                        onChange={(e) => handleUpdateAction({ icon: e.target.value })}
                        className="flex-1 px-3.5 py-2 rounded-lg bg-[#111317] border border-white/10 text-xs text-white focus:outline-none focus:border-white font-mono uppercase"
                      >
                        {availableIcons.map((ic) => (
                          <option key={ic} value={ic}>
                            {ic}
                          </option>
                        ))}
                      </select>
                    </div>
                  </div>

                  {/* Type */}
                  <div>
                    <label className="font-mono text-xs text-neutral-400 mb-1.5 block uppercase">
                      Action Type
                    </label>
                    <select
                      value={selectedAction.type}
                      onChange={(e) =>
                        handleUpdateAction({ type: e.target.value as ActionType })
                      }
                      className="w-full px-3.5 py-2 rounded-lg bg-[#111317] border border-white/10 text-xs text-white focus:outline-none focus:border-white font-mono uppercase"
                    >
                      <option value="hotkey">Hotkey Keypress</option>
                      <option value="mode">Interactive Mode (Volume/Backlight)</option>
                      <option value="key">Media Key (Play/Next/Prev)</option>
                      <option value="command">Shell Command</option>
                      <option value="mute">Mute Audio Toggle</option>
                    </select>
                  </div>

                  {/* Target / Keys */}
                  {selectedAction.type === "hotkey" && (
                    <div>
                      <label className="font-mono text-xs text-neutral-400 mb-1.5 block uppercase">
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
                        placeholder="KEY_LEFTCTRL + KEY_C"
                        className="w-full px-3.5 py-2 rounded-lg bg-[#111317] border border-white/10 text-xs text-white font-mono focus:outline-none focus:border-white"
                      />
                    </div>
                  )}

                  {/* Description */}
                  <div>
                    <label className="font-mono text-xs text-neutral-400 mb-1.5 block uppercase">
                      Telemetry Hint
                    </label>
                    <input
                      type="text"
                      value={selectedAction.desc || ""}
                      onChange={(e) => handleUpdateAction({ desc: e.target.value })}
                      placeholder="e.g. CLIPBOARD COPY"
                      className="w-full px-3.5 py-2 rounded-lg bg-[#111317] border border-white/10 text-xs text-white font-mono focus:outline-none focus:border-white uppercase"
                    />
                  </div>
                </div>
              )}
            </div>
          </div>
        )}

        {/* Tab 2: Aesthetics */}
        {activeTab === "appearance" && (
          <div className="p-8 max-w-lg flex flex-col gap-6">
            <div>
              <div className="font-ndot text-xs uppercase text-neutral-400 tracking-widest mb-3">
                ACCENT ILLUMINATION
              </div>
              <div className="grid grid-cols-3 gap-3">
                {[
                  { id: "cmf_orange", name: "CMF Orange", color: "#FF5500" },
                  { id: "cmf_cyan", name: "Electric Cyan", color: "#00E5FF" },
                  { id: "stealth", name: "Pure Titanium", color: "#FFFFFF" },
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
                    className={`flex items-center gap-2.5 p-3 rounded-xl border text-xs font-mono uppercase transition-all ${
                      currentConfig.settings.theme === th.id
                        ? "bg-white/10 border-white/30 text-white"
                        : "bg-[#111317] border-white/5 text-neutral-400 hover:text-white"
                    }`}
                  >
                    <div
                      className="w-3.5 h-3.5 rounded-full"
                      style={{ backgroundColor: th.color }}
                    />
                    <span>{th.name}</span>
                  </button>
                ))}
              </div>
            </div>

            <div>
              <div className="flex justify-between font-mono text-xs text-neutral-400 mb-2 uppercase">
                <span>Outer Bezel Radius</span>
                <span className="font-ndot text-white">{currentConfig.settings.dial_radius} PX</span>
              </div>
              <input
                type="range"
                min="150"
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
                className="w-full accent-white"
              />
            </div>

            <div>
              <div className="flex justify-between font-mono text-xs text-neutral-400 mb-2 uppercase">
                <span>Donut Ring Width</span>
                <span className="font-ndot text-white">{currentConfig.settings.donut_thickness} PX</span>
              </div>
              <input
                type="range"
                min="45"
                max="85"
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
                className="w-full accent-white"
              />
            </div>
          </div>
        )}

        {/* Tab 3: Hardware Controls */}
        {activeTab === "hardware" && (
          <div className="p-8 max-w-lg flex flex-col gap-6">
            <div className="flex items-center justify-between p-4 rounded-xl bg-[#111317] border border-white/10">
              <div>
                <div className="font-ndot text-xs text-white uppercase tracking-wider">
                  EARBUD AUDIO FEEDBACK
                </div>
                <div className="text-[10px] font-mono text-neutral-400 mt-1 uppercase">
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
                className="accent-white w-4 h-4"
              />
            </div>

            <div className="flex items-center justify-between p-4 rounded-xl bg-[#111317] border border-white/10">
              <div>
                <div className="font-ndot text-xs text-white uppercase tracking-wider">
                  START RESIDENT IN TRAY
                </div>
                <div className="text-[10px] font-mono text-neutral-400 mt-1 uppercase">
                  Run quietly in background panel when window closes
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
                className="accent-white w-4 h-4"
              />
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
