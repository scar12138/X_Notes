import type { Note } from "../types";

/** 便签在列表里的标题：图片便签固定文案，文字便签取第一行非空内容 */
export function noteTitle(note: Note): string {
  if (note.type === "image") return "图片便签";
  const preview = (note.content || "").split("\n").find((l) => l.trim()) ?? "";
  return preview || "（空白便签）";
}

/** 按字符数截断，超出部分用省略号代替 */
export function truncate(text: string, max: number): string {
  if (text.length <= max) return text;
  return `${text.slice(0, max)}…`;
}
