use std::path::Path;
use std::sync::MutexGuard;

use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::images;
use crate::models::{shadow_preset, Category, Layer, Note, NoteType, Settings, Shadow};
use crate::storage::Storage;
use crate::{
    broadcast_note, broadcast_notes, ensure_on_screen, note_label, open_note_window,
    open_note_window_ext, open_style_panel_in_manager, place_window, AppState,
};

fn lock<'a>(state: &'a State<'a, AppState>) -> Result<MutexGuard<'a, Storage>, String> {
    state
        .storage
        .lock()
        .map_err(|_| "数据被占用，请稍后重试".to_string())
}

fn now() -> String {
    chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

#[tauri::command]
pub fn list_notes(state: State<AppState>) -> Vec<Note> {
    lock(&state).map(|s| s.notes.clone()).unwrap_or_default()
}

#[tauri::command]
pub fn get_note(state: State<AppState>, id: String) -> Result<Note, String> {
    let storage = lock(&state)?;
    storage.get(&id).cloned().ok_or_else(|| "便签不存在".into())
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Settings {
    lock(&state).map(|s| s.settings.clone()).unwrap_or_default()
}

// ---------- 分类 ----------

fn clean_category_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("分类名称不能为空".into());
    }
    if name.chars().count() > 20 {
        return Err("分类名称最多 20 个字".into());
    }
    Ok(name.to_string())
}

#[tauri::command]
pub fn list_categories(state: State<AppState>) -> Vec<Category> {
    lock(&state).map(|s| s.categories.clone()).unwrap_or_default()
}

#[tauri::command]
pub fn create_category(
    app: AppHandle,
    state: State<AppState>,
    name: String,
) -> Result<Category, String> {
    let name = clean_category_name(&name)?;
    let category = {
        let mut storage = lock(&state)?;
        if let Some(existing) = storage.categories.iter().find(|c| c.name == name) {
            return Ok(existing.clone());
        }
        let category = Category {
            id: format!("cat-{}", uuid::Uuid::new_v4().simple()),
            name,
        };
        storage.categories.push(category.clone());
        storage.save_notes()?;
        category
    };
    let _ = app.emit("categories-changed", ());
    Ok(category)
}

#[tauri::command]
pub fn rename_category(
    app: AppHandle,
    state: State<AppState>,
    id: String,
    name: String,
) -> Result<Category, String> {
    let name = clean_category_name(&name)?;
    let category = {
        let mut storage = lock(&state)?;
        if storage
            .categories
            .iter()
            .any(|c| c.name == name && c.id != id)
        {
            return Err("已经有同名分类".into());
        }
        let slot = storage
            .categories
            .iter_mut()
            .find(|c| c.id == id)
            .ok_or_else(|| "分类不存在".to_string())?;
        slot.name = name;
        let category = slot.clone();
        storage.save_notes()?;
        category
    };
    let _ = app.emit("categories-changed", ());
    Ok(category)
}

#[tauri::command]
pub fn delete_category(
    app: AppHandle,
    state: State<AppState>,
    id: String,
) -> Result<usize, String> {
    let moved = {
        let mut storage = lock(&state)?;
        let before = storage.categories.len();
        storage.categories.retain(|c| c.id != id);
        if storage.categories.len() == before {
            return Err("分类不存在".into());
        }
        let mut moved = 0;
        for note in &mut storage.notes {
            if note.category == id {
                note.category = String::new();
                moved += 1;
            }
        }
        storage.save_notes()?;
        moved
    };
    let _ = app.emit("categories-changed", ());
    broadcast_notes(&app);
    Ok(moved)
}

/// 按传入的 id 顺序重排分类。顺序直接落成存储里的 Vec 顺序，
/// 因此不需要给 Category 加 order 字段，筛选行也按同一顺序显示。
#[tauri::command]
pub fn reorder_categories(
    app: AppHandle,
    state: State<AppState>,
    ids: Vec<String>,
) -> Result<Vec<Category>, String> {
    let categories = {
        let mut storage = lock(&state)?;
        storage.categories = crate::models::order_categories(&storage.categories, &ids);
        storage.save_notes()?;
        storage.categories.clone()
    };
    let _ = app.emit("categories-changed", ());
    Ok(categories)
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    state: State<AppState>,
    settings: Settings,
) -> Result<Settings, String> {
    let mut next = settings;
    next.sanitize();

    // 先改系统自启项；失败就把错误交给界面，不假装保存成功
    crate::apply_autostart(&app, next.autostart)?;

    {
        let mut storage = lock(&state)?;
        storage.settings = next.clone();
        storage.save_settings()?;
    }

    if let Err(err) = crate::register_hotkeys(&app, next.global_hotkeys_enabled) {
        if let Ok(mut storage) = lock(&state) {
            storage.warning = Some(err.clone());
        }
        let _ = app.emit("app-warning", err);
    }
    let _ = app.emit("settings-changed", next.clone());
    Ok(next)
}

