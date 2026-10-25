import React, { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ArrowUp, ArrowDown, Keyboard, Plus, Search, Trash2, AppWindow } from "lucide-react";
import type { InstalledApp, MacroStep, MacroStepKind, WheelAction } from "../../types/profile";
import { describeStep, MACRO_STEP_KINDS, MAX_WAIT_MS, moveItem, newStep, parseKeysInput } from "../../utils/macro";

const LABEL = "font-ndot text-[11px] text-neutral-400 mb-1.5 block uppercase tracking-wider";
const INPUT =
  "w-full px-3.5 py-2 rounded-lg bg-[#111317] border border-white/10 text-xs text-white font-ndot focus:outline-none focus:border-white tracking-wider";
const SMALL_BTN =
  "px-2.5 py-1.5 rounded-lg bg-white/[0.06] hover:bg-white/[0.12] text-neutral-300 font-ndot text-[10px] tracking-wider uppercase transition-all cursor-pointer disabled:opacity-30 disabled:cursor-default";

// Scanning the app folders takes a moment; do it once per Settings session.
let appsPromise: Promise<InstalledApp[]> | null = null;
function loadInstalledApps(): Promise<InstalledApp[]> {
  if (!appsPromise) {
    appsPromise = invoke<InstalledApp[]>("list_installed_apps").catch((e) => {
      console.error(e);
      appsPromise = null;
      return [];
    });
  }
  return appsPromise;
}

/** App / file / URL chooser: pick from installed apps or type anything `launch` understands. */
export const LaunchTargetPicker: React.FC<{ value: string; onChange: (target: string, appName?: string) => void }> = ({
  value,
  onChange,
}) => {
  const [open, setOpen] = useState(false);
  const [apps, setApps] = useState<InstalledApp[] | null>(null);
  const [query, setQuery] = useState("");

  useEffect(() => {
    if (open && apps === null) loadInstalledApps().then(setApps);
  }, [open, apps]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    return (apps ?? []).filter((a) => !q || a.name.toLowerCase().includes(q)).slice(0, 200);
  }, [apps, query]);

  const chosen = apps?.find((a) => a.target === value);

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center gap-2">
        <input
          type="text"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder="App, file, folder or https:// URL"
          className={INPUT}
        />
        <button
          type="button"
          onClick={() => setOpen((o) => !o)}
          className="shrink-0 flex items-center gap-1.5 px-3 py-2 rounded-lg bg-white hover:bg-neutral-200 text-black font-ndot text-[10px] font-bold tracking-wider uppercase cursor-pointer"
        >
          <AppWindow className="w-3.5 h-3.5" />
          <span>{open ? "CLOSE" : "APPS"}</span>
        </button>
      </div>
      {chosen && <span className="font-ndot text-[10px] text-neutral-500 tracking-wider uppercase">→ {chosen.name}</span>}
      {open && (
        <div className="flex flex-col rounded-xl bg-[#0E1015] border border-white/10 overflow-hidden">
          <div className="flex items-center gap-2 px-3 py-2 border-b border-white/5">
            <Search className="w-3.5 h-3.5 text-neutral-500" />
            <input
              autoFocus
              type="text"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Search installed apps"
              className="flex-1 bg-transparent text-xs text-white font-ndot tracking-wider focus:outline-none"
            />
          </div>
          <div className="max-h-56 overflow-y-auto">
            {apps === null ? (
              <div className="px-3 py-3 font-ndot text-[10px] text-neutral-500 tracking-wider">SCANNING APPS…</div>
            ) : filtered.length === 0 ? (
              <div className="px-3 py-3 font-ndot text-[10px] text-neutral-500 tracking-wider">NO MATCHING APPS</div>
            ) : (
              filtered.map((a) => (
                <button
                  key={a.target}
                  type="button"
                  onClick={() => {
                    onChange(a.target, a.name);
                    setOpen(false);
                    setQuery("");
                  }}
                  className={`w-full text-left px-3 py-1.5 text-xs font-ndot tracking-wider cursor-pointer hover:bg-white/[0.08] ${
                    a.target === value ? "text-white bg-white/[0.06]" : "text-neutral-300"
                  }`}
                >
                  {a.name}
                </button>
              ))
            )}
          </div>
        </div>
      )}
    </div>
  );
};

