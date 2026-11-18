import { invoke } from "@tauri-apps/api/core";
import type { Category, Note, NotePatch, NoteType, Settings, Shadow } from "./types";

export const listNotes = () => invoke<Note[]>("list_notes");
export const getNote = (id: string) => invoke<Note>("get_note", { id });
export const listCategories = () => invoke<Category[]>("list_categories");
export const createCategory = (name: string) =>
  invoke<Category>("create_category", { name });
export const renameCategory = (id: string, name: string) =>
  invoke<Category>("rename_category", { id, name });
export const deleteCategory = (id: string) =>
  invoke<number>("delete_category", { id });
/** 按传入的 id 顺序重排分类，返回重排后的完整列表 */
export const reorderCategories = (ids: string[]) =>
  invoke<Category[]>("reorder_categories", { ids });
export const getSettings = () => invoke<Settings>("get_settings");
export const saveSettings = (settings: Settings) =>
  invoke<Settings>("save_settings", { settings });
export const takeWarning = () => invoke<string | null>("take_warning");

export const createNote = (type: NoteType, x?: number, y?: number) =>
  invoke<Note>("create_note", { noteType: type, x: x ?? null, y: y ?? null });

export const updateNote = (id: string, patch: NotePatch) =>
  invoke<Note>("update_note", { id, patch });

export const resizeNote = (id: string, width: number, height: number) =>
  invoke<Note>("resize_note", { id, width, height });

export const restoreImageSize = (id: string) =>
  invoke<Note>("restore_image_size", { id });

export const deleteNote = (id: string) => invoke<void>("delete_note", { id });
export const trashNotes = (ids: string[]) => invoke<number>("trash_notes", { ids });
export const restoreNotes = (ids: string[]) => invoke<number>("restore_notes", { ids });
export const purgeNotes = (ids: string[]) => invoke<number>("purge_notes", { ids });
export const emptyTrash = () => invoke<number>("empty_trash");
export const setNotesVisible = (ids: string[], visible: boolean) =>
  invoke<number>("set_notes_visible", { ids, visible });
export const setNotesLocked = (ids: string[], locked: boolean) =>
  invoke<number>("set_notes_locked", { ids, locked });
export const setNotesCategory = (ids: string[], category: string) =>
  invoke<number>("set_notes_category", { ids, category });
export const duplicateNote = (id: string) => invoke<Note>("duplicate_note", { id });
export const hideNote = (id: string) => invoke<void>("hide_note", { id });
export const showNote = (id: string) => invoke<void>("show_note", { id });
export const setAllVisible = (visible: boolean) =>
  invoke<boolean>("set_all_visible", { visible });
export const locateNote = (id: string) => invoke<void>("locate_note", { id });
export const applyShadowToAll = (shadow: Shadow) =>
  invoke<number>("apply_shadow_to_all", { shadow });

export const importImage = (path: string) =>
  invoke<Note>("import_image", { path });
export const importImageClipboard = () => invoke<Note>("import_image_clipboard");
export const readImage = (name: string) => invoke<string>("read_image", { name });
/** 列表缩略图：最长边 144px，Rust 侧缩放并缓存 */
export const readThumbnail = (name: string) =>
  invoke<string>("read_thumbnail", { name });
export const exportImage = (name: string, dest: string) =>
  invoke<void>("export_image", { name, dest });

export const showNoteMenu = (id: string) => invoke<void>("show_note_menu", { id });
export const setClickThrough = (id: string, value: boolean) =>
  invoke<Note>("set_click_through", { id, value });

export const openDataFolder = () => invoke<void>("open_data_folder");
export const getDataDir = () => invoke<string>("get_data_dir");
export const exportAllData = (dest: string) =>
  invoke<number>("export_all_data", { dest });
export const importAllData = (src: string) =>
  invoke<number>("import_all_data", { src });