#[tauri::command]
pub fn take_warning(state: State<AppState>) -> Option<String> {
    lock(&state).ok().and_then(|mut s| s.warning.take())
}

fn build_note(
    app: &AppHandle,
    storage: &Storage,
    note_type: NoteType,
    x: Option<f64>,
    y: Option<f64>,
    image: Option<(String, u32, u32)>,
) -> Note {
    let settings = &storage.settings;
    let mut note = Note::default();
    note.id = format!("note-{}", uuid::Uuid::new_v4().simple());
    note.note_type = note_type;
    note.layer = settings.default_layer;
    note.background_color = settings.default_background_color.clone();
    note.text_color = settings.default_text_color.clone();
    note.font_family = settings.default_font_family.clone();
    note.font_size = settings.default_font_size;
    note.shadow = shadow_preset(&settings.default_shadow_preset);
    note.border.enabled = settings.default_border_enabled;
    note.frame.style = settings.default_frame_style;
    note.created_at = now();
    note.updated_at = note.created_at.clone();

    if let Some((file, w, h)) = image {
        let (dw, dh) = images::fit_size(w, h, 560.0);
        note.image_file = Some(file);
        note.width = dw;
        note.height = dh;
    } else {
        note.width = 280.0;
        note.height = 200.0;
    }

    let cascade = (storage.notes.len() % 8) as f64 * 34.0;
    note.x = x.unwrap_or(140.0 + cascade);
    note.y = y.unwrap_or(120.0 + cascade);

    if let Ok(primary) = app.primary_monitor() {
        if let Some(monitor) = primary {
            let scale = monitor.scale_factor();
            let size = monitor.size();
            let pos = monitor.position();
            let mw = size.width as f64 / scale;
            let mh = size.height as f64 / scale;
            let mx = pos.x as f64 / scale;
            let my = pos.y as f64 / scale;
            note.x = note.x.clamp(mx + 20.0, (mx + mw - note.width - 80.0).max(mx + 20.0));
            note.y = note.y.clamp(my + 20.0, (my + mh - note.height - 80.0).max(my + 20.0));
        }
    }
    ensure_on_screen(app, &mut note);
    note
}

fn create_internal(
    app: &AppHandle,
    state: &State<AppState>,
    note_type: NoteType,
    x: Option<f64>,
    y: Option<f64>,
    image: Option<(String, u32, u32)>,
) -> Result<Note, String> {
    let note = {
        let mut storage = lock(state)?;
        let note = build_note(app, &storage, note_type, x, y, image);
        storage.notes.push(note.clone());
        storage.save_notes()?;
        note
    };
    open_note_window(app, &note)?;
    broadcast_notes(app);
    Ok(note)
}

#[tauri::command]
pub fn create_note(
    app: AppHandle,
    state: State<AppState>,
    note_type: NoteType,
    x: Option<f64>,
    y: Option<f64>,
) -> Result<Note, String> {
    create_internal(&app, &state, note_type, x, y, None)
}

/// update_note 允许前端修改的字段。
///
/// 位置 / 尺寸 / 样式 / 内容之外的一切（id、type、imageFile、createdAt、updatedAt、
/// deletedAt、visible 等）都必须走各自的命令：直接写 deletedAt 会绕过关窗逻辑，
/// 直接写 visible 会绕过建窗 / 销毁逻辑，所以这里用白名单而不是黑名单。
const PATCHABLE_FIELDS: &[&str] = &[
    "content",
    "x",
    "y",
    "width",
    "height",
    "layer",
    "backgroundColor",
    "textColor",
    "fontFamily",
    "fontSize",
    "bold",
    "italic",
    "textAlign",
    "opacity",
    "cornerRadius",
    "shadow",
    "border",
    "frame",
    "locked",
    "clickThrough",
    "lockAspect",
    "category",
];

