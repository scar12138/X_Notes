import type { Note, NoteType } from "../types";

/**
 * 主界面筛选行的选项。
 *
 * 整排只有一个选中项，所以用一个单值而不是两个独立筛选：
 *   全部 → 文字便签 → 图片便签 → 未分类 → 各个自定义分类。
 * 「文字便签」「图片便签」是系统默认分类，硬编码在这里，不能改名也不能删除，
 * 永远排在最前面；「未分类」指 category 为空字符串的便签。
 */
export type NoteFilter =
  | { kind: "all" }
  | { kind: "type"; value: NoteType }
  | { kind: "category"; value: string };

export const FILTER_ALL: NoteFilter = { kind: "all" };

/** 稳定字符串，用于 React key 与比较：all / type:text / cat:（未分类）/ cat:<分类 id> */
export function filterKey(filter: NoteFilter): string {
  if (filter.kind === "all") return "all";
  if (filter.kind === "type") return `type:${filter.value}`;
  return `cat:${filter.value}`;
}

export function sameFilter(a: NoteFilter, b: NoteFilter): boolean {
  return filterKey(a) === filterKey(b);
}

/** 按当前选项过滤；「未分类」只留 category 为空的便签 */
export function applyFilter(notes: Note[], filter: NoteFilter): Note[] {
  if (filter.kind === "all") return notes;
  if (filter.kind === "type") return notes.filter((n) => n.type === filter.value);
  return notes.filter((n) => (n.category || "") === filter.value);
}

/** 自建分类被删掉后，筛选要回到「全部」，否则会停在一个不存在的选项上 */
export function filterStillValid(filter: NoteFilter, categoryIds: string[]): boolean {
  if (filter.kind !== "category") return true;
  if (filter.value === "") return true;
  return categoryIds.includes(filter.value);
}
