import React, { useState } from "react";
import { ProfilesConfig, WheelAction, ActionType, DialProfile, getOrderedProfileIds } from "../../types/profile";
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

  React.useEffect(() => {
    setCurrentConfig(config);
  }, [config]);

  // New Dial Modal state
  const [isNewDialOpen, setIsNewDialOpen] = useState(false);
  const [newDialName, setNewDialName] = useState("");
  const [newDialIcon, setNewDialIcon] = useState("disc");
  const [newDialPatterns, setNewDialPatterns] = useState("");
  const [selectedImportActions, setSelectedImportActions] = useState<string[]>([]);

  // Action Importer Drawer in Slice Inspector
  const [isImportActionOpen, setIsImportActionOpen] = useState(false);

  // Shortcut state & recording
  const [shortcutText, setShortcutText] = useState(
    currentConfig.settings.activation_shortcut || "Super+Shift+D"
  );
  const [shortcutStatus, setShortcutStatus] = useState<string | null>(null);
  const [isRecording, setIsRecording] = useState(false);
  const [stagedCombo, setStagedCombo] = useState<string>("");
  const stagedComboRef = React.useRef<string>("");
  const heldModifiersRef = React.useRef<Set<string>>(new Set());
  const [showShortcutBanner, setShowShortcutBanner] = useState(() => {
    return !localStorage.getItem("dial_shortcut_banner_dismissed");
  });

  const activeProfile = currentConfig.profiles[selectedProfileId] || currentConfig.profiles.default;
  const actions = activeProfile?.actions || [];
  const selectedAction = actions[selectedActionIndex] || actions[0];

  const handleSaveShortcut = (sc: string) => {
    setShortcutText(sc);
    const updated: ProfilesConfig = {
      ...currentConfig,
      settings: {
        ...currentConfig.settings,
        activation_shortcut: sc,
      },
    };
    setCurrentConfig(updated);
    onSaveConfig(updated);
    invoke<string>("register_activation_shortcut", { shortcutStr: sc })
      .then(() => setShortcutStatus(sc ? "RECORDED & ACTIVE" : "DISABLED"))
      .catch(() => setShortcutStatus("ERROR"));
    setTimeout(() => setShortcutStatus(null), 3000);
  };

  // Helper functions for bulletproof keyboard combination detection
  const getModifierFromKey = (e: KeyboardEvent): "Super" | "Ctrl" | "Alt" | "Shift" | null => {
    const code = (e.code || "").toLowerCase();
    const key = (e.key || "").toLowerCase();

    if (
      code.startsWith("meta") ||
      code.startsWith("os") ||
      key.includes("super") ||
      key.includes("meta") ||
      key === "os"
    ) {
      return "Super";
    }
    if (code.startsWith("control") || key.includes("ctrl") || key.includes("control")) {
      return "Ctrl";
    }
    if (code.startsWith("alt") || key.includes("alt")) {
      return "Alt";
    }
    if (code.startsWith("shift") || key.includes("shift")) {
      return "Shift";
    }
    return null;
  };

  const isModifier = (e: KeyboardEvent): boolean => {
    return getModifierFromKey(e) !== null;
  };

  const getActiveModifiers = (): string[] => {
    const active: string[] = [];
    if (heldModifiersRef.current.has("Super")) active.push("Super");
    if (heldModifiersRef.current.has("Ctrl")) active.push("Ctrl");
    if (heldModifiersRef.current.has("Alt")) active.push("Alt");
    if (heldModifiersRef.current.has("Shift")) active.push("Shift");
    return active;
  };

  const getMainKey = (e: KeyboardEvent): string | null => {
    if (isModifier(e)) return null;

    const code = e.code || "";
    if (code.startsWith("Key")) return code.replace("Key", "").toUpperCase();
    if (code.startsWith("Digit")) return code.replace("Digit", "");
    if (code.startsWith("Numpad")) return code.replace("Numpad", "");
    if (code === "Space") return "Space";
    if (code === "Backslash") return "Backslash";
    if (code === "Slash") return "Slash";
    if (code === "Backquote") return "Grave";
    if (code === "Minus") return "Minus";
    if (code === "Equal") return "Equal";
    if (code === "BracketLeft") return "BracketLeft";
    if (code === "BracketRight") return "BracketRight";
    if (code === "Tab") return "Tab";

    if (/^F\d+$/i.test(code)) return code.toUpperCase();

    if (e.key && e.key.length === 1 && !isModifier(e)) {
      return e.key.toUpperCase();
    }
    return null;
  };

  // Keyboard shortcut recorder listener (tracks multiple modifiers simultaneously)
  React.useEffect(() => {
    if (!isRecording) return;

    heldModifiersRef.current.clear();
    setStagedCombo("");
    stagedComboRef.current = "";

    const onKeyDown = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      if (e.key === "Escape") {
        setIsRecording(false);
        heldModifiersRef.current.clear();
        setStagedCombo("");
        stagedComboRef.current = "";
        return;
      }

      if (
        e.key === "Backspace" &&
        !e.ctrlKey &&
        !e.altKey &&
        !e.shiftKey &&
        !e.metaKey &&
        heldModifiersRef.current.size === 0
      ) {
        handleSaveShortcut("");
        setIsRecording(false);
        heldModifiersRef.current.clear();
        setStagedCombo("");
        stagedComboRef.current = "";
        return;
      }

      // Add modifier from key identity or event flags
      const mod = getModifierFromKey(e);
      if (mod) {
        heldModifiersRef.current.add(mod);
      }
      if (e.metaKey) heldModifiersRef.current.add("Super");
      if (e.ctrlKey) heldModifiersRef.current.add("Ctrl");
      if (e.altKey) heldModifiersRef.current.add("Alt");
      if (e.shiftKey) heldModifiersRef.current.add("Shift");

      const mods = getActiveModifiers();
      const mainKey = getMainKey(e);

      if (mainKey) {
        const fullParts = [...mods];
        if (!fullParts.includes(mainKey)) {
          fullParts.push(mainKey);
        }
        const fullCombo = fullParts.join("+");
        setStagedCombo(fullCombo);
        stagedComboRef.current = fullCombo;
      } else if (mods.length > 0) {
        const modCombo = mods.join("+");
        setStagedCombo(modCombo);
        stagedComboRef.current = modCombo;
      }
    };

    const onKeyUp = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      const combo = stagedComboRef.current;
      if (combo) {
        const parts = combo.split("+");
        // Verify we have at least one non-modifier key
        const hasMainKey = parts.some(
          (p) => !["Super", "Ctrl", "Alt", "Shift"].includes(p)
        );
        if (hasMainKey) {
          handleSaveShortcut(combo);
          setIsRecording(false);
          heldModifiersRef.current.clear();
          setStagedCombo("");
          stagedComboRef.current = "";
          return;
        }
      }

      // If only modifiers were held and one was released
      const mod = getModifierFromKey(e);
      if (mod) {
        heldModifiersRef.current.delete(mod);
      }
      if (!e.ctrlKey) heldModifiersRef.current.delete("Ctrl");
      if (!e.altKey) heldModifiersRef.current.delete("Alt");
      if (!e.shiftKey) heldModifiersRef.current.delete("Shift");

      const remainingMods = getActiveModifiers();
      if (remainingMods.length === 0) {
        setStagedCombo("");
        stagedComboRef.current = "";
      } else {
        const remainingCombo = remainingMods.join("+");
        setStagedCombo(remainingCombo);
        stagedComboRef.current = remainingCombo;
      }
    };

    const onBlur = () => {
      heldModifiersRef.current.clear();
      setStagedCombo("");
      stagedComboRef.current = "";
    };

    window.addEventListener("keydown", onKeyDown, true);
    window.addEventListener("keyup", onKeyUp, true);
    window.addEventListener("blur", onBlur);
    return () => {
      window.removeEventListener("keydown", onKeyDown, true);
      window.removeEventListener("keyup", onKeyUp, true);
      window.removeEventListener("blur", onBlur);
    };
  }, [isRecording, currentConfig]);

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

  const handleMoveProfile = (direction: -1 | 1) => {
    const ordered = getOrderedProfileIds(currentConfig);
    const currIdx = ordered.indexOf(selectedProfileId);
    if (currIdx === -1) return;
    const newIdx = currIdx + direction;
    if (newIdx >= 0 && newIdx < ordered.length) {
      const updatedOrder = [...ordered];
      const temp = updatedOrder[currIdx];
      updatedOrder[currIdx] = updatedOrder[newIdx];
      updatedOrder[newIdx] = temp;

      const updated = {
        ...currentConfig,
        profile_order: updatedOrder,
      };
      setCurrentConfig(updated);
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
        { id: `vol_${Date.now()}`, label: "VOLUME", icon: "volume", type: "mode", action: "volume", desc: "Rotate to adjust volume" },
        { id: `media_${Date.now()}`, label: "PLAY / PAUSE", icon: "playpause", type: "key", action: "playpause", desc: "Toggle media playback" }
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

    const currentOrder = getOrderedProfileIds(currentConfig);
    const updatedConfig = {
      ...currentConfig,
      profiles: {
        ...currentConfig.profiles,
        [id]: newProfile,
      },
      profile_order: [...currentOrder.filter((p) => p !== id), id],
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
    const currentOrder = getOrderedProfileIds(currentConfig).filter((id) => id !== profileId);
    setCurrentConfig({
      ...currentConfig,
      profiles: remaining,
      profile_order: currentOrder,
    });
    setSelectedProfileId("default");
    setSelectedActionIndex(0);
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
    <div className="flex h-full w-full bg-[#07080A] text-white font-ndot overflow-hidden select-none">
      {/* ── Left Sidebar Navigation ────────────────────────────────────────── */}
      <div className="w-72 shrink-0 flex flex-col justify-between border-r border-white/[0.08] bg-[#0C0D11] p-5 h-full">
        <div>
          {/* App Title in Ndot57 */}
          <div className="flex items-center gap-3 px-2 py-3 mb-6 border-b border-white/[0.06] pb-5">
            <div className="w-9 h-9 rounded-full bg-white flex items-center justify-center text-black shadow-md shrink-0">
              <Disc className="w-5 h-5 text-black animate-spin-slow" />
            </div>
            <div>
              <div className="font-ndot text-sm text-white tracking-widest whitespace-nowrap">DIAL CONTROL</div>
              <div className="text-[10px] font-ndot text-neutral-400 uppercase tracking-wider">
                NOTHING OS // 50
              </div>
            </div>
          </div>

          {/* Tab Buttons */}
          <div className="flex flex-col gap-2">
            <button
              type="button"
              onClick={() => setActiveTab("profiles")}
              className={`flex items-center gap-3.5 px-4 py-3 rounded-xl text-xs transition-all cursor-pointer select-none whitespace-nowrap ${
                activeTab === "profiles"
                  ? "bg-white/[0.1] text-white border border-white/10 shadow-sm font-bold"
                  : "text-neutral-400 hover:text-white hover:bg-white/[0.04]"
              }`}
            >
              <Layers className="w-4 h-4 text-white shrink-0" />
              <span className="font-ndot tracking-wider text-[11px] uppercase">PROFILES & DIALS</span>
            </button>

            <button
              type="button"
              onClick={() => setActiveTab("appearance")}
              className={`flex items-center gap-3.5 px-4 py-3 rounded-xl text-xs transition-all cursor-pointer select-none whitespace-nowrap ${
                activeTab === "appearance"
                  ? "bg-white/[0.1] text-white border border-white/10 shadow-sm font-bold"
                  : "text-neutral-400 hover:text-white hover:bg-white/[0.04]"
              }`}
            >
              <Palette className="w-4 h-4 text-white shrink-0" />
              <span className="font-ndot tracking-wider text-[11px] uppercase">AESTHETICS</span>
            </button>

            <button
              type="button"
              onClick={() => setActiveTab("hardware")}
              className={`flex items-center gap-3.5 px-4 py-3 rounded-xl text-xs transition-all cursor-pointer select-none whitespace-nowrap ${
                activeTab === "hardware"
                  ? "bg-white/[0.1] text-white border border-white/10 shadow-sm font-bold"
                  : "text-neutral-400 hover:text-white hover:bg-white/[0.04]"
              }`}
            >
              <Keyboard className="w-4 h-4 text-white shrink-0" />
              <span className="font-ndot tracking-wider text-[11px] uppercase">SHORTCUT & HARDWARE</span>
            </button>
          </div>
        </div>

        {/* Bottom Save Action - Permanently pinned */}
        <div className="pt-4 border-t border-white/[0.08] mt-auto">
          <button
            type="button"
            onClick={handleSave}
            className="flex items-center justify-center gap-2 w-full py-3 rounded-xl bg-white hover:bg-neutral-200 text-black text-xs font-bold transition-all shadow-md font-ndot uppercase tracking-widest cursor-pointer"
          >
            {saveSuccess ? (
              <>
                <Check className="w-4 h-4 text-black" />
                <span>SYNCED</span>
              </>
            ) : (
              <span>APPLY & SAVE</span>
            )}
          </button>
        </div>
      </div>

      {/* ── Main Content Area ──────────────────────────────────────────────── */}
      <div className="flex-1 flex flex-col bg-[#0A0B0E] overflow-hidden min-w-0">
        {/* Top Header */}
        <div className="h-16 border-b border-white/[0.08] flex items-center justify-between px-8 bg-[#0C0D11]/80 backdrop-blur-md shrink-0">
          <div className="flex items-center gap-2.5 min-w-0">
            <div className="w-2.5 h-2.5 rounded-full red-led mr-1 shrink-0" />
            <span className="font-ndot text-xs text-neutral-400 uppercase tracking-widest shrink-0">
              TELEMETRY //
            </span>
            <span className="font-ndot text-xs tracking-widest uppercase text-white truncate">
              {activeTab === "profiles"
                ? "WHEEL CONFIGURATION"
                : activeTab === "appearance"
                ? "DIAL GEOMETRY"
                : "ACTIVATION SHORTCUT & HARDWARE"}
            </span>
          </div>

          <div className="flex items-center gap-3 shrink-0 pl-4">
            <span className="font-ndot text-[11px] text-neutral-400 tracking-wider whitespace-nowrap">
              GLOBAL SHORTCUT:
            </span>
            <span className="font-ndot text-xs text-white px-2.5 py-1 rounded-md bg-white/[0.06] border border-white/10 tracking-widest whitespace-nowrap">
              {shortcutText || "NONE"}
            </span>
          </div>
        </div>

        {/* Tab 1: Profiles & Slices Editor */}
        <div className={`flex-1 flex flex-col overflow-hidden ${activeTab === "profiles" ? "" : "hidden"}`}>
          {showShortcutBanner && (
            <div className="mx-6 mt-4 p-3.5 rounded-xl bg-white/[0.04] border border-white/10 flex items-center justify-between shrink-0">
              <div className="flex items-center gap-3">
                <div className="p-2 rounded-lg bg-white text-black">
                  <Keyboard className="w-4 h-4" />
                </div>
                <div>
                  <div className="font-ndot text-xs text-white tracking-widest">
                    GLOBAL ACTIVATION SHORTCUT // {shortcutText || "NONE"}
                  </div>
                  <div className="font-ndot text-[10px] text-neutral-400 tracking-wider mt-0.5">
                    No CMF Buds dial case? Open the radial HUD using this shortcut anywhere. The dial is 100% keyboard navigable (Arrow keys to cycle, Enter/Space to select, Esc to close).
                  </div>
                </div>
              </div>
              <div className="flex items-center gap-2 shrink-0">
                <button
                  type="button"
                  onClick={() => {
                    setShowShortcutBanner(false);
                    localStorage.setItem("dial_shortcut_banner_dismissed", "true");
                  }}
                  className="px-3 py-1.5 rounded-lg bg-white/[0.08] hover:bg-white/15 text-white font-ndot text-[10px] tracking-wider uppercase transition-all cursor-pointer"
                >
                  GOT IT
                </button>
              </div>
            </div>
          )}

          <div className="flex-1 flex overflow-x-auto overflow-y-hidden">
            {/* Column 1: Profiles List */}
            <div className="w-64 shrink-0 border-r border-white/[0.08] p-4 flex flex-col gap-2 overflow-y-auto bg-[#0C0D11]/40">
              <div className="flex items-center justify-between mb-2">
                <div className="font-ndot text-[10px] uppercase text-neutral-500 tracking-widest">
                  DIALS ({Object.keys(currentConfig.profiles).length})
                </div>
                <div className="flex items-center gap-1">
                  <button
                    type="button"
                    onClick={() => handleMoveProfile(-1)}
                    className="p-1 rounded bg-white/[0.06] hover:bg-white/10 text-neutral-300 transition-all cursor-pointer"
                    title="Move selected dial up"
                  >
                    <ArrowUp className="w-3.5 h-3.5" />
                  </button>
                  <button
                    type="button"
                    onClick={() => handleMoveProfile(1)}
                    className="p-1 rounded bg-white/[0.06] hover:bg-white/10 text-neutral-300 transition-all cursor-pointer"
                    title="Move selected dial down"
                  >
                    <ArrowDown className="w-3.5 h-3.5" />
                  </button>
                  <button
                    type="button"
                    onClick={() => setIsNewDialOpen(true)}
                    className="flex items-center gap-1 px-2 py-1 rounded bg-white/[0.08] hover:bg-white/15 text-white text-[10px] font-ndot tracking-wider uppercase transition-all cursor-pointer"
                    title="Add custom dial menu"
                  >
                    <Plus className="w-3 h-3" />
                    <span>NEW</span>
                  </button>
                </div>
              </div>

              {getOrderedProfileIds(currentConfig).map((id) => {
                const prof = currentConfig.profiles[id];
                if (!prof) return null;
                return (
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
                );
              })}
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
                        onChange={(e) => {
                          const newType = e.target.value as ActionType;
                          const updates: Partial<WheelAction> = { type: newType };
                          if (newType === "mode") {
                            updates.action = "volume";
                            updates.icon = "volume";
                          } else if (newType === "key") {
                            updates.action = "playpause";
                            updates.icon = "playpause";
                          } else if (newType === "home") {
                            updates.icon = "home";
                          } else if (newType === "close") {
                            updates.icon = "close";
                          } else if (newType === "mute") {
                            updates.icon = "mute";
                          }
                          handleUpdateAction(updates);
                        }}
                        className="w-full px-3.5 py-2 rounded-lg bg-[#111317] border border-white/10 text-xs text-white focus:outline-none focus:border-white font-ndot uppercase tracking-wider"
                      >
                        <option value="hotkey">HOTKEY KEYPRESS</option>
                        <option value="mode">INTERACTIVE MODE (VOLUME/BACKLIGHT)</option>
                        <option value="key">MEDIA KEY (PLAY/NEXT/PREV)</option>
                        <option value="command">SHELL COMMAND</option>
                        <option value="mute">MUTE AUDIO TOGGLE</option>
                        <option value="profile">DIAL LINK (SWITCH PROFILE)</option>
                        <option value="home">HOME DIAL LAUNCHER</option>
                        <option value="close">CLOSE / DISMISS HUD</option>
                      </select>
                    </div>

                    {/* Mode Subtype Selector */}
                    {selectedAction.type === "mode" && (
                      <div>
                        <label className="font-ndot text-[11px] text-neutral-400 mb-1.5 block uppercase tracking-wider">
                          Interactive Mode Target
                        </label>
                        <select
                          value={
                            selectedAction.action === "brightness" || selectedAction.id?.includes("bright")
                              ? "brightness"
                              : "volume"
                          }
                          onChange={(e) => {
                            const val = e.target.value;
                            handleUpdateAction({
                              action: val,
                              icon: val === "brightness" ? "brightness" : "volume",
                              label: val === "brightness" ? "BRIGHTNESS" : "VOLUME",
                              desc: val === "brightness" ? "Rotate to adjust brightness" : "Rotate to adjust volume",
                            });
                          }}
                          className="w-full px-3.5 py-2 rounded-lg bg-[#111317] border border-white/10 text-xs text-white focus:outline-none focus:border-white font-ndot uppercase tracking-wider"
                        >
                          <option value="volume">VOLUME GAUGE</option>
                          <option value="brightness">BRIGHTNESS GAUGE</option>
                        </select>
                      </div>
                    )}

                    {/* Media Key Action Selector */}
                    {selectedAction.type === "key" && (
                      <div>
                        <label className="font-ndot text-[11px] text-neutral-400 mb-1.5 block uppercase tracking-wider">
                          Media Action
                        </label>
                        <select
                          value={selectedAction.action || "playpause"}
                          onChange={(e) => {
                            const act = e.target.value;
                            const icon = act === "next" ? "next" : act === "prev" ? "prev" : "playpause";
                            const label =
                              act === "next"
                                ? "NEXT TRACK"
                                : act === "prev"
                                ? "PREV TRACK"
                                : "PLAY / PAUSE";
                            handleUpdateAction({ action: act, icon, label });
                          }}
                          className="w-full px-3.5 py-2 rounded-lg bg-[#111317] border border-white/10 text-xs text-white focus:outline-none focus:border-white font-ndot uppercase tracking-wider"
                        >
                          <option value="playpause">PLAY / PAUSE</option>
                          <option value="next">NEXT TRACK</option>
                          <option value="prev">PREVIOUS TRACK</option>
                        </select>
                      </div>
                    )}

                    {/* Target Profile Selector */}
                    {selectedAction.type === "profile" && (
                      <div>
                        <label className="font-ndot text-[11px] text-neutral-400 mb-1.5 block uppercase tracking-wider">
                          Target Profile / Dial
                        </label>
                        <select
                          value={selectedAction.action || "default"}
                          onChange={(e) =>
                            handleUpdateAction({
                              action: e.target.value,
                              label:
                                currentConfig.profiles[e.target.value]?.name.toUpperCase() ||
                                e.target.value.toUpperCase(),
                            })
                          }
                          className="w-full px-3.5 py-2 rounded-lg bg-[#111317] border border-white/10 text-xs text-white focus:outline-none focus:border-white font-ndot uppercase tracking-wider"
                        >
                          {getOrderedProfileIds(currentConfig).map((id) => (
                            <option key={id} value={id}>
                              {currentConfig.profiles[id]?.name.toUpperCase() || id.toUpperCase()}
                            </option>
                          ))}
                        </select>
                      </div>
                    )}

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
        </div>

        {/* Tab 2: Aesthetics */}
        <div className={`flex-1 overflow-y-auto p-8 max-w-2xl flex flex-col gap-8 ${activeTab === "appearance" ? "" : "hidden"}`}>
          <div className="p-7 rounded-2xl bg-[#0C0E14] border border-white/10 flex flex-col gap-4 shadow-sm">
            <div className="font-ndot text-xs uppercase text-neutral-400 tracking-widest">
              ACCENT ILLUMINATION
            </div>
            <div className="grid grid-cols-3 gap-3.5">
              {[
                { id: "nothing_red", name: "Nothing Red", color: "#E2201F" },
                { id: "cmf_orange", name: "CMF Orange", color: "#FF5500" },
                { id: "stealth", name: "Pure Titanium", color: "#FFFFFF" },
              ].map((th) => (
                <button
                  key={th.id}
                  type="button"
                  onClick={() =>
                    setCurrentConfig({
                      ...currentConfig,
                      settings: {
                        ...currentConfig.settings,
                        theme: th.id as any,
                      },
                    })
                  }
                  className={`flex items-center gap-3 p-3.5 rounded-xl border text-xs font-ndot uppercase tracking-wider transition-all cursor-pointer ${
                    currentConfig.settings.theme === th.id
                      ? "bg-white/10 border-white/30 text-white font-bold shadow-sm"
                      : "bg-[#111317] border-white/5 text-neutral-400 hover:text-white"
                  }`}
                >
                  <div
                    className="w-4 h-4 rounded-full shrink-0 shadow-sm"
                    style={{ backgroundColor: th.color }}
                  />
                  <span className="truncate">{th.name.toUpperCase()}</span>
                </button>
              ))}
            </div>
          </div>

          <div className="p-7 rounded-2xl bg-[#0C0E14] border border-white/10 flex flex-col gap-4 shadow-sm">
            <div className="flex justify-between items-center font-ndot text-xs text-neutral-400 uppercase tracking-wider">
              <span>Outer Bezel Radius</span>
              <span className="font-ndot text-white px-2.5 py-1 rounded-md bg-white/[0.06] border border-white/10 tracking-widest text-xs">
                {currentConfig.settings.dial_radius} PX
              </span>
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
              className="w-full accent-white cursor-pointer h-2 bg-white/10 rounded-lg mt-1"
            />
          </div>

          <div className="p-7 rounded-2xl bg-[#0C0E14] border border-white/10 flex flex-col gap-4 shadow-sm">
            <div className="flex justify-between items-center font-ndot text-xs text-neutral-400 uppercase tracking-wider">
              <span>Donut Ring Width</span>
              <span className="font-ndot text-white px-2.5 py-1 rounded-md bg-white/[0.06] border border-white/10 tracking-widest text-xs">
                {currentConfig.settings.donut_thickness} PX
              </span>
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
              className="w-full accent-white cursor-pointer h-2 bg-white/10 rounded-lg mt-1"
            />
          </div>
        </div>

        {/* Tab 3: Hardware & Activation Shortcut */}
        <div className={`flex-1 overflow-y-auto p-8 max-w-2xl flex flex-col gap-8 ${activeTab === "hardware" ? "" : "hidden"}`}>
          {/* Global Shortcut Card */}
          <div className="p-7 rounded-2xl bg-[#0C0E14] border border-white/10 flex flex-col gap-6 shadow-sm">
            <div>
              <div className="font-ndot text-sm text-white uppercase tracking-widest flex items-center justify-between">
                <span>GLOBAL ACTIVATION SHORTCUT</span>
                {shortcutStatus && (
                  <span className="text-[11px] text-green-400 font-ndot tracking-wider bg-green-500/10 px-2.5 py-0.5 rounded border border-green-500/30">
                    {shortcutStatus}
                  </span>
                )}
              </div>
              <div className="text-[11px] font-ndot text-neutral-400 mt-2 uppercase tracking-wider leading-relaxed">
                Press this shortcut anywhere in your OS to trigger the radial HUD. The dial is also 100% keyboard navigable (Arrow keys cycle, Space / Enter selects, Esc closes).
              </div>
            </div>

            {/* Interactive Recording Area */}
            {isRecording ? (
              <div className="p-6 rounded-2xl bg-[#140E10] border-2 border-red-500/70 flex flex-col gap-5 shadow-lg">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-3">
                    <div className="relative flex items-center justify-center">
                      <div className="w-3 h-3 rounded-full bg-red-500" />
                      <div className="absolute w-5 h-5 rounded-full bg-red-500/40 animate-ping" />
                    </div>
                    <span className="font-ndot text-xs text-white tracking-widest uppercase font-bold">
                      RECORDING... PRESS KEYS TOGETHER
                    </span>
                  </div>
                  <div className="flex items-center gap-2">
                    {stagedCombo && stagedCombo.split("+").some(p => !["Super", "Ctrl", "Alt", "Shift"].includes(p)) && (
                      <button
                        type="button"
                        onClick={() => {
                          handleSaveShortcut(stagedCombo);
                          setIsRecording(false);
                          heldModifiersRef.current.clear();
                          setStagedCombo("");
                          stagedComboRef.current = "";
                        }}
                        className="px-3 py-1.5 rounded-lg bg-white hover:bg-neutral-200 text-black font-ndot text-[11px] tracking-wider uppercase font-bold cursor-pointer shadow-sm"
                      >
                        SAVE
                      </button>
                    )}
                    <button
                      type="button"
                      onClick={() => {
                        setIsRecording(false);
                        heldModifiersRef.current.clear();
                        setStagedCombo("");
                        stagedComboRef.current = "";
                      }}
                      className="text-neutral-400 hover:text-white text-[11px] font-ndot uppercase tracking-wider px-2 py-1 rounded hover:bg-white/10 cursor-pointer"
                    >
                      CANCEL (ESC)
                    </button>
                  </div>
                </div>

                <div className="flex items-center gap-2.5 min-h-[58px] p-3.5 rounded-xl bg-[#08090C] border border-white/10 flex-wrap">
                  {stagedCombo ? (
                    <>
                      {stagedCombo.split("+").map((keyPart, i) => (
                        <React.Fragment key={i}>
                          {i > 0 && <span className="text-red-400 font-ndot text-sm">+</span>}
                          <div className="px-4 py-2 rounded-lg bg-red-500/20 border border-red-500/60 text-red-100 font-ndot text-sm tracking-widest shadow-md">
                            {keyPart.toUpperCase()}
                          </div>
                        </React.Fragment>
                      ))}
                      {!stagedCombo.split("+").some(p => !["Super", "Ctrl", "Alt", "Shift"].includes(p)) && (
                        <>
                          <span className="text-neutral-500 font-ndot text-sm">+</span>
                          <div className="px-3 py-2 rounded-lg border border-dashed border-white/20 text-neutral-400 font-ndot text-xs tracking-wider animate-pulse">
                            PRESS ANY KEY...
                          </div>
                        </>
                      )}
                    </>
                  ) : (
                    <span className="text-neutral-500 font-ndot text-xs tracking-wider">
                      Press and hold modifier keys (Super, Ctrl, Alt, Shift) + your desired key...
                    </span>
                  )}
                </div>
                <div className="text-[10px] font-ndot text-neutral-400 tracking-wider flex items-center justify-between">
                  <span>Release keys to auto-save combination.</span>
                  <span>ESC to cancel • Backspace to disable</span>
                </div>
              </div>
            ) : (
              <div className="flex flex-col gap-5 p-5 rounded-2xl bg-[#08090C] border border-white/10">
                <div className="flex items-center justify-between flex-wrap gap-4">
                  <div className="flex items-center gap-2.5 flex-wrap">
                    {shortcutText ? (
                      shortcutText.split("+").map((keyPart, i) => (
                        <React.Fragment key={i}>
                          {i > 0 && <span className="text-neutral-500 font-ndot text-sm">+</span>}
                          <div className="px-4 py-2 rounded-xl bg-[#14171F] border border-white/20 text-white font-ndot text-sm tracking-widest shadow-inner">
                            {keyPart.toUpperCase()}
                          </div>
                        </React.Fragment>
                      ))
                    ) : (
                      <span className="text-neutral-500 font-ndot text-xs tracking-wider">
                        NO SHORTCUT CONFIGURED
                      </span>
                    )}
                  </div>

                  <div className="flex items-center gap-3">
                    <button
                      type="button"
                      onClick={() => {
                        setIsRecording(true);
                        heldModifiersRef.current.clear();
                        setStagedCombo("");
                        stagedComboRef.current = "";
                      }}
                      className="flex items-center gap-2 px-5 py-2.5 rounded-xl bg-white hover:bg-neutral-200 text-black font-ndot text-xs tracking-wider uppercase transition-all font-bold cursor-pointer shadow-md"
                    >
                      <Keyboard className="w-4 h-4 text-black" />
                      <span>RECORD SHORTCUT</span>
                    </button>
                    {shortcutText && (
                      <button
                        type="button"
                        onClick={() => handleSaveShortcut("")}
                        className="px-4 py-2.5 rounded-xl bg-white/[0.06] hover:bg-red-500/20 hover:text-red-300 text-neutral-400 font-ndot text-xs tracking-wider uppercase transition-all cursor-pointer"
                        title="Remove shortcut"
                      >
                        CLEAR
                      </button>
                    )}
                  </div>
                </div>
              </div>
            )}

            {/* Quick Presets */}
            <div className="pt-2">
              <div className="font-ndot text-[11px] text-neutral-500 uppercase tracking-widest mb-3">
                QUICK PRESETS:
              </div>
              <div className="flex flex-wrap gap-2.5">
                {[
                  { label: "SUPER + SHIFT + D", val: "Super+Shift+D" },
                  { label: "SUPER + BACKSLASH (\\)", val: "Super+Backslash" },
                  { label: "ALT + SPACE", val: "Alt+Space" },
                  { label: "SUPER + K", val: "Super+K" },
                  { label: "CTRL + SHIFT + D", val: "Ctrl+Shift+D" },
                ].map((p) => (
                  <button
                    key={p.val}
                    type="button"
                    onClick={() => handleSaveShortcut(p.val)}
                    className={`px-3.5 py-2 rounded-xl border text-[11px] font-ndot tracking-wider uppercase transition-all cursor-pointer ${
                      shortcutText === p.val
                        ? "bg-white/[0.15] border-white/40 text-white font-bold shadow-sm"
                        : "bg-white/[0.03] border-white/5 text-neutral-400 hover:text-white hover:bg-white/[0.08]"
                    }`}
                  >
                    {p.label}
                  </button>
                ))}
              </div>
            </div>
          </div>

          <div className="flex items-center justify-between p-6 rounded-2xl bg-[#0C0E14] border border-white/10 shadow-sm">
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
              className="accent-white w-5 h-5 cursor-pointer"
            />
          </div>

          <div className="flex items-center justify-between p-6 rounded-2xl bg-[#0C0E14] border border-white/10 shadow-sm">
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
              className="accent-white w-5 h-5 cursor-pointer"
            />
          </div>
        </div>
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