fn merge_patch(note: &mut Note, patch: &Value) -> Result<(), String> {
    let mut value = serde_json::to_value(note.clone()).map_err(|e| e.to_string())?;
    {
        let obj = value
            .as_object_mut()
            .ok_or_else(|| "便签数据格式错误".to_string())?;
        if let Some(p) = patch.as_object() {
            for (k, v) in p {
                if !PATCHABLE_FIELDS.contains(&k.as_str()) {
                    continue;
                }
                obj.insert(k.clone(), v.clone());
            }
        }
    }
    let mut merged: Note =
        serde_json::from_value(value).map_err(|e| format!("便签字段无效：{e}"))?;
    merged.id = note.id.clone();
    merged.created_at = note.created_at.clone();
    merged.sanitize();
    *note = merged;
    Ok(())
}

#[tauri::command]
pub fn update_note(
    app: AppHandle,
    state: State<AppState>,
    id: String,
    patch: Value,
) -> Result<Note, String> {
    let (note, size_changed, layer_changed, click_through) = {
        let mut storage = lock(&state)?;
        let old = storage
            .get(&id)
            .cloned()
            .ok_or_else(|| "便签不存在".to_string())?;
        let mut next = old.clone();
        merge_patch(&mut next, &patch)?;
        next.updated_at = now();
        if !storage.category_exists(&next.category) {
            next.category = String::new();
        }

        let patch_has_pos = patch.get("x").is_some() || patch.get("y").is_some();
        if !patch_has_pos {
            // 相框/阴影留白变化时，保持内容在屏幕上的位置不动
            next.anchor_content_after_insets_change(old.window_insets());
        }

        let old_size = old.window_size();
        let new_size = next.window_size();
        let size_changed =
            (old_size.0 - new_size.0).abs() > 0.5 || (old_size.1 - new_size.1).abs() > 0.5;
        let layer_changed = old.layer != next.layer;

        let slot = storage
            .get_mut(&id)
            .ok_or_else(|| "便签不存在".to_string())?;
        *slot = next.clone();
        storage.save_notes()?;
        (next.clone(), size_changed, layer_changed, next.click_through)
    };

    if let Some(win) = app.get_webview_window(&note_label(&id)) {
        if layer_changed {
            crate::window_layer::apply_layer(&win, note.layer);
        }
        if size_changed || layer_changed {
            place_window(&win, &note);
        }
        let _ = win.set_ignore_cursor_events(click_through);
    }
    broadcast_note(&app, &note);
    broadcast_notes(&app);
    Ok(note)
}

#[tauri::command]
pub fn resize_note(
    app: AppHandle,
    state: State<AppState>,
    id: String,
    width: f64,
    height: f64,
) -> Result<Note, String> {
    let note = {
        let mut storage = lock(&state)?;
        let images_dir = storage.images_dir.clone();
        let slot = storage
            .get_mut(&id)
            .ok_or_else(|| "便签不存在".to_string())?;
        slot.width = width;
        slot.height = height;
        // 图片便签锁定比例时，宽高只一起变：按原图比例把高度反算回来，
        // 前端传什么比例都不至于把图压扁
        if slot.note_type == crate::models::NoteType::Image && slot.lock_aspect {
            if let Some(file) = slot.image_file.clone() {
                if let Ok((ow, oh)) = images::image_dimensions(&images_dir.join(&file)) {
                    if ow > 0 && oh > 0 {
                        let ratio = ow as f64 / oh as f64;
                        if ratio > 0.0 {
                            slot.height = slot.width / ratio;
                        }
                    }
                }
            }
        }
        slot.updated_at = now();
        slot.sanitize();
        let note = slot.clone();
        storage.save_notes()?;
        note
    };
    if let Some(win) = app.get_webview_window(&note_label(&id)) {
        place_window(&win, &note);
    }
    broadcast_note(&app, &note);
    broadcast_notes(&app);
    Ok(note)
}

