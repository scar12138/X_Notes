//! 便签右键菜单（Rust 侧原生菜单，可超出便签窗口显示）

use tauri::menu::{CheckMenuItem, ContextMenu, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};

use crate::commands;
use crate::models::{FrameStyle, Layer, Note, NoteType};

fn menu_id(action: &str, note_id: &str) -> String {
    format!("note:{action}:{note_id}")
}

pub fn show_note_menu(app: &AppHandle, note: &Note) -> Result<(), String> {
    let id = note.id.as_str();
    let is_image = note.note_type == NoteType::Image;
    let err = |e: tauri::Error| e.to_string();

    let edit = MenuItem::with_id(
        app,
        menu_id("edit", id),
        "编辑",
        !note.locked && !is_image,
        None::<&str>,
    )
    .map_err(err)?;

    let lock_text = if note.locked { "解锁" } else { "锁定" };
    let lock = MenuItem::with_id(app, menu_id("lock", id), lock_text, true, None::<&str>)
        .map_err(err)?;

    let layer_top = CheckMenuItem::with_id(
        app,
        menu_id("layer-top", id),
        "置顶（总在最前）",
        true,
        note.layer == Layer::Top,
        None::<&str>,
    )
    .map_err(err)?;
    let layer_desktop = CheckMenuItem::with_id(
        app,
        menu_id("layer-desktop", id),
        "桌面（被窗口盖住）",
        true,
        note.layer == Layer::Desktop,
        None::<&str>,
    )
    .map_err(err)?;
    let layer_normal = CheckMenuItem::with_id(
        app,
        menu_id("layer-normal", id),
        "普通（参与层叠）",
        true,
        note.layer == Layer::Normal,
        None::<&str>,
    )
    .map_err(err)?;
    let layer_menu = Submenu::with_id_and_items(
        app,
        menu_id("layer", id),
        "窗口层级",
        true,
        &[&layer_top, &layer_desktop, &layer_normal],
    )
    .map_err(err)?;

    let style = MenuItem::with_id(app, menu_id("style", id), "样式设置…", true, None::<&str>)
        .map_err(err)?;

    // 分类子菜单：未分类 + 用户自建分类
    let categories = {
        let state = app.state::<crate::AppState>();
        state
            .storage
            .lock()
            .map(|s| s.categories.clone())
            .unwrap_or_default()
    };
    let mut category_items: Vec<CheckMenuItem<tauri::Wry>> = Vec::new();
    let all_categories = std::iter::once((String::new(), "未分类".to_string()))
        .chain(categories.iter().map(|c| (c.id.clone(), c.name.clone())));
    for (cat_id, cat_name) in all_categories {
        let item = CheckMenuItem::with_id(
            app,
            format!("note:category:{}:{}", id, cat_id),
            cat_name,
            true,
            note.category == cat_id,
            None::<&str>,
        )
        .map_err(err)?;
        category_items.push(item);
    }
    let category_refs: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> = category_items
        .iter()
        .map(|i| i as &dyn tauri::menu::IsMenuItem<tauri::Wry>)
        .collect();
    let category_menu =
        Submenu::with_items(app, "分类", true, &category_refs).map_err(err)?;

    let restore_size = if is_image {
        Some(
            MenuItem::with_id(
                app,
                menu_id("restore-size", id),
                "恢复原始大小",
                true,
                None::<&str>,
            )
            .map_err(err)?,
        )
    } else {
        None
    };
    let save_image = if is_image {
        Some(
            MenuItem::with_id(
                app,
                menu_id("save-image", id),
                "另存为原图…",
                true,
                None::<&str>,
            )
            .map_err(err)?,
        )
    } else {
        None
    };

    let duplicate = MenuItem::with_id(app, menu_id("duplicate", id), "复制便签", true, None::<&str>)
        .map_err(err)?;
    let click_through = CheckMenuItem::with_id(
        app,
        menu_id("clickthrough", id),
        "鼠标穿透",
        true,
        note.click_through,
        None::<&str>,
    )
    .map_err(err)?;
    let hide = MenuItem::with_id(app, menu_id("hide", id), "隐藏便签", true, None::<&str>)
        .map_err(err)?;
    let delete = MenuItem::with_id(app, menu_id("delete", id), "移到回收站…", true, None::<&str>)
        .map_err(err)?;

    let sep1 = PredefinedMenuItem::separator(app).map_err(err)?;
    let sep2 = PredefinedMenuItem::separator(app).map_err(err)?;
    let sep3 = PredefinedMenuItem::separator(app).map_err(err)?;

    let mut items: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> = Vec::new();
    items.push(&edit);
    items.push(&lock);
    items.push(&sep1);
    items.push(&layer_menu);
    items.push(&category_menu);
    items.push(&sep2);

    // 文字便签：边框开关；图片便签：相框样式子菜单
    let border = if !is_image {
        Some(
            CheckMenuItem::with_id(
                app,
                menu_id("border", id),
                "显示边框",
                true,
                note.border.enabled,
                None::<&str>,
            )
            .map_err(err)?,
        )
    } else {
        None
    };

    let mut frame_items: Vec<CheckMenuItem<tauri::Wry>> = Vec::new();
    let frame_menu = if is_image {
        for (style_value, label) in [
            (FrameStyle::None, "无边框"),
            (FrameStyle::Thin, "细线框"),
            (FrameStyle::Rounded, "圆角框"),
            (FrameStyle::Polaroid, "拍立得白边框"),
            (FrameStyle::Gallery, "阴影画框"),
        ] {
            let item = CheckMenuItem::with_id(
                app,
                format!("note:frame:{}:{}", id, style_value.as_str()),
                label,
                true,
                note.frame.style == style_value,
                None::<&str>,
            )
            .map_err(err)?;
            frame_items.push(item);
        }
        let refs: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> =
            frame_items.iter().map(|i| i as &dyn tauri::menu::IsMenuItem<tauri::Wry>).collect();
        Some(Submenu::with_items(app, "相框样式", true, &refs).map_err(err)?)
    } else {
        None
    };

    let mut shadow_items: Vec<CheckMenuItem<tauri::Wry>> = Vec::new();
    for (preset, label) in [
        ("none", "无阴影"),
        ("soft", "柔和"),
        ("hard", "硬阴影"),
        ("deep", "深重"),
        ("neon", "霓虹发光"),
    ] {
        let item = CheckMenuItem::with_id(
            app,
            format!("note:shadow:{}:{}", id, preset),
            label,
            true,
            note.shadow.preset == preset,
            None::<&str>,
        )
        .map_err(err)?;
        shadow_items.push(item);
    }
    let shadow_refs: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> = shadow_items
        .iter()
        .map(|i| i as &dyn tauri::menu::IsMenuItem<tauri::Wry>)
        .collect();
    let shadow_menu = Submenu::with_items(app, "阴影", true, &shadow_refs).map_err(err)?;

    if let Some(border) = &border {
        items.push(border);
    }
    if let Some(frame_menu) = &frame_menu {
        items.push(frame_menu);
    }
    items.push(&shadow_menu);
    if let Some(restore_size) = &restore_size {
        items.push(restore_size);
    }
    if let Some(save_image) = &save_image {
        items.push(save_image);
    }
    items.push(&style);
    items.push(&sep3);
    items.push(&duplicate);
    items.push(&click_through);
    items.push(&hide);
    items.push(&delete);

    let menu = Menu::with_items(app, &items).map_err(err)?;
    let window = app
        .get_webview_window(&format!("note-{}", id))
        .ok_or_else(|| "便签窗口不存在".to_string())?;
    menu.popup(window.as_ref().window()).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn handle_note_menu_event(app: &AppHandle, menu_id: &str) {
    let parts: Vec<&str> = menu_id.split(':').collect();
    if parts.len() < 3 || parts[0] != "note" {
        return;
    }
    let action = parts[1];
    let note_id = parts[2];
    let extra = parts.get(3).copied();

    match action {
        "edit" => {
            if let Some(win) = app.get_webview_window(&format!("note-{note_id}")) {
                let _ = win.emit("start-edit", ());
            }
        }
        "lock" => {
            let _ = commands::mutate_note(app, note_id, |note| note.locked = !note.locked);
        }
        "layer-top" => {
            let _ = commands::mutate_note(app, note_id, |note| note.layer = Layer::Top);
        }
        "layer-desktop" => {
            let _ = commands::mutate_note(app, note_id, |note| note.layer = Layer::Desktop);
        }
        "layer-normal" => {
            let _ = commands::mutate_note(app, note_id, |note| note.layer = Layer::Normal);
        }
        "category" => {
            let target = extra.unwrap_or("").to_string();
            let valid = {
                let state = app.state::<crate::AppState>();
                state
                    .storage
                    .lock()
                    .map(|s| s.category_exists(&target))
                    .unwrap_or(false)
            };
            let category = if valid { target } else { String::new() };
            let _ = commands::mutate_note(app, note_id, |note| note.category = category.clone());
        }
        "border" => {
            let _ = commands::mutate_note(app, note_id, |note| {
                note.border.enabled = !note.border.enabled
            });
        }
        "frame" => {
            if let Some(style) = extra.and_then(parse_frame_style) {
                let _ = commands::mutate_note(app, note_id, |note| note.frame.style = style);
            }
        }
        "shadow" => {
            if let Some(preset) = extra {
                let shadow = crate::models::shadow_preset(preset);
                let _ = commands::mutate_note(app, note_id, |note| note.shadow = shadow.clone());
            }
        }
        "restore-size" => {
            let _ = commands::restore_image_size_quick(app, note_id);
        }
        "save-image" => {
            if let Some(win) = app.get_webview_window(&format!("note-{note_id}")) {
                let _ = win.emit("image-export-request", ());
            }
        }
        "style" => {
            let _ = commands::open_style_quick(app, note_id);
        }
        "duplicate" => {
            let _ = commands::duplicate_quick(app, note_id);
        }
        "clickthrough" => {
            let _ = commands::mutate_note(app, note_id, |note| {
                note.click_through = !note.click_through
            });
        }
        "hide" => {
            commands::hide_quick(app, note_id);
        }
        "delete" => {
            let app_handle = app.clone();
            let id = note_id.to_string();
            app.dialog()
                .message("确定把这张便签移进回收站吗？之后可以在管理窗口的回收站里恢复。")
                .title("移入回收站")
                .buttons(MessageDialogButtons::OkCancelCustom(
                    "删除".into(),
                    "取消".into(),
                ))
                .show(move |confirmed| {
                    if confirmed {
                        let _ = commands::delete_quick(&app_handle, &id);
                    }
                });
        }
        _ => {}
    }
}

fn parse_frame_style(value: &str) -> Option<FrameStyle> {
    match value {
        "none" => Some(FrameStyle::None),
        "thin" => Some(FrameStyle::Thin),
        "rounded" => Some(FrameStyle::Rounded),
        "polaroid" => Some(FrameStyle::Polaroid),
        "gallery" => Some(FrameStyle::Gallery),
        _ => None,
    }
}