const KeysField: React.FC<{
  keys: string[];
  recording: boolean;
  stagedCombo: string;
  onRecord: () => void;
  onChange: (keys: string[]) => void;
}> = ({ keys, recording, stagedCombo, onRecord, onChange }) => {
  const [raw, setRaw] = useState(keys.join(", "));
  useEffect(() => setRaw(keys.join(", ")), [keys.join(",")]);

  if (recording) {
    return (
      <div className="flex items-center gap-2 px-3 py-2 rounded-lg bg-red-950/20 border border-red-500/40">
        <span className="w-2 h-2 rounded-full bg-red-500 animate-ping shrink-0" />
        <span className="font-ndot text-[11px] text-red-300 tracking-widest uppercase">
          {stagedCombo ? stagedCombo.toUpperCase() : "PRESS KEYS…"}
        </span>
        <span className="ml-auto font-ndot text-[9px] text-neutral-500 tracking-wider">ESC CANCEL</span>
      </div>
    );
  }
  return (
    <div className="flex items-center gap-2">
      <input
        type="text"
        value={raw}
        onChange={(e) => {
          setRaw(e.target.value);
          onChange(parseKeysInput(e.target.value));
        }}
        placeholder="Ctrl+Shift+P"
        className={`${INPUT} uppercase`}
      />
      <button type="button" onClick={onRecord} className={`${SMALL_BTN} shrink-0 flex items-center gap-1`} title="Record keys">
        <Keyboard className="w-3.5 h-3.5" />
        REC
      </button>
    </div>
  );
};

const StepFields: React.FC<{
  step: MacroStep;
  recording: boolean;
  stagedCombo: string;
  onRecord: () => void;
  onChange: (step: MacroStep) => void;
}> = ({ step, recording, stagedCombo, onRecord, onChange }) => {
  switch (step.kind) {
    case "hotkey":
      return (
        <KeysField
          keys={step.keys}
          recording={recording}
          stagedCombo={stagedCombo}
          onRecord={onRecord}
          // A hand-edited PC shortcut invalidates any authored macOS form.
          onChange={(keys) => onChange({ kind: "hotkey", keys })}
        />
      );
    case "text":
      return (
        <textarea
          value={step.text}
          onChange={(e) => onChange({ ...step, text: e.target.value })}
          rows={2}
          placeholder="Text to type"
          className={`${INPUT} resize-y`}
        />
      );
    case "wait":
      return (
        <div className="flex items-center gap-2">
          <input
            type="number"
            min={0}
            max={MAX_WAIT_MS}
            step={50}
            value={step.ms}
            onChange={(e) =>
              onChange({ ...step, ms: Math.max(0, Math.min(MAX_WAIT_MS, Math.round(Number(e.target.value) || 0))) })
            }
            className={`${INPUT} w-28`}
          />
          <span className="font-ndot text-[10px] text-neutral-500 tracking-wider">MILLISECONDS (MAX 10 000)</span>
        </div>
      );
    case "launch":
      return <LaunchTargetPicker value={step.target} onChange={(target) => onChange({ ...step, target })} />;
    case "command":
      return (
        <input
          type="text"
          value={step.cmd}
          onChange={(e) => onChange({ ...step, cmd: e.target.value })}
          placeholder="notify-send done"
          className={INPUT}
        />
      );
    case "media":
      return (
        <select
          value={step.action}
          onChange={(e) => onChange({ ...step, action: e.target.value as "playpause" | "next" | "prev" })}
          className={`${INPUT} uppercase`}
        >
          <option value="playpause">PLAY / PAUSE</option>
          <option value="next">NEXT TRACK</option>
          <option value="prev">PREVIOUS TRACK</option>
        </select>
      );
  }
};