#[tauri::command]
pub fn restore_image_size(
    app: AppHandle,
    state: State<AppState>,
    id: String,
) -> Result<Note, String> {
    let (file, images_dir) = {
        let storage = lock(&state)?;
        let note = storage
            .get(&id)
            .ok_or_else(|| "便签不存在".to_string())?;
        (
            note.image_file
                .clone()
                .ok_or_else(|| "不是图片便签".to_string())?,
            storage.images_dir.clone(),
        )
    };
    let (w, h) = images::image_dimensions(&images_dir.join(&file))?;
    // 超大图片仍按原比例限制在 3000px 内，避免恢复后窗口比屏幕还大
    let (w, h) = images::fit_size(w, h, 3000.0);
    let note = {
        let mut storage = lock(&state)?;
        let slot = storage
            .get_mut(&id)
            .ok_or_else(|| "便签不存在".to_string())?;
        slot.width = w;
        slot.height = h;
        slot.updated_at = now();
        slot.sanitize();
        let note = slot.clone();
        storage.save_notes()?;
        note
    };
    if let Some(win) = app.get_webview_window(&note_label(&id)) {
        place_window(&win, &note);
    }
    broadcast_note(&app, &note);
    broadcast_notes(&app);
    Ok(note)
}

#[tauri::command]
pub fn set_click_through(
    app: AppHandle,
    state: State<AppState>,
    id: String,
    value: bool,
) -> Result<Note, String> {
    let note = {
        let mut storage = lock(&state)?;
        let slot = storage
            .get_mut(&id)
            .ok_or_else(|| "便签不存在".to_string())?;
        slot.click_through = value;
        slot.updated_at = now();
        let note = slot.clone();
        storage.save_notes()?;
        note
    };
    if let Some(win) = app.get_webview_window(&note_label(&id)) {
        let _ = win.set_ignore_cursor_events(value);
    }
    broadcast_note(&app, &note);
    broadcast_notes(&app);
    Ok(note)
}

pub fn hide_note_internal(app: &AppHandle, state: &State<AppState>, id: &str) -> Result<(), String> {
    {
        let mut storage = lock(state)?;
        let slot = storage
            .get_mut(id)
            .ok_or_else(|| "便签不存在".to_string())?;
        slot.visible = false;
        slot.updated_at = now();
        storage.save_notes()?;
    }
    if let Some(win) = app.get_webview_window(&note_label(id)) {
        let _ = win.destroy();
    }
    broadcast_notes(app);
    Ok(())
}

#[tauri::command]
pub fn hide_note(app: AppHandle, state: State<AppState>, id: String) -> Result<(), String> {
    hide_note_internal(&app, &state, &id)
}

pub fn show_note_internal(app: &AppHandle, state: &State<AppState>, id: &str) -> Result<(), String> {
    show_note_internal_ex(app, state, id, false)
}

/// `flash` 为 true 时，新窗口加载完成后自己播定位高亮（见 `open_note_window_ext`）
pub fn show_note_internal_ex(
    app: &AppHandle,
    state: &State<AppState>,
    id: &str,
    flash: bool,
) -> Result<(), String> {
    let note = {
        let mut storage = lock(state)?;
        let slot = storage
            .get_mut(id)
            .ok_or_else(|| "便签不存在".to_string())?;
        if slot.is_trashed() {
            return Err("便签在回收站里，请先恢复".into());
        }
        slot.visible = true;
        slot.updated_at = now();
        ensure_on_screen(app, slot);
        let note = slot.clone();
        storage.save_notes()?;
        note
    };
    open_note_window_ext(app, &note, flash)?;
    broadcast_note(app, &note);
    broadcast_notes(app);
    Ok(())
}

#[tauri::command]
pub fn show_note(app: AppHandle, state: State<AppState>, id: String) -> Result<(), String> {
    show_note_internal(&app, &state, &id)
}

pub fn set_all_visible_internal(
    app: &AppHandle,
    state: &State<AppState>,
    visible: bool,
) -> Result<(), String> {
    let notes = {
        let mut storage = lock(state)?;
        for note in &mut storage.notes {
            if note.is_trashed() {
                continue;
            }
            note.visible = visible;
        }
        storage.save_notes()?;
        storage.notes.clone()
    };
    if visible {
        for note in notes.iter().filter(|n| n.visible && !n.is_trashed()) {
            let _ = open_note_window(app, note);
        }
    } else {
        for note in &notes {
            let label = note_label(&note.id);
            if let Some(win) = app.get_webview_window(&label) {
                let _ = win.destroy();
            }
        }
    }
    broadcast_notes(app);
    Ok(())
}

#[tauri::command]
pub fn set_all_visible(
    app: AppHandle,
    state: State<AppState>,
    visible: bool,
) -> Result<bool, String> {
    set_all_visible_internal(&app, &state, visible)?;
    Ok(visible)
}

