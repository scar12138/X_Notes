export type NoteType = "text" | "image";
export type Layer = "top" | "desktop" | "normal";
export type FrameStyle = "none" | "thin" | "rounded" | "polaroid" | "gallery";

export interface Shadow {
  enabled: boolean;
  color: string;
  opacity: number;
  blur: number;
  offsetX: number;
  offsetY: number;
  preset: string;
}

export interface Border {
  enabled: boolean;
  color: string;
  width: number;
  cornerRadius: number;
}

export interface Frame {
  style: FrameStyle;
  color: string;
  width: number;
  cornerRadius: number;
  frameBottomWidth: number;
}

export interface Note {
  id: string;
  type: NoteType;
  content: string;
  imageFile: string | null;
  x: number;
  y: number;
  width: number;
  height: number;
  layer: Layer;
  backgroundColor: string;
  textColor: string;
  fontFamily: string;
  fontSize: number;
  bold: boolean;
  italic: boolean;
  textAlign: "left" | "center";
  opacity: number;
  cornerRadius: number;
  shadow: Shadow;
  border: Border;
  frame: Frame;
  locked: boolean;
  visible: boolean;
  clickThrough: boolean;
  lockAspect: boolean;
  /** 分类 id；空字符串表示“未分类” */
  category: string;
  /** 放进回收站的时间；null 表示正常便签 */
  deletedAt: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface Category {
  id: string;
  name: string;
}

export interface Settings {
  autostart: boolean;
  defaultLayer: Layer;
  defaultShadowPreset: string;
  defaultBackgroundColor: string;
  defaultTextColor: string;
  defaultFontFamily: string;
  defaultFontSize: number;
  defaultBorderEnabled: boolean;
  defaultFrameStyle: FrameStyle;
  globalHotkeysEnabled: boolean;
  showUpdatedAt: boolean;
  managerWidth: number;
  managerHeight: number;
  /** 管理窗口上次关闭时的位置（逻辑像素）；null 表示还没记录过 */
  managerX: number | null;
  managerY: number | null;
}

export interface NotePatch extends Partial<Omit<Note, "id" | "createdAt">> {}

export const LAYER_LABELS: Record<Layer, string> = {
  top: "置顶",
  desktop: "桌面",
  normal: "普通",
};

export const FRAME_LABELS: Record<FrameStyle, string> = {
  none: "无边框",
  thin: "细线框",
  rounded: "圆角框",
  polaroid: "拍立得",
  gallery: "阴影画框",
};

export const FRAME_STYLES: FrameStyle[] = ["none", "thin", "rounded", "polaroid", "gallery"];

export const FONT_OPTIONS = [
  "Microsoft YaHei",
  "微软雅黑",
  "SimHei",
  "SimSun",
  "KaiTi",
  "DengXian",
  "Segoe UI",
  "Arial",
  "Consolas",
];

interface ShadowPreset {
  id: string;
  name: string;
  shadow: Shadow;
}

export const SHADOW_PRESETS: ShadowPreset[] = [
  {
    id: "none",
    name: "无阴影",
    shadow: {
      enabled: false,
      color: "#000000",
      opacity: 0,
      blur: 0,
      offsetX: 0,
      offsetY: 0,
      preset: "none",
    },
  },
  {
    id: "soft",
    name: "柔和",
    shadow: {
      enabled: true,
      color: "#000000",
      opacity: 0.18,
      blur: 24,
      offsetX: 0,
      offsetY: 8,
      preset: "soft",
    },
  },
  {
    id: "hard",
    name: "硬阴影",
    shadow: {
      enabled: true,
      color: "#000000",
      opacity: 0.45,
      blur: 0,
      offsetX: 4,
      offsetY: 4,
      preset: "hard",
    },
  },
  {
    id: "deep",
    name: "深重",
    shadow: {
      enabled: true,
      color: "#000000",
      opacity: 0.55,
      blur: 32,
      offsetX: 0,
      offsetY: 12,
      preset: "deep",
    },
  },
  {
    id: "neon",
    name: "霓虹发光",
    shadow: {
      enabled: true,
      color: "#FF3FA4",
      opacity: 0.8,
      blur: 32,
      offsetX: 0,
      offsetY: 0,
      preset: "neon",
    },
  },
];

export function presetShadow(name: string): Shadow {
  return (
    SHADOW_PRESETS.find((p) => p.id === name)?.shadow ?? SHADOW_PRESETS[1].shadow
  );
}

/** #RRGGBB + 透明度 -> rgba() */
function rgba(hex: string, opacity: number): string {
  const value = normalizeHex(hex);
  const r = parseInt(value.slice(1, 3), 16);
  const g = parseInt(value.slice(3, 5), 16);
  const b = parseInt(value.slice(5, 7), 16);
  const a = Math.min(1, Math.max(0, opacity));
  return `rgba(${r},${g},${b},${a})`;
}

function normalizeHex(hex: string): string {
  const v = (hex || "").trim();
  if (/^#[0-9a-fA-F]{6}$/.test(v)) return v.toUpperCase();
  if (/^#[0-9a-fA-F]{3}$/.test(v)) {
    return `#${v[1]}${v[1]}${v[2]}${v[2]}${v[3]}${v[3]}`.toUpperCase();
  }
  return "#000000";
}

export interface Insets {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

/** 窗口总留白 = 相框外扩；阴影画在便签内侧，不撑大窗口 */
export function windowInsets(note: Note): Insets {
  if (note.type !== "image" || note.frame.style === "none") {
    return { left: 0, top: 0, right: 0, bottom: 0 };
  }
  const w = Math.min(40, Math.max(0, note.frame.width));
  if (note.frame.style === "polaroid") {
    return { left: w, top: w, right: w, bottom: Math.min(200, Math.max(w, note.frame.frameBottomWidth)) };
  }
  return { left: w, top: w, right: w, bottom: w };
}

/**
 * 图片便签的外投影：只一层、spread = 0，只有偏移 + 模糊。
 * 柔和预设即 `0 8px 24px rgba(0,0,0,.18)`。
 */
export function shadowOuterBoxShadow(shadow: Shadow): string {
  if (!shadow.enabled) return "none";
  const dx = Math.round(shadow.offsetX);
  const dy = Math.round(shadow.offsetY);
  const blur = Math.round(shadow.blur);
  return `${dx}px ${dy}px ${blur}px ${rgba(shadow.color, shadow.opacity)}`;
}

/**
 * 窗口与便签等大，阴影不能外扩：画成紧贴便签内侧的柔和暗边。
 * 偏移取反，让「光从上方来」的投影落在下缘内侧。
 */
export function shadowBoxShadow(shadow: Shadow): string {
  if (!shadow.enabled) return "none";
  const blur = Math.min(18, Math.max(4, shadow.blur * 0.6));
  const dx = -shadow.offsetX * 0.5;
  const dy = -Math.max(0, shadow.offsetY);
  const color = rgba(shadow.color, Math.min(1, shadow.opacity + 0.06));
  return `inset ${dx}px ${dy}px ${blur}px ${color}`;
}

export function formatTime(iso: string): string {
  if (!iso) return "";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(
    d.getHours(),
  )}:${pad(d.getMinutes())}`;
}