/** Ordered step list for a macro slice. Hotkey recording is owned by the Settings window. */
export const MacroEditor: React.FC<{
  steps: MacroStep[];
  onChange: (steps: MacroStep[]) => void;
  recordingStep: number | null;
  stagedCombo: string;
  onRecordStep: (index: number) => void;
}> = ({ steps, onChange, recordingStep, stagedCombo, onRecordStep }) => {
  const [expanded, setExpanded] = useState<number | null>(steps.length ? 0 : null);
  const update = (i: number, step: MacroStep) => onChange(steps.map((s, j) => (j === i ? step : s)));
  const add = (kind: MacroStepKind) => {
    onChange([...steps, newStep(kind)]);
    setExpanded(steps.length);
  };

  return (
    <div className="flex flex-col gap-2">
      <label className={LABEL}>Macro Steps (run top to bottom)</label>
      {steps.length === 0 && (
        <div className="px-3 py-3 rounded-lg border border-dashed border-white/15 font-ndot text-[10px] text-neutral-500 tracking-wider">
          NO STEPS YET. ADD ONE BELOW, E.G. OPEN APP → WAIT → HOTKEY → TYPE TEXT.
        </div>
      )}
      {steps.map((step, i) => {
        const kindLabel = MACRO_STEP_KINDS.find((k) => k.kind === step.kind)?.short ?? step.kind;
        const isOpen = expanded === i || recordingStep === i;
        return (
          <div key={i} className={`rounded-xl border ${isOpen ? "border-white/25 bg-[#111317]" : "border-white/10 bg-[#0E1015]"}`}>
            <div className="flex items-center gap-2 px-3 py-2">
              <span className="w-5 h-5 shrink-0 rounded-full bg-white/[0.08] flex items-center justify-center font-ndot text-[10px] text-neutral-300">
                {i + 1}
              </span>
              <button
                type="button"
                onClick={() => setExpanded(isOpen ? null : i)}
                className="flex-1 min-w-0 text-left cursor-pointer"
              >
                <span className="font-ndot text-[10px] text-neutral-500 tracking-wider">{kindLabel}</span>
                <span className="block truncate font-ndot text-xs text-white tracking-wider">{describeStep(step)}</span>
              </button>
              <button type="button" disabled={i === 0} onClick={() => onChange(moveItem(steps, i, -1))} className={SMALL_BTN} title="Move up">
                <ArrowUp className="w-3 h-3" />
              </button>
              <button
                type="button"
                disabled={i === steps.length - 1}
                onClick={() => onChange(moveItem(steps, i, 1))}
                className={SMALL_BTN}
                title="Move down"
              >
                <ArrowDown className="w-3 h-3" />
              </button>
              <button
                type="button"
                onClick={() => {
                  onChange(steps.filter((_, j) => j !== i));
                  setExpanded(null);
                }}
                className={`${SMALL_BTN} hover:bg-red-500/20 hover:text-red-300`}
                title="Delete step"
              >
                <Trash2 className="w-3 h-3" />
              </button>
            </div>
            {isOpen && (
              <div className="px-3 pb-3">
                <StepFields
                  step={step}
                  recording={recordingStep === i}
                  stagedCombo={stagedCombo}
                  onRecord={() => onRecordStep(i)}
                  onChange={(s) => update(i, s)}
                />
              </div>
            )}
          </div>
        );
      })}
      <div className="flex flex-wrap gap-1.5 pt-1">
        {MACRO_STEP_KINDS.map(({ kind, label }) => (
          <button key={kind} type="button" onClick={() => add(kind)} className={`${SMALL_BTN} flex items-center gap-1`}>
            <Plus className="w-3 h-3" />
            {label}
          </button>
        ))}
      </div>
    </div>
  );
};

/** Runs the slice after a countdown so the user can focus the window it should act on. */
export const TestRunButton: React.FC<{ action: WheelAction }> = ({ action }) => {
  const [countdown, setCountdown] = useState(0);
  useEffect(() => {
    if (countdown <= 0) return;
    const t = setTimeout(() => {
      if (countdown === 1) invoke("execute_action", { action }).catch(console.error);
      setCountdown(countdown - 1);
    }, 1000);
    return () => clearTimeout(t);
  }, [countdown, action]);

  return (
    <button
      type="button"
      onClick={() => setCountdown(countdown > 0 ? 0 : 3)}
      className={`${SMALL_BTN} self-start`}
      title="Runs the slice after 3 seconds; switch to the target window meanwhile"
    >
      {countdown > 0 ? `RUNNING IN ${countdown}… (CLICK TO CANCEL)` : "TEST RUN (3S DELAY)"}
    </button>
  );
};