#[tauri::command]
pub fn toggle_all_visible(app: AppHandle, state: State<AppState>) -> Result<bool, String> {
    // 只统计活动便签：回收站里的便签 visible 仍是 true，算进来会让这个开关
    // 永远走“隐藏”分支，全部隐藏后再按 Ctrl+Alt+H 也回不来
    let any_visible = {
        let storage = lock(&state)?;
        storage.notes.iter().any(|n| n.visible && !n.is_trashed())
    };
    set_all_visible_internal(&app, &state, !any_visible)?;
    Ok(!any_visible)
}

/// 把便签移进回收站：只标记时间，数据仍在，桌面窗口关闭
pub fn trash_note_internal(app: &AppHandle, state: &State<AppState>, id: &str) -> Result<(), String> {
    {
        let mut storage = lock(state)?;
        let slot = storage
            .get_mut(id)
            .ok_or_else(|| "便签不存在".to_string())?;
        if slot.deleted_at.is_none() {
            slot.deleted_at = Some(now());
        }
        slot.updated_at = now();
        storage.save_notes()?;
    }
    if let Some(win) = app.get_webview_window(&note_label(id)) {
        let _ = win.destroy();
    }
    let _ = app.emit("note-deleted", id);
    broadcast_notes(app);
    Ok(())
}

#[tauri::command]
pub fn delete_note(app: AppHandle, state: State<AppState>, id: String) -> Result<(), String> {
    trash_note_internal(&app, &state, &id)
}

#[tauri::command]
pub fn trash_notes(
    app: AppHandle,
    state: State<AppState>,
    ids: Vec<String>,
) -> Result<usize, String> {
    let mut count = 0;
    for id in ids {
        if trash_note_internal(&app, &state, &id).is_ok() {
            count += 1;
        }
    }
    Ok(count)
}

#[tauri::command]
pub fn restore_notes(
    app: AppHandle,
    state: State<AppState>,
    ids: Vec<String>,
) -> Result<usize, String> {
    let restored = {
        let mut storage = lock(&state)?;
        let mut out: Vec<Note> = Vec::new();
        for id in &ids {
            let Some(slot) = storage.get_mut(id) else {
                continue;
            };
            if slot.deleted_at.is_none() {
                continue;
            }
            slot.deleted_at = None;
            slot.updated_at = now();
            ensure_on_screen(&app, slot);
            out.push(slot.clone());
        }
        storage.save_notes()?;
        out
    };
    for note in &restored {
        if note.visible {
            let _ = open_note_window(&app, note);
        }
        broadcast_note(&app, note);
    }
    broadcast_notes(&app);
    Ok(restored.len())
}

/// 彻底删除：只允许删回收站里的便签
fn purge_ids(app: &AppHandle, state: &State<AppState>, ids: &[String]) -> Result<usize, String> {
    let mut removed = 0;
    {
        let mut storage = lock(state)?;
        for id in ids {
            let trashed = storage.get(id).is_some_and(|n| n.is_trashed());
            if !trashed {
                continue;
            }
            let Some(note) = storage.remove(id) else {
                continue;
            };
            removed += 1;
            if let Some(file) = &note.image_file {
                let still_used = storage
                    .notes
                    .iter()
                    .any(|n| n.image_file.as_deref() == Some(file.as_str()));
                if !still_used {
                    let _ = std::fs::remove_file(storage.images_dir.join(file));
                }
            }
        }
        storage.save_notes()?;
    }
    broadcast_notes(app);
    Ok(removed)
}

#[tauri::command]
pub fn purge_notes(
    app: AppHandle,
    state: State<AppState>,
    ids: Vec<String>,
) -> Result<usize, String> {
    purge_ids(&app, &state, &ids)
}

#[tauri::command]
pub fn empty_trash(app: AppHandle, state: State<AppState>) -> Result<usize, String> {
    let ids: Vec<String> = {
        let storage = lock(&state)?;
        storage
            .notes
            .iter()
            .filter(|n| n.is_trashed())
            .map(|n| n.id.clone())
            .collect()
    };
    purge_ids(&app, &state, &ids)
}

