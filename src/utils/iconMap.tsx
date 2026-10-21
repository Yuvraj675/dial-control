import React from "react";
import {
  Volume2,
  VolumeX,
  Sun,
  Play,
  Pause,
  SkipForward,
  SkipBack,
  Copy,
  Clipboard,
  Trash2,
  FolderPlus,
  Edit3,
  Terminal,
  Save,
  Code,
  Search,
  MessageSquare,
  GitBranch,
  ArrowLeft,
  ArrowRight,
  RotateCw,
  Plus,
  X,
  Undo2,
  Undo,
  Redo,
  ZoomIn,
  ZoomOut,
  Hand,
  MousePointer,
  Sliders,
  Settings,
  Disc,
  Layers,
  Folder,
  Monitor,
  Video,
  Globe,
  Palette,
  Home,
} from "lucide-react";

export const ICON_MAP: Record<string, React.FC<{ className?: string; size?: number; strokeWidth?: number }>> = {
  // Mode / System
  volume: Volume2,
  brightness: Sun,
  mute: VolumeX,
  settings: Settings,
  sliders: Sliders,
  disc: Disc,
  layers: Layers,
  home: Home,

  // Media
  play: Play,
  pause: Pause,
  playpause: Play,
  next: SkipForward,
  prev: SkipBack,

  // Files
  copy: Copy,
  paste: Clipboard,
  trash: Trash2,
  new_folder: FolderPlus,
  rename: Edit3,
  terminal: Terminal,
  folder: Folder,

  // Code
  save: Save,
  format: Code,
  find: Search,
  comment: MessageSquare,
  git: GitBranch,

  // Browser
  back: ArrowLeft,
  forward: ArrowRight,
  reload: RotateCw,
  new_tab: Plus,
  close_tab: X,
  reopen_tab: Undo2,
  globe: Globe,

  // Creative
  undo: Undo,
  redo: Redo,
  zoom_in: ZoomIn,
  zoom_out: ZoomOut,
  pan: Hand,
  select: MousePointer,
  palette: Palette,
  video: Video,
  monitor: Monitor,
};

export function getActionIcon(iconKey: string, size = 18, className = "", strokeWidth = 1.75) {
  const IconComponent = ICON_MAP[iconKey.toLowerCase()] || Disc;
  return <IconComponent size={size} className={className} strokeWidth={strokeWidth} />;
}
