import React, { useState } from "react";
import { ProfilesConfig, WheelAction, ActionType, DialProfile } from "../../types/profile";
import { getActionIcon, ICON_MAP } from "../../utils/iconMap";
import {
  Palette,
  Layers,
  ArrowUp,
  ArrowDown,
  Plus,
  Trash2,
  Check,
  Disc,
  Keyboard,
  X,
  Copy,
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";

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

  // New Dial Modal state
  const [isNewDialOpen, setIsNewDialOpen] = useState(false);
  const [newDialName, setNewDialName] = useState("");
  const [newDialIcon, setNewDialIcon] = useState("disc");
  const [newDialPatterns, setNewDialPatterns] = useState("");
  const [selectedImportActions, setSelectedImportActions] = useState<string[]>([]);

  // Action Importer Drawer in Slice Inspector
  const [isImportActionOpen, setIsImportActionOpen] = useState(false);

  // Shortcut state
  const [shortcutText, setShortcutText] = useState(
    currentConfig.settings.activation_shortcut || "Super+Shift+D"
  );
  const [shortcutStatus, setShortcutStatus] = useState<string | null>(null);
  const [showShortcutBanner, setShowShortcutBanner] = useState(() => {
    return !localStorage.getItem("dial_shortcut_banner_dismissed");
  });

  const activeProfile = currentConfig.profiles[selectedProfileId] || currentConfig.profiles.default;
  const actions = activeProfile?.actions || [];
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

  const handleUpdateProfileMeta = (updates: Partial<DialProfile>) => {
    const updatedProfile = { ...activeProfile, ...updates };
    setCurrentConfig({
      ...currentConfig,
      profiles: {
        ...currentConfig.profiles,
        [selectedProfileId]: updatedProfile,
      },
    });
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
      label: "NEW SHORTCUT",
      icon: "disc",
      type: "hotkey",
      keys: ["KEY_LEFTCTRL", "KEY_S"],
      desc: "CUSTOM ACTION",
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

  // Create a brand new custom dial profile
  const handleCreateNewDial = () => {
    const id = newDialName.toLowerCase().replace(/[^a-z0-9]/g, "_") || `dial_${Date.now()}`;
    const patterns = newDialPatterns
      .split(",")
      .map((p) => p.trim())
      .filter((p) => p.length > 0);

    // Collect imported actions
    const imported: WheelAction[] = [];
    for (const actionId of selectedImportActions) {
      for (const prof of Object.values(currentConfig.profiles)) {
        const found = prof.actions.find((a) => a.id === actionId);
        if (found && !imported.some((a) => a.id === found.id)) {
          imported.push({ ...found });
          break;
        }
      }
    }

    // Default template actions if none selected
    if (imported.length === 0) {
      imported.push(
        { id: `vol_${Date.now()}`, label: "VOLUME", icon: "volume", type: "mode" },
        { id: `media_${Date.now()}`, label: "PLAY / PAUSE", icon: "playpause", type: "key", action: "playpause" }
      );
    }

    // Ensure HOME and CLOSE are included
    if (!imported.some((a) => a.id === "home_dial" || a.type === "home")) {
      imported.push({ id: "home_dial", label: "HOME", icon: "home", type: "home", desc: "Choose Dial Menu" });
    }
    if (!imported.some((a) => a.id === "close_hud" || a.type === "close")) {
      imported.push({ id: "close_hud", label: "CLOSE", icon: "close", type: "close", desc: "Dismiss HUD" });
    }

    const newProfile: DialProfile = {
      id,
      name: newDialName.trim() || "Custom Dial",
      icon: newDialIcon,
      match_patterns: patterns.length > 0 ? patterns : [`*${id}*`],
      actions: imported,
    };

    const updatedConfig = {
      ...currentConfig,
      profiles: {
        ...currentConfig.profiles,
        [id]: newProfile,
      },
    };

    setCurrentConfig(updatedConfig);
    setSelectedProfileId(id);
    setSelectedActionIndex(0);
    setIsNewDialOpen(false);
    setNewDialName("");
    setNewDialPatterns("");
    setSelectedImportActions([]);
  };

  const handleDeleteProfile = (profileId: string) => {
    if (profileId === "default") return;
    const { [profileId]: _, ...remaining } = currentConfig.profiles;
    setCurrentConfig({
      ...currentConfig,
      profiles: remaining,
    });
    setSelectedProfileId("default");
    setSelectedActionIndex(0);
  };

  const handleApplyShortcut = (sc: string) => {
    setShortcutText(sc);
    const updated = {
      ...currentConfig,
      settings: {
        ...currentConfig.settings,
        activation_shortcut: sc,
      },
    };
    setCurrentConfig(updated);
    invoke<string>("register_activation_shortcut", { shortcutStr: sc })
      .then(() => setShortcutStatus("ACTIVATED"))
      .catch(() => setShortcutStatus("ERROR"));
    setTimeout(() => setShortcutStatus(null), 2500);
  };

  const handleSave = () => {
    onSaveConfig(currentConfig);
    if (shortcutText) {
      invoke("register_activation_shortcut", { shortcutStr: shortcutText }).catch(console.error);
    }
    setSaveSuccess(true);
    setTimeout(() => setSaveSuccess(false), 2200);
  };

  const availableIcons = Object.keys(ICON_MAP);

  // Pool of all existing actions across all profiles for importing
  const allExistingActions: { action: WheelAction; sourceName: string }[] = [];
  const seenIds = new Set<string>();
  for (const [, prof] of Object.entries(currentConfig.profiles)) {
    for (const act of prof.actions) {
      if (act.id !== "home_dial" && act.id !== "close_hud" && !seenIds.has(act.id)) {
        seenIds.add(act.id);
        allExistingActions.push({ action: act, sourceName: prof.name });
      }
    }
  }

  return (
    <div className="flex h-screen w-screen bg-[#07080A] text-white font-ndot overflow-hidden select-none">
      {/* ── Left Sidebar Navigation ────────────────────────────────────────── */}
      <div className="w-56 shrink-0 flex flex-col border-r border-white/[0.08] bg-[#0C0D11] p-4">
        {/* App Title in Ndot57 */}
        <div className="flex items-center gap-2.5 px-2 py-3 mb-5 border-b border-white/[0.06] pb-4">
          <div className="w-8 h-8 rounded-full bg-white flex items-center justify-center text-black shadow-md">
            <Disc className="w-5 h-5 text-black animate-spin-slow" />
          </div>
          <div>
            <div className="font-ndot text-sm text-white tracking-widest">DIAL CONTROL</div>
            <div className="text-[10px] font-ndot text-neutral-400 uppercase tracking-wider">
              NOTHING OS // 50
            </div>
          </div>
        </div>

        {/* Tab Buttons */}
        <div className="flex flex-col gap-1.5 flex-1">
          <button
            onClick={() => setActiveTab("profiles")}
            className={`flex items-center gap-3 px-3.5 py-2.5 rounded-lg text-xs transition-all ${
              activeTab === "profiles"
                ? "bg-white/[0.1] text-white border border-white/10 shadow-sm"
                : "text-neutral-400 hover:text-white hover:bg-white/[0.04]"
            }`}
          >
            <Layers className="w-4 h-4 text-white" />
            <span className="font-ndot tracking-wider text-[11px] uppercase">PROFILES & DIALS</span>
          </button>

          <button
            onClick={() => setActiveTab("appearance")}
            className={`flex items-center gap-3 px-3.5 py-2.5 rounded-lg text-xs transition-all ${
              activeTab === "appearance"
                ? "bg-white/[0.1] text-white border border-white/10 shadow-sm"
                : "text-neutral-400 hover:text-white hover:bg-white/[0.04]"
            }`}
          >
            <Palette className="w-4 h-4 text-white" />
            <span className="font-ndot tracking-wider text-[11px] uppercase">AESTHETICS</span>
          </button>

          <button
            onClick={() => setActiveTab("hardware")}
            className={`flex items-center gap-3 px-3.5 py-2.5 rounded-lg text-xs transition-all ${
              activeTab === "hardware"
                ? "bg-white/[0.1] text-white border border-white/10 shadow-sm"
                : "text-neutral-400 hover:text-white hover:bg-white/[0.04]"
            }`}
          >
            <Keyboard className="w-4 h-4 text-white" />
            <span className="font-ndot tracking-wider text-[11px] uppercase">SHORTCUT & HARDWARE</span>
          </button>
        </div>

        {/* Bottom Save Action */}
        <button
          onClick={handleSave}
          className="flex items-center justify-center gap-2 w-full py-2.5 rounded-lg bg-white hover:bg-neutral-200 text-black text-xs font-bold transition-all shadow-md font-ndot uppercase tracking-widest"
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
                : "ACTIVATION SHORTCUT & HARDWARE"}
            </span>
          </div>

          <div className="flex items-center gap-3">
            <span className="font-ndot text-[11px] text-neutral-400 tracking-wider">
              GLOBAL SHORTCUT:
            </span>
            <span className="font-ndot text-xs text-white px-2 py-0.5 rounded bg-white/[0.06] border border-white/10 tracking-widest">
              {shortcutText}
            </span>
          </div>
        </div>

        {/* First-time Non-Hardware User Notice Banner */}
        {showShortcutBanner && (
          <div className="mx-6 mt-4 p-3.5 rounded-xl bg-white/[0.04] border border-white/10 flex items-center justify-between">
            <div className="flex items-center gap-3">
              <div className="p-2 rounded-lg bg-white text-black">
                <Keyboard className="w-4 h-4" />
              </div>
              <div>
                <div className="font-ndot text-xs text-white tracking-widest">
                  GLOBAL ACTIVATION SHORTCUT // {shortcutText}
                </div>
                <div className="font-ndot text-[10px] text-neutral-400 tracking-wider mt-0.5">
                  No CMF Buds dial case? Open the radial HUD using this shortcut anywhere. The dial is 100% keyboard navigable (Arrow keys to cycle, Enter/Space to select, Esc to close).
                </div>
              </div>
            </div>
            <div className="flex items-center gap-2 shrink-0">
              <button
                onClick={() => {
                  setShowShortcutBanner(false);
                  localStorage.setItem("dial_shortcut_banner_dismissed", "true");
                }}
                className="px-3 py-1.5 rounded-lg bg-white/[0.08] hover:bg-white/15 text-white font-ndot text-[10px] tracking-wider uppercase transition-all"
              >
                GOT IT
              </button>
            </div>
          </div>
        )}

        {/* Tab 1: Profiles & Slices Editor */}
        {activeTab === "profiles" && (
          <div className="flex-1 flex overflow-x-auto overflow-y-hidden">
            {/* Column 1: Profiles List */}
            <div className="w-64 shrink-0 border-r border-white/[0.08] p-4 flex flex-col gap-2 overflow-y-auto bg-[#0C0D11]/40">
              <div className="flex items-center justify-between mb-2">
                <div className="font-ndot text-[10px] uppercase text-neutral-500 tracking-widest">
                  ACTIVE DIALS ({Object.keys(currentConfig.profiles).length})
                </div>
                <button
                  onClick={() => setIsNewDialOpen(true)}
                  className="flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-white/[0.08] hover:bg-white/15 text-white text-[10px] font-ndot tracking-wider uppercase transition-all"
                  title="Add custom dial menu"
                >
                  <Plus className="w-3 h-3" />
                  <span>NEW DIAL</span>
                </button>
              </div>

              {Object.entries(currentConfig.profiles).map(([id, prof]) => (
                <div
                  key={id}
                  onClick={() => {
                    setSelectedProfileId(id);
                    setSelectedActionIndex(0);
                  }}
                  className={`group flex items-center justify-between p-3 rounded-xl border transition-all cursor-pointer ${
                    selectedProfileId === id
                      ? "bg-white/[0.08] border-white/20 text-white shadow-sm"
                      : "bg-[#101217]/60 border-white/[0.03] text-neutral-400 hover:text-white hover:bg-white/[0.04]"
                  }`}
                >
                  <div className="flex items-center gap-3 min-w-0">
                    <div className="p-2 rounded-lg bg-white/[0.05] border border-white/10 text-white shrink-0">
                      {getActionIcon(prof.icon, 16)}
                    </div>
                    <div className="min-w-0">
                      <div className="font-ndot text-xs tracking-wider truncate uppercase">
                        {prof.name}
                      </div>
                      <div className="text-[10px] font-ndot text-neutral-500 tracking-widest truncate">
                        {prof.actions.length} SLICES
                      </div>
                    </div>
                  </div>

                  {id !== "default" && (
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        handleDeleteProfile(id);
                      }}
                      className="opacity-0 group-hover:opacity-100 p-1.5 rounded hover:bg-red-500/20 text-neutral-500 hover:text-red-400 transition-all"
                      title="Delete this custom dial"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  )}
                </div>
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
                    title="Add new custom slice"
                  >
                    <Plus className="w-3.5 h-3.5" />
                  </button>
                  <button
                    onClick={() => setIsImportActionOpen(true)}
                    className="p-1.5 rounded bg-white/[0.06] hover:bg-white/10 text-white"
                    title="Import action from another dial"
                  >
                    <Copy className="w-3.5 h-3.5" />
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
                      <div className="font-ndot text-xs tracking-wider truncate uppercase">
                        {act.label}
                      </div>
                      <div className="text-[10px] font-ndot text-neutral-500 uppercase tracking-widest">
                        {act.type}
                      </div>
                    </div>
                  </button>
                ))}
              </div>
            </div>

            {/* Column 3: Slice Inspector & Profile Context Settings */}
            <div className="flex-1 min-w-[340px] p-6 overflow-y-auto">
              <div className="max-w-md flex flex-col gap-6">
                {/* Profile Context Matching Section */}
                <div className="p-4 rounded-xl bg-white/[0.03] border border-white/10">
                  <div className="font-ndot text-xs uppercase text-neutral-400 tracking-widest mb-3">
                    DIAL CONTEXT & TRIGGERS // {activeProfile.name.toUpperCase()}
                  </div>

                  <div className="flex flex-col gap-3">
                    <div>
                      <label className="font-ndot text-[11px] text-neutral-400 mb-1 block uppercase tracking-wider">
                        Dial Name
                      </label>
                      <input
                        type="text"
                        value={activeProfile.name}
                        onChange={(e) => handleUpdateProfileMeta({ name: e.target.value })}
                        className="w-full px-3 py-2 rounded-lg bg-[#111317] border border-white/10 text-xs text-white focus:outline-none focus:border-white font-ndot uppercase tracking-wider"
                      />
                    </div>

                    <div>
                      <label className="font-ndot text-[11px] text-neutral-400 mb-1 block uppercase tracking-wider">
                        Context App Patterns (Comma separated)
                      </label>
                      <input
                        type="text"
                        value={activeProfile.match_patterns?.join(", ") || ""}
                        onChange={(e) =>
                          handleUpdateProfileMeta({
                            match_patterns: e.target.value.split(",").map((s) => s.trim().toLowerCase()),
                          })
                        }
                        placeholder="*chrome*, *spotify*, *code*"
                        className="w-full px-3 py-2 rounded-lg bg-[#111317] border border-white/10 text-xs text-white focus:outline-none focus:border-white font-ndot tracking-wider"
                      />
                    </div>
                  </div>
                </div>

                {/* Slice Inspector */}
                {selectedAction && (
                  <div className="p-4 rounded-xl bg-white/[0.03] border border-white/10 flex flex-col gap-4">
                    <div className="font-ndot text-xs uppercase text-neutral-400 tracking-widest">
                      SLICE INSPECTOR // {selectedActionIndex + 1} ({selectedAction.label})
                    </div>

                    {/* Label */}
                    <div>
                      <label className="font-ndot text-[11px] text-neutral-400 mb-1.5 block uppercase tracking-wider">
                        Label
                      </label>
                      <input
                        type="text"
                        value={selectedAction.label}
                        onChange={(e) => handleUpdateAction({ label: e.target.value.toUpperCase() })}
                        className="w-full px-3.5 py-2 rounded-lg bg-[#111317] border border-white/10 text-xs text-white focus:outline-none focus:border-white font-ndot tracking-wider uppercase"
                      />
                    </div>

                    {/* Icon Selector */}
                    <div>
                      <label className="font-ndot text-[11px] text-neutral-400 mb-1.5 block uppercase tracking-wider">
                        Vector Icon
                      </label>
                      <div className="flex items-center gap-3">
                        <div className="p-2.5 rounded-lg bg-white/[0.08] border border-white/10 text-white">
                          {getActionIcon(selectedAction.icon, 20)}
                        </div>
                        <select
                          value={selectedAction.icon}
                          onChange={(e) => handleUpdateAction({ icon: e.target.value })}
                          className="flex-1 px-3.5 py-2 rounded-lg bg-[#111317] border border-white/10 text-xs text-white focus:outline-none focus:border-white font-ndot uppercase tracking-wider"
                        >
                          {availableIcons.map((ic) => (
                            <option key={ic} value={ic}>
                              {ic.toUpperCase()}
                            </option>
                          ))}
                        </select>
                      </div>
                    </div>

                    {/* Action Type */}
                    <div>
                      <label className="font-ndot text-[11px] text-neutral-400 mb-1.5 block uppercase tracking-wider">
                        Action Type
                      </label>
                      <select
                        value={selectedAction.type}
                        onChange={(e) =>
                          handleUpdateAction({ type: e.target.value as ActionType })
                        }
                        className="w-full px-3.5 py-2 rounded-lg bg-[#111317] border border-white/10 text-xs text-white focus:outline-none focus:border-white font-ndot uppercase tracking-wider"
                      >
                        <option value="hotkey">HOTKEY KEYPRESS</option>
                        <option value="mode">INTERACTIVE MODE (VOLUME/BACKLIGHT)</option>
                        <option value="key">MEDIA KEY (PLAY/NEXT/PREV)</option>
                        <option value="command">SHELL COMMAND</option>
                        <option value="mute">MUTE AUDIO TOGGLE</option>
                        <option value="home">HOME DIAL LAUNCHER</option>
                        <option value="close">CLOSE / DISMISS HUD</option>
                      </select>
                    </div>

                    {/* Keystrokes */}
                    {selectedAction.type === "hotkey" && (
                      <div>
                        <label className="font-ndot text-[11px] text-neutral-400 mb-1.5 block uppercase tracking-wider">
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
                          className="w-full px-3.5 py-2 rounded-lg bg-[#111317] border border-white/10 text-xs text-white font-ndot focus:outline-none focus:border-white tracking-wider uppercase"
                        />
                      </div>
                    )}

                    {/* Command */}
                    {selectedAction.type === "command" && (
                      <div>
                        <label className="font-ndot text-[11px] text-neutral-400 mb-1.5 block uppercase tracking-wider">
                          Shell Command
                        </label>
                        <input
                          type="text"
                          value={selectedAction.cmd || ""}
                          onChange={(e) => handleUpdateAction({ cmd: e.target.value })}
                          placeholder="gnome-terminal || xterm"
                          className="w-full px-3.5 py-2 rounded-lg bg-[#111317] border border-white/10 text-xs text-white font-ndot focus:outline-none focus:border-white tracking-wider"
                        />
                      </div>
                    )}

                    {/* Description */}
                    <div>
                      <label className="font-ndot text-[11px] text-neutral-400 mb-1.5 block uppercase tracking-wider">
                        Telemetry Hint
                      </label>
                      <input
                        type="text"
                        value={selectedAction.desc || ""}
                        onChange={(e) => handleUpdateAction({ desc: e.target.value.toUpperCase() })}
                        placeholder="e.g. CLIPBOARD COPY"
                        className="w-full px-3.5 py-2 rounded-lg bg-[#111317] border border-white/10 text-xs text-white font-ndot focus:outline-none focus:border-white uppercase tracking-wider"
                      />
                    </div>
                  </div>
                )}
              </div>
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
                  { id: "nothing_red", name: "Nothing Red", color: "#E2201F" },
                  { id: "cmf_orange", name: "CMF Orange", color: "#FF5500" },
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
                    className={`flex items-center gap-2.5 p-3 rounded-xl border text-xs font-ndot uppercase tracking-wider transition-all ${
                      currentConfig.settings.theme === th.id
                        ? "bg-white/10 border-white/30 text-white"
                        : "bg-[#111317] border-white/5 text-neutral-400 hover:text-white"
                    }`}
                  >
                    <div
                      className="w-3.5 h-3.5 rounded-full"
                      style={{ backgroundColor: th.color }}
                    />
                    <span>{th.name.toUpperCase()}</span>
                  </button>
                ))}
              </div>
            </div>

            <div>
              <div className="flex justify-between font-ndot text-xs text-neutral-400 mb-2 uppercase tracking-wider">
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
              <div className="flex justify-between font-ndot text-xs text-neutral-400 mb-2 uppercase tracking-wider">
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

        {/* Tab 3: Hardware & Activation Shortcut */}
        {activeTab === "hardware" && (
          <div className="p-8 max-w-xl flex flex-col gap-6 overflow-y-auto">
            {/* Global Shortcut Section */}
            <div className="p-5 rounded-xl bg-[#111317] border border-white/10 flex flex-col gap-4">
              <div>
                <div className="font-ndot text-xs text-white uppercase tracking-widest flex items-center justify-between">
                  <span>GLOBAL ACTIVATION SHORTCUT</span>
                  {shortcutStatus && (
                    <span className="text-[10px] text-green-400 font-ndot tracking-wider">
                      {shortcutStatus}
                    </span>
                  )}
                </div>
                <div className="text-[10px] font-ndot text-neutral-400 mt-1 uppercase tracking-wider">
                  Press this shortcut anywhere in your OS to trigger the radial HUD. Perfect for users without hardware dial earbuds.
                </div>
              </div>

              <div className="flex gap-2">
                <input
                  type="text"
                  value={shortcutText}
                  onChange={(e) => setShortcutText(e.target.value)}
                  placeholder="Super+Shift+D"
                  className="flex-1 px-3.5 py-2.5 rounded-lg bg-[#0C0D11] border border-white/10 text-xs text-white font-ndot tracking-widest focus:outline-none focus:border-white"
                />
                <button
                  onClick={() => handleApplyShortcut(shortcutText)}
                  className="px-4 py-2.5 rounded-lg bg-white text-black font-ndot text-xs tracking-wider uppercase hover:bg-neutral-200 transition-all font-bold"
                >
                  TEST & BIND
                </button>
              </div>

              {/* Quick Presets */}
              <div>
                <div className="font-ndot text-[10px] text-neutral-500 uppercase tracking-widest mb-2">
                  RECOMMENDED PRESETS (NON-COLLIDING):
                </div>
                <div className="flex flex-wrap gap-2">
                  {[
                    { label: "SUPER + SHIFT + D", val: "Super+Shift+D" },
                    { label: "SUPER + GRAVE (~)", val: "Super+Grave" },
                    { label: "SUPER + BACKSLASH (\\)", val: "Super+Backslash" },
                    { label: "ALT + SPACE", val: "Alt+Space" },
                  ].map((p) => (
                    <button
                      key={p.val}
                      onClick={() => handleApplyShortcut(p.val)}
                      className={`px-3 py-1.5 rounded-md border text-[10px] font-ndot tracking-wider uppercase transition-all ${
                        shortcutText === p.val
                          ? "bg-white/[0.12] border-white/30 text-white"
                          : "bg-white/[0.03] border-white/5 text-neutral-400 hover:text-white"
                      }`}
                    >
                      {p.label}
                    </button>
                  ))}
                </div>
              </div>
            </div>

            <div className="flex items-center justify-between p-4 rounded-xl bg-[#111317] border border-white/10">
              <div>
                <div className="font-ndot text-xs text-white uppercase tracking-wider">
                  EARBUD AUDIO FEEDBACK
                </div>
                <div className="text-[10px] font-ndot text-neutral-400 mt-1 uppercase tracking-wider">
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
                className="accent-white w-4 h-4 cursor-pointer"
              />
            </div>

            <div className="flex items-center justify-between p-4 rounded-xl bg-[#111317] border border-white/10">
              <div>
                <div className="font-ndot text-xs text-white uppercase tracking-wider">
                  START RESIDENT IN TRAY
                </div>
                <div className="text-[10px] font-ndot text-neutral-400 mt-1 uppercase tracking-wider">
                  Stay resident in top bar / tray icon when settings window is closed
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
                className="accent-white w-4 h-4 cursor-pointer"
              />
            </div>
          </div>
        )}
      </div>

      {/* ── Modal: Create New Custom Dial ──────────────────────────────────── */}
      {isNewDialOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 backdrop-blur-sm p-4">
          <div className="w-full max-w-lg bg-[#0E1015] border border-white/20 rounded-2xl p-6 shadow-2xl flex flex-col gap-5">
            <div className="flex items-center justify-between border-b border-white/10 pb-3">
              <div className="font-ndot text-sm text-white tracking-widest uppercase">
                CREATE CUSTOM DIAL MENU
              </div>
              <button
                onClick={() => setIsNewDialOpen(false)}
                className="p-1 rounded text-neutral-400 hover:text-white"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <div className="flex flex-col gap-4">
              <div>
                <label className="font-ndot text-[11px] text-neutral-400 mb-1.5 block uppercase tracking-wider">
                  Dial Name
                </label>
                <input
                  type="text"
                  value={newDialName}
                  onChange={(e) => setNewDialName(e.target.value)}
                  placeholder="e.g. Discord, Blender, Spotify"
                  className="w-full px-3.5 py-2.5 rounded-lg bg-[#14161D] border border-white/10 text-xs text-white font-ndot tracking-wider focus:outline-none focus:border-white uppercase"
                />
              </div>

              <div>
                <label className="font-ndot text-[11px] text-neutral-400 mb-1.5 block uppercase tracking-wider">
                  Icon
                </label>
                <div className="flex items-center gap-3">
                  <div className="p-2.5 rounded-lg bg-white/[0.08] border border-white/10 text-white">
                    {getActionIcon(newDialIcon, 20)}
                  </div>
                  <select
                    value={newDialIcon}
                    onChange={(e) => setNewDialIcon(e.target.value)}
                    className="flex-1 px-3.5 py-2.5 rounded-lg bg-[#14161D] border border-white/10 text-xs text-white font-ndot uppercase tracking-wider focus:outline-none focus:border-white"
                  >
                    {availableIcons.map((ic) => (
                      <option key={ic} value={ic}>
                        {ic.toUpperCase()}
                      </option>
                    ))}
                  </select>
                </div>
              </div>

              <div>
                <label className="font-ndot text-[11px] text-neutral-400 mb-1.5 block uppercase tracking-wider">
                  Context Window Patterns (Comma separated)
                </label>
                <input
                  type="text"
                  value={newDialPatterns}
                  onChange={(e) => setNewDialPatterns(e.target.value)}
                  placeholder="*discord*, *blender*, *figma*"
                  className="w-full px-3.5 py-2.5 rounded-lg bg-[#14161D] border border-white/10 text-xs text-white font-ndot tracking-wider focus:outline-none focus:border-white"
                />
              </div>

              {/* Import Slices from other dials */}
              <div>
                <label className="font-ndot text-[11px] text-neutral-400 mb-2 block uppercase tracking-wider">
                  Import Slices From Other Dials (Optional)
                </label>
                <div className="max-h-40 overflow-y-auto rounded-lg bg-[#14161D] border border-white/10 p-2 flex flex-col gap-1.5">
                  {allExistingActions.map(({ action: act, sourceName }) => {
                    const isChecked = selectedImportActions.includes(act.id);
                    return (
                      <label
                        key={act.id}
                        className="flex items-center gap-3 p-2 rounded hover:bg-white/[0.04] cursor-pointer"
                      >
                        <input
                          type="checkbox"
                          checked={isChecked}
                          onChange={(e) => {
                            if (e.target.checked) {
                              setSelectedImportActions([...selectedImportActions, act.id]);
                            } else {
                              setSelectedImportActions(
                                selectedImportActions.filter((id) => id !== act.id)
                              );
                            }
                          }}
                          className="accent-white w-3.5 h-3.5"
                        />
                        <div className="p-1 rounded bg-white/[0.05] text-white">
                          {getActionIcon(act.icon, 12)}
                        </div>
                        <span className="font-ndot text-xs text-white uppercase tracking-wider flex-1 truncate">
                          {act.label}
                        </span>
                        <span className="font-ndot text-[10px] text-neutral-500 uppercase">
                          {sourceName}
                        </span>
                      </label>
                    );
                  })}
                </div>
              </div>
            </div>

            <div className="flex justify-end gap-3 border-t border-white/10 pt-4">
              <button
                onClick={() => setIsNewDialOpen(false)}
                className="px-4 py-2 rounded-lg bg-white/[0.08] hover:bg-white/15 text-white font-ndot text-xs uppercase tracking-wider"
              >
                CANCEL
              </button>
              <button
                onClick={handleCreateNewDial}
                className="px-5 py-2 rounded-lg bg-white text-black font-ndot text-xs font-bold uppercase tracking-widest hover:bg-neutral-200 transition-all"
              >
                CREATE DIAL
              </button>
            </div>
          </div>
        </div>
      )}

      {/* ── Modal: Import Action Drawer ────────────────────────────────────── */}
      {isImportActionOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 backdrop-blur-sm p-4">
          <div className="w-full max-w-md bg-[#0E1015] border border-white/20 rounded-2xl p-6 shadow-2xl flex flex-col gap-4">
            <div className="flex items-center justify-between border-b border-white/10 pb-3">
              <div className="font-ndot text-sm text-white tracking-widest uppercase">
                ADD SLICE FROM ANOTHER DIAL
              </div>
              <button
                onClick={() => setIsImportActionOpen(false)}
                className="p-1 rounded text-neutral-400 hover:text-white"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <div className="max-h-64 overflow-y-auto rounded-lg bg-[#14161D] border border-white/10 p-2 flex flex-col gap-1.5">
              {allExistingActions.map(({ action: act, sourceName }) => (
                <button
                  key={act.id}
                  onClick={() => {
                    const newAct = { ...act, id: `${act.id}_${Date.now()}` };
                    const updatedActions = [...actions, newAct];
                    const updatedProfile = { ...activeProfile, actions: updatedActions };
                    setCurrentConfig({
                      ...currentConfig,
                      profiles: {
                        ...currentConfig.profiles,
                        [selectedProfileId]: updatedProfile,
                      },
                    });
                    setSelectedActionIndex(updatedActions.length - 1);
                    setIsImportActionOpen(false);
                  }}
                  className="flex items-center gap-3 p-2 rounded hover:bg-white/[0.08] text-left transition-all"
                >
                  <div className="p-1.5 rounded bg-white/[0.05] text-white">
                    {getActionIcon(act.icon, 14)}
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="font-ndot text-xs text-white uppercase tracking-wider truncate">
                      {act.label}
                    </div>
                    <div className="font-ndot text-[10px] text-neutral-500 uppercase tracking-widest">
                      {act.type} • {sourceName}
                    </div>
                  </div>
                </button>
              ))}
            </div>

            <div className="flex justify-end pt-2">
              <button
                onClick={() => setIsImportActionOpen(false)}
                className="px-4 py-2 rounded-lg bg-white/[0.08] hover:bg-white/15 text-white font-ndot text-xs uppercase tracking-wider"
              >
                CLOSE
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