#[tauri::command]
pub fn set_notes_visible(
    app: AppHandle,
    state: State<AppState>,
    ids: Vec<String>,
    visible: bool,
) -> Result<usize, String> {
    let notes = {
        let mut storage = lock(&state)?;
        let mut out: Vec<Note> = Vec::new();
        for id in &ids {
            let Some(slot) = storage.get_mut(id) else {
                continue;
            };
            if slot.is_trashed() {
                continue;
            }
            slot.visible = visible;
            slot.updated_at = now();
            out.push(slot.clone());
        }
        storage.save_notes()?;
        out
    };
    if visible {
        for note in &notes {
            let _ = open_note_window(&app, note);
        }
    } else {
        for note in &notes {
            if let Some(win) = app.get_webview_window(&note_label(&note.id)) {
                let _ = win.destroy();
            }
        }
    }
    for note in &notes {
        broadcast_note(&app, note);
    }
    broadcast_notes(&app);
    Ok(notes.len())
}

#[tauri::command]
pub fn set_notes_locked(
    app: AppHandle,
    state: State<AppState>,
    ids: Vec<String>,
    locked: bool,
) -> Result<usize, String> {
    let notes = {
        let mut storage = lock(&state)?;
        let mut out: Vec<Note> = Vec::new();
        for id in &ids {
            let Some(slot) = storage.get_mut(id) else {
                continue;
            };
            if slot.is_trashed() {
                continue;
            }
            slot.locked = locked;
            slot.updated_at = now();
            out.push(slot.clone());
        }
        storage.save_notes()?;
        out
    };
    for note in &notes {
        broadcast_note(&app, note);
    }
    broadcast_notes(&app);
    Ok(notes.len())
}

#[tauri::command]
pub fn set_notes_category(
    app: AppHandle,
    state: State<AppState>,
    ids: Vec<String>,
    category: String,
) -> Result<usize, String> {
    let notes = {
        let mut storage = lock(&state)?;
        if !storage.category_exists(&category) {
            return Err("分类不存在".into());
        }
        let mut out: Vec<Note> = Vec::new();
        for id in &ids {
            let Some(slot) = storage.get_mut(id) else {
                continue;
            };
            if slot.is_trashed() {
                continue;
            }
            slot.category = category.clone();
            slot.updated_at = now();
            out.push(slot.clone());
        }
        storage.save_notes()?;
        out
    };
    for note in &notes {
        broadcast_note(&app, note);
    }
    broadcast_notes(&app);
    Ok(notes.len())
}

#[tauri::command]
pub fn duplicate_note(app: AppHandle, state: State<AppState>, id: String) -> Result<Note, String> {
    let note = {
        let mut storage = lock(&state)?;
        let source = storage
            .get(&id)
            .cloned()
            .ok_or_else(|| "便签不存在".to_string())?;
        let mut copy = source;
        copy.id = format!("note-{}", uuid::Uuid::new_v4().simple());
        copy.x += 34.0;
        copy.y += 34.0;
        copy.visible = true;
        copy.deleted_at = None;
        let ts = now();
        copy.created_at = ts.clone();
        copy.updated_at = ts;
        ensure_on_screen(&app, &mut copy);
        storage.notes.push(copy.clone());
        storage.save_notes()?;
        copy
    };
    open_note_window(&app, &note)?;
    broadcast_notes(&app);
    Ok(note)
}

#[tauri::command]
pub fn locate_note(app: AppHandle, state: State<AppState>, id: String) -> Result<(), String> {
    let note = {
        let storage = lock(&state)?;
        storage.get(&id).cloned().ok_or_else(|| "便签不存在".to_string())?
    };
    if note.is_trashed() {
        return Err("便签在回收站里，请先恢复".into());
    }
    // 先让所有便签清掉高亮，再只给目标窗口发一次，避免整排一起闪
    let _ = app.emit("flash-clear", ());
    let label = note_label(&id);

    // 窗口还不存在时，flash 事件发出去也没人收：改为带着 flash=1 建窗，
    // 由新窗口在页面加载完成后自己播高亮，建窗线程 show 之后负责聚焦。
    if app.get_webview_window(&label).is_none() {
        if note.visible {
            return open_note_window_ext(&app, &note, true);
        }
        return show_note_internal_ex(&app, &state, &id, true);
    }

    if !note.visible {
        show_note_internal_ex(&app, &state, &id, false)?;
    }
    let _ = app.emit_to(&label, "flash", ());
    if let Some(win) = app.get_webview_window(&label) {
        let _ = win.show();
        if note.layer == Layer::Top {
            let _ = win.set_always_on_top(true);
            let _ = win.set_focus();
        } else if note.layer == Layer::Normal {
            // 普通层定位时抬到前面，否则可能被其他窗口挡住看不到闪烁
            let _ = win.set_focus();
        }
    }
    Ok(())
}

#[tauri::command]
pub fn apply_shadow_to_all(
    app: AppHandle,
    state: State<AppState>,
    shadow: Shadow,
) -> Result<usize, String> {
    let (notes, applied) = {
        let mut storage = lock(&state)?;
        let mut applied = 0;
        for note in &mut storage.notes {
            if note.is_trashed() {
                continue;
            }
            // 换阴影会改变窗口四周留白，同步平移窗口以保持内容位置不变
            let old_insets = note.window_insets();
            note.shadow = shadow.clone();
            note.sanitize();
            note.anchor_content_after_insets_change(old_insets);
            // 纯样式操作不算“内容更新”：不动 updatedAt，列表顺序保持稳定
            applied += 1;
        }
        storage.save_notes()?;
        (storage.notes.clone(), applied)
    };
    for note in notes.iter().filter(|n| !n.is_trashed()) {
        if let Some(win) = app.get_webview_window(&note_label(&note.id)) {
            place_window(&win, note);
        }
        broadcast_note(&app, note);
    }
    broadcast_notes(&app);
    Ok(applied)
}

#[tauri::command]
pub fn import_image(
    app: AppHandle,
    state: State<AppState>,
    path: String,
) -> Result<Note, String> {
    let (file, w, h) = {
        let storage = lock(&state)?;
        images::import_image_file(&storage.images_dir, Path::new(&path))?
    };
    create_internal(&app, &state, NoteType::Image, None, None, Some((file, w, h)))
}

#[tauri::command]
pub fn import_image_clipboard(app: AppHandle, state: State<AppState>) -> Result<Note, String> {
    let (file, w, h) = {
        let storage = lock(&state)?;
        images::import_clipboard_image(&storage.images_dir)?
    };
    create_internal(&app, &state, NoteType::Image, None, None, Some((file, w, h)))
}

#[tauri::command]
pub fn read_image(state: State<AppState>, name: String) -> Result<String, String> {
    let storage = lock(&state)?;
    images::read_data_url(&storage.images_dir, &name)
}

/// 列表缩略图：Rust 侧缩到最长边 144px 再回传，不把整张原图 base64 到前端。
/// 先把 images_dir 复制出来再解码，避免解码期间一直持锁。
#[tauri::command]
pub fn read_thumbnail(state: State<AppState>, name: String) -> Result<String, String> {
    let images_dir = {
        let storage = lock(&state)?;
        storage.images_dir.clone()
    };
    images::read_thumbnail_data_url(&images_dir, &name)
}

#[tauri::command]
pub fn export_image(
    state: State<AppState>,
    name: String,
    dest: String,
) -> Result<(), String> {
    let storage = lock(&state)?;
    images::export_image_copy(&storage.images_dir, &name, Path::new(&dest))
}

#[tauri::command]
pub fn open_data_folder(state: State<AppState>) -> Result<(), String> {
    let dir = {
        let storage = lock(&state)?;
        storage.dir.clone()
    };
    std::process::Command::new("explorer")
        .arg(dir)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("打开数据文件夹失败：{e}"))
}

#[tauri::command]
pub fn get_data_dir(state: State<AppState>) -> String {
    lock(&state)
        .map(|s| s.dir.display().to_string())
        .unwrap_or_default()
}

#[tauri::command]
pub fn export_all_data(
    state: State<AppState>,
    dest: String,
) -> Result<usize, String> {
    let storage = lock(&state)?;
    crate::export::export_all(&storage.dir, &storage.images_dir, Path::new(&dest))
}

#[tauri::command]
pub fn import_all_data(
    app: AppHandle,
    state: State<AppState>,
    src: String,
) -> Result<usize, String> {
    // 关闭现有便签窗口
    let labels: Vec<String> = {
        let storage = lock(&state)?;
        let mut labels = Vec::new();
        for note in &storage.notes {
            labels.push(note_label(&note.id));
        }
        labels
    };
    for label in labels {
        if let Some(win) = app.get_webview_window(&label) {
            let _ = win.destroy();
        }
    }

    let count = {
        let mut storage = lock(&state)?;
        let dir = storage.dir.clone();
        let images_dir = storage.images_dir.clone();
        let count = crate::export::import_all(&dir, &images_dir, Path::new(&src))?;
        let fresh = Storage::load(&app);
        *storage = fresh;
        count
    };

    let (notes, settings) = {
        let storage = lock(&state)?;
        (storage.notes.clone(), storage.settings.clone())
    };

    let mut warnings = Vec::new();
    if let Err(err) = crate::apply_autostart(&app, settings.autostart) {
        warnings.push(err);
    }
    if let Err(err) = crate::register_hotkeys(&app, settings.global_hotkeys_enabled) {
        warnings.push(err);
    }
    if !warnings.is_empty() {
        let text = warnings.join("；");
        if let Ok(mut storage) = lock(&state) {
            storage.warning = Some(text.clone());
        }
        let _ = app.emit("app-warning", text);
    }

    for note in notes.iter().filter(|n| n.visible && !n.is_trashed()) {
        let _ = open_note_window(&app, note);
    }
    let _ = app.emit("settings-changed", settings);
    broadcast_notes(&app);
    Ok(count)
}

#[tauri::command]
pub fn open_style_panel(
    app: AppHandle,
    state: State<AppState>,
    id: String,
) -> Result<(), String> {
    let exists = {
        let storage = lock(&state)?;
        storage.get(&id).is_some()
    };
    if !exists {
        return Err("便签不存在".into());
    }
    open_style_panel_in_manager(&app, &id)
}

#[tauri::command]
pub fn show_note_menu(app: AppHandle, state: State<AppState>, id: String) -> Result<(), String> {
    let note = {
        let storage = lock(&state)?;
        storage.get(&id).cloned().ok_or_else(|| "便签不存在".to_string())?
    };
    crate::menu::show_note_menu(&app, &note)
}

// ---------- 托盘 / 菜单内部调用 ----------

/// 直接修改一张便签（菜单等后端入口用），修改后立即落盘并刷新窗口
pub fn mutate_note<F>(app: &AppHandle, id: &str, f: F) -> Result<Note, String>
where
    F: FnOnce(&mut Note),
{
    let note = {
        let state = app.state::<AppState>();
        let mut storage = state
            .storage
            .lock()
            .map_err(|_| "数据被占用，请稍后重试".to_string())?;
        let slot = storage.get_mut(id).ok_or_else(|| "便签不存在".to_string())?;
        f(slot);
        slot.updated_at = now();
        slot.sanitize();
        let note = slot.clone();
        storage.save_notes()?;
        note
    };
    if let Some(win) = app.get_webview_window(&note_label(&note.id)) {
        crate::window_layer::apply_layer(&win, note.layer);
        place_window(&win, &note);
        let _ = win.set_ignore_cursor_events(note.click_through);
    }
    broadcast_note(app, &note);
    broadcast_notes(app);
    Ok(note)
}

pub fn delete_quick(app: &AppHandle, id: &str) -> Result<(), String> {
    let state = app.state::<AppState>();
    delete_note(app.clone(), state, id.to_string())
}

pub fn duplicate_quick(app: &AppHandle, id: &str) -> Result<(), String> {
    let state = app.state::<AppState>();
    duplicate_note(app.clone(), state, id.to_string()).map(|_| ())
}

pub fn hide_quick(app: &AppHandle, id: &str) {
    let state = app.state::<AppState>();
    let _ = hide_note_internal(app, &state, id);
}

pub fn open_style_quick(app: &AppHandle, id: &str) -> Result<(), String> {
    let state = app.state::<AppState>();
    open_style_panel(app.clone(), state, id.to_string())
}

pub fn restore_image_size_quick(app: &AppHandle, id: &str) -> Result<(), String> {
    let state = app.state::<AppState>();
    restore_image_size(app.clone(), state, id.to_string()).map(|_| ())
}

pub fn create_text_note_quick(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    create_internal(app, &state, NoteType::Text, None, None, None).map(|_| ())
}

pub fn add_image_quick(app: &AppHandle) {
    crate::show_manager(app);
    let _ = app.emit("add-image-request", ());
}

pub fn request_settings(app: &AppHandle) {
    crate::show_manager(app);
    let _ = app.emit("open-settings", ());
}

pub fn toggle_visible_quick(app: &AppHandle) {
    let state = app.state::<AppState>();
    let _ = toggle_all_visible(app.clone(), state);
}
