mod commands;
mod export;
mod images;
mod menu;
mod models;
mod storage;
mod tray;
mod window_layer;

use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager, RunEvent, WebviewWindow, WindowEvent};

use models::{Layer, Note};
use storage::Storage;

pub struct AppState {
    pub storage: Mutex<Storage>,
}

pub fn note_label(id: &str) -> String {
    format!("note-{id}")
}

pub fn broadcast_notes(app: &AppHandle) {
    let _ = app.emit("notes-changed", ());
}

pub fn broadcast_note(app: &AppHandle, note: &Note) {
    let _ = app.emit("note-updated", note);
}

/// 按数据里的位置 / 尺寸 / 层级摆放窗口
pub fn place_window(win: &WebviewWindow, note: &Note) {
    let (w, h) = note.window_size();
    if note.layer == Layer::Desktop {
        window_layer::ensure_desktop(win);
        window_layer::place_on_desktop(win, note.x, note.y, w, h);
    } else {
        let _ = win.set_size(tauri::LogicalSize::new(w, h));
        let _ = win.set_position(tauri::LogicalPosition::new(note.x, note.y));
    }
    let _ = win.set_ignore_cursor_events(note.click_through);
}

pub fn open_note_window(app: &AppHandle, note: &Note) -> Result<(), String> {
    open_note_window_ext(app, note, false)
}

/// flash=true 时在 URL 上带 `flash=1`：新窗口加载完成后由前端自己播定位高亮。
/// 旧做法是建窗线程还没跑完就 `emit_to("flash")`，事件落在窗口创建之前直接丢掉。
pub fn open_note_window_ext(app: &AppHandle, note: &Note, flash: bool) -> Result<(), String> {
    // 在 Windows 上同步创建 Webview 窗口会阻塞事件循环导致死锁，
    // 因此统一放到后台线程创建（Tauri 官方推荐的用法）。
    let app = app.clone();
    let note = note.clone();
    std::thread::spawn(move || {
        if let Err(err) = open_note_window_sync(&app, &note, flash) {
            eprintln!("[x-notes] {err}");
        }
    });
    Ok(())
}

fn open_note_window_sync(app: &AppHandle, note: &Note, flash: bool) -> Result<(), String> {
    if note.is_trashed() {
        return Ok(());
    }
    let label = note_label(&note.id);
    let (w, h) = note.window_size();

    if let Some(win) = app.get_webview_window(&label) {
        if window_layer::is_alive(&win) {
            place_window(&win, note);
            let _ = win.show();
            if flash {
                let _ = win.set_focus();
            }
            return Ok(());
        }
        let _ = win.destroy();
    }

    let url = if flash {
        format!("note.html?id={}&flash=1", note.id)
    } else {
        format!("note.html?id={}", note.id)
    };
    let win = tauri::WebviewWindowBuilder::new(
        app,
        &label,
        tauri::WebviewUrl::App(url.into()),
    )
    .title("便签")
    .inner_size(w, h)
    .position(note.x, note.y)
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .resizable(false)
    .skip_taskbar(true)
    .always_on_top(false)
    .visible(false)
    .build()
    .map_err(|e| format!("创建便签窗口失败：{e}"))?;

    window_layer::apply_layer(&win, note.layer);
    place_window(&win, note);
    let _ = win.show();
    Ok(())
}

/// 打开管理窗口并切到某张便签的样式面板（不再单独开窗）
pub fn open_style_panel_in_manager(app: &AppHandle, id: &str) -> Result<(), String> {
    let Some(win) = app.get_webview_window("main") else {
        return Err("管理窗口未就绪".into());
    };
    show_manager(app);
    win.emit("open-style-panel", id.to_string())
        .map_err(|e| format!("打开样式面板失败：{e}"))
}

/// 便签所在显示器被拔掉时，挪回主屏可见区域；返回是否真的挪动过
pub fn ensure_on_screen(app: &AppHandle, note: &mut Note) -> bool {
    let (w, h) = note.window_size();
    let monitors = app.available_monitors().unwrap_or_default();
    if monitors.is_empty() {
        return false;
    }
    let visible = monitors.iter().any(|m| {
        let s = m.scale_factor();
        let p = m.position();
        let sz = m.size();
        let mx = p.x as f64 / s;
        let my = p.y as f64 / s;
        let mw = sz.width as f64 / s;
        let mh = sz.height as f64 / s;
        note.x + w > mx + 8.0
            && note.x < mx + mw - 8.0
            && note.y + h > my + 8.0
            && note.y < my + mh - 8.0
    });
    if visible {
        return false;
    }
    let primary = app
        .primary_monitor()
        .ok()
        .flatten()
        .or_else(|| monitors.first().cloned());
    if let Some(m) = primary {
        let s = m.scale_factor();
        let p = m.position();
        let sz = m.size();
        let mx = p.x as f64 / s;
        let my = p.y as f64 / s;
        let mw = sz.width as f64 / s;
        let mh = sz.height as f64 / s;
        note.x = (mx + 40.0).min((mx + mw - w - 40.0).max(mx));
        note.y = (my + 40.0).min((my + mh - h - 40.0).max(my));
        return true;
    }
    false
}

/// 记录管理窗口尺寸（逻辑像素），只在合法范围内生效
pub fn remember_manager_size(app: &AppHandle, w: f64, h: f64) {
    let w = w.round();
    let h = h.round();
    if !(320.0..=2000.0).contains(&w) || !(400.0..=2000.0).contains(&h) {
        return;
    }
    if let Ok(mut storage) = app.state::<AppState>().storage.lock() {
        let s = &mut storage.settings;
        if (s.manager_width - w).abs() > 0.5 || (s.manager_height - h).abs() > 0.5 {
            s.manager_width = w;
            s.manager_height = h;
        }
    }
}

/// 把管理窗口当前尺寸和位置写回设置（关闭 / 退出时调用）
pub fn remember_manager_now(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let scale = win.scale_factor().unwrap_or(1.0);
        if let Ok(size) = win.inner_size() {
            remember_manager_size(app, size.width as f64 / scale, size.height as f64 / scale);
        }
        if let Ok(pos) = win.outer_position() {
            remember_manager_pos(app, pos.x as f64 / scale, pos.y as f64 / scale);
        }
    }
}

/// 记录管理窗口位置（逻辑像素）；只接受有限值，异常坐标直接丢弃
pub fn remember_manager_pos(app: &AppHandle, x: f64, y: f64) {
    if !x.is_finite() || !y.is_finite() {
        return;
    }
    let x = x.round();
    let y = y.round();
    if x.abs() > 100_000.0 || y.abs() > 100_000.0 {
        return;
    }
    if let Ok(mut storage) = app.state::<AppState>().storage.lock() {
        let s = &mut storage.settings;
        if s.manager_x != Some(x) || s.manager_y != Some(y) {
            s.manager_x = Some(x);
            s.manager_y = Some(y);
        }
    }
}

/// 管理窗口位置是否还落在某块显示器上（拔掉外接屏后不要恢复到看不见的地方）
fn manager_pos_on_screen(app: &AppHandle, x: f64, y: f64, w: f64, h: f64) -> bool {
    let monitors = app.available_monitors().unwrap_or_default();
    monitors.iter().any(|m| {
        let s = m.scale_factor();
        let p = m.position();
        let sz = m.size();
        let mx = p.x as f64 / s;
        let my = p.y as f64 / s;
        let mw = sz.width as f64 / s;
        let mh = sz.height as f64 / s;
        x + w > mx + 8.0 && x < mx + mw - 8.0 && y + h > my + 8.0 && y < my + mh - 8.0
    })
}

pub fn show_manager(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}

pub fn save_all(app: &AppHandle) {
    let state = app.state::<AppState>();
    let mut storage = match state.storage.lock() {
        Ok(storage) => storage,
        Err(_) => return,
    };
    let _ = storage.save_notes();
    let _ = storage.save_settings();
}

pub fn quit_app(app: &AppHandle) {
    save_all(app);
    app.exit(0);
}

pub fn apply_autostart(app: &AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;

    let autolaunch = app.autolaunch();
    let current = autolaunch
        .is_enabled()
        .map_err(|e| format!("读取开机自启状态失败：{e}"))?;

    if enabled && !current {
        autolaunch
            .enable()
            .map_err(|e| format!("开启开机自启失败：{e}"))?;
    } else if !enabled && current {
        autolaunch
            .disable()
            .map_err(|e| format!("关闭开机自启失败：{e}"))?;
    }
    Ok(())
}

pub fn register_hotkeys(app: &AppHandle, enable: bool) -> Result<(), String> {
    use tauri_plugin_global_shortcut::{
        Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
    };

    let shortcuts = app.global_shortcut();
    let _ = shortcuts.unregister_all();
    if !enable {
        return Ok(());
    }

    let new_note = Shortcut::new(
        Some(Modifiers::CONTROL | Modifiers::ALT),
        Code::KeyN,
    );
    let toggle = Shortcut::new(
        Some(Modifiers::CONTROL | Modifiers::ALT),
        Code::KeyH,
    );

    let register_new = shortcuts.on_shortcut(new_note, |app, _shortcut, event| {
        if event.state() == ShortcutState::Pressed {
            let _ = commands::create_text_note_quick(app);
        }
    });
    if let Err(e) = register_new {
        let _ = shortcuts.unregister_all();
        return Err(format!(
            "全局快捷键 Ctrl+Alt+N 注册失败（可能被其他程序占用）：{e}"
        ));
    }

    let register_toggle = shortcuts.on_shortcut(toggle, |app, _shortcut, event| {
        if event.state() == ShortcutState::Pressed {
            commands::toggle_visible_quick(app);
        }
    });
    if let Err(e) = register_toggle {
        let _ = shortcuts.unregister_all();
        return Err(format!(
            "全局快捷键 Ctrl+Alt+H 注册失败（可能被其他程序占用）：{e}"
        ));
    }
    Ok(())
}

fn spawn_desktop_watchdog(app: AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(1500));

        let notes: Vec<Note> = {
            let state = app.state::<AppState>();
            let storage = match state.storage.lock() {
                Ok(storage) => storage,
                Err(_) => continue,
            };
            storage
                .notes
                .iter()
                .filter(|n| n.visible && n.layer == Layer::Desktop && !n.is_trashed())
                .cloned()
                .collect()
        };

        for note in notes {
            let label = note_label(&note.id);
            match app.get_webview_window(&label) {
                Some(win) => {
                    if !window_layer::is_alive(&win) {
                        // 资源管理器重启会连带销毁挂在桌面下的子窗口，这里重建
                        let _ = win.destroy();
                        let _ = open_note_window(&app, &note);
                    } else {
                        window_layer::ensure_desktop(&win);
                    }
                }
                None => {
                    let _ = open_note_window(&app, &note);
                }
            }
        }
    });
}

#[cfg(windows)]
struct SingleInstanceGuard(windows_sys::Win32::Foundation::HANDLE);

#[cfg(windows)]
impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                windows_sys::Win32::Foundation::CloseHandle(self.0);
            }
        }
    }
}

/// 用命名互斥体避免双开。两个实例共用同一个 WebView2 数据目录时会互相破坏。
/// 已有一个实例时返回 Err；互斥体创建失败则放行，不阻止用户使用。
#[cfg(windows)]
fn acquire_single_instance(silent: bool) -> Result<SingleInstanceGuard, ()> {
    use windows_sys::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS};
    use windows_sys::Win32::System::Threading::CreateMutexW;
    use windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW;

    let name: Vec<u16> = "Local\\XNotes.SingleInstance.v1\0"
        .encode_utf16()
        .collect();
    unsafe {
        let handle = CreateMutexW(std::ptr::null(), 1, name.as_ptr());
        if handle.is_null() {
            return Ok(SingleInstanceGuard(handle));
        }
        if windows_sys::Win32::Foundation::GetLastError() == ERROR_ALREADY_EXISTS {
            CloseHandle(handle);
            if !silent {
                let text: Vec<u16> = "X Notes 已经在运行。\n请从系统托盘图标打开管理窗口。\0"
                    .encode_utf16()
                    .collect();
                let caption: Vec<u16> = "X Notes\0".encode_utf16().collect();
                MessageBoxW(
                    std::ptr::null_mut(),
                    text.as_ptr(),
                    caption.as_ptr(),
                    0x0000_0040, // MB_ICONINFORMATION | MB_OK
                );
            }
            return Err(());
        }
        Ok(SingleInstanceGuard(handle))
    }
}

#[cfg(not(windows))]
struct SingleInstanceGuard;

#[cfg(not(windows))]
fn acquire_single_instance(_silent: bool) -> Result<SingleInstanceGuard, ()> {
    Ok(SingleInstanceGuard)
}

pub fn handle_menu_event(app: &AppHandle, menu_id: &str) {
    match menu_id {
        "tray:new" => {
            let _ = commands::create_text_note_quick(app);
        }
        "tray:add-image" => commands::add_image_quick(app),
        "tray:show-all" => {
            let state = app.state::<AppState>();
            let _ = commands::set_all_visible(app.clone(), state, true);
        }
        "tray:hide-all" => {
            let state = app.state::<AppState>();
            let _ = commands::set_all_visible(app.clone(), state, false);
        }
        "tray:manager" => show_manager(app),
        "tray:settings" => commands::request_settings(app),
        "tray:quit" => quit_app(app),
        other => {
            if other.starts_with("note:") {
                menu::handle_note_menu_event(app, other);
            }
        }
    }
}

pub fn run() {
    let autostart_launch = std::env::args().any(|a| a == "--autostart");
    let _single_instance = match acquire_single_instance(autostart_launch) {
        Ok(guard) => guard,
        Err(()) => return,
    };

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            commands::list_notes,
            commands::get_note,
            commands::list_categories,
            commands::create_category,
            commands::rename_category,
            commands::delete_category,
            commands::reorder_categories,
            commands::get_settings,
            commands::save_settings,
            commands::take_warning,
            commands::create_note,
            commands::update_note,
            commands::resize_note,
            commands::restore_image_size,
            commands::set_click_through,
            commands::hide_note,
            commands::show_note,
            commands::set_all_visible,
            commands::toggle_all_visible,
            commands::delete_note,
            commands::trash_notes,
            commands::restore_notes,
            commands::purge_notes,
            commands::empty_trash,
            commands::set_notes_visible,
            commands::set_notes_locked,
            commands::set_notes_category,
            commands::duplicate_note,
            commands::locate_note,
            commands::apply_shadow_to_all,
            commands::import_image,
            commands::import_image_clipboard,
            commands::read_image,
            commands::read_thumbnail,
            commands::export_image,
            commands::open_data_folder,
            commands::get_data_dir,
            commands::export_all_data,
            commands::import_all_data,
            commands::open_style_panel,
            commands::show_note_menu,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();

            let storage = Storage::load(&handle);
            let hotkeys_enabled = storage.settings.global_hotkeys_enabled;
            app.manage(AppState {
                storage: Mutex::new(storage),
            });

            tray::create_tray(&handle)?;
            if let Err(err) = register_hotkeys(&handle, hotkeys_enabled) {
                if let Ok(mut storage) = app.state::<AppState>().storage.lock() {
                    storage.warning = Some(err);
                }
            }
            spawn_desktop_watchdog(handle.clone());

            // 恢复上次显示中的便签；顺带修正因为拔掉显示器而落到屏幕外的位置
            let notes = {
                let state = app.state::<AppState>();
                let mut guard = match state.storage.lock() {
                    Ok(storage) => storage,
                    Err(_) => return Ok(()),
                };
                let mut moved = false;
                for note in &mut guard.notes {
                    moved |= ensure_on_screen(&handle, note);
                }
                // 只有坐标真的被修正过才落盘，正常启动不再无谓写一次文件
                if moved {
                    let _ = guard.save_notes();
                }
                guard.notes.clone()
            };
            for note in notes.iter().filter(|n| n.visible && !n.is_trashed()) {
                let _ = open_note_window(&handle, note);
            }

            // 恢复管理窗口上次关闭时的尺寸和位置
            let (mw, mh, pos) = {
                let state = app.state::<AppState>();
                let guard = state.storage.lock();
                match guard {
                    Ok(storage) => (
                        storage.settings.manager_width,
                        storage.settings.manager_height,
                        storage.settings.manager_x.zip(storage.settings.manager_y),
                    ),
                    Err(_) => (360.0, 600.0, None),
                }
            };
            if let Some(win) = handle.get_webview_window("main") {
                let _ = win.set_min_size(Some(tauri::LogicalSize::new(320.0, 400.0)));
                let _ = win.set_size(tauri::LogicalSize::new(mw, mh));
                // 位置还被某块显示器包含时才恢复，否则保持 tauri.conf 里的居中
                if let Some((x, y)) = pos {
                    if manager_pos_on_screen(&handle, x, y, mw, mh) {
                        let _ = win.set_position(tauri::LogicalPosition::new(x, y));
                    }
                }
            }

            // 开机自启时不弹主窗口，只恢复便签和托盘
            if !autostart_launch {
                show_manager(&handle);
            }

            Ok(())
        })
        .on_menu_event(|app, event| {
            handle_menu_event(app, event.id().as_ref());
        })
        .on_window_event(|window, event| {
            match event {
                WindowEvent::CloseRequested { api, .. } => {
                    let label = window.label().to_string();
                    if label == "main" {
                        // 记住关掉时的尺寸，下次打开仍是这个大小
                        let app = window.app_handle();
                        remember_manager_now(app);
                        if let Ok(mut storage) = app.state::<AppState>().storage.lock() {
                            let _ = storage.save_settings();
                        }
                        api.prevent_close();
                        let _ = window.hide();
                    } else if let Some(note_id) = label.strip_prefix("note-") {
                        api.prevent_close();
                        let app = window.app_handle();
                        let state = app.state::<AppState>();
                        let _ = commands::hide_note_internal(app, &state, note_id);
                    }
                }
                WindowEvent::Resized(_) => {}
                WindowEvent::Destroyed => {
                    // 便签窗口关闭后无需额外处理
                }
                _ => {}
            }
        });

    let app = builder
        .build(tauri::generate_context!())
        .expect("启动便签程序失败");
    app.run(|app_handle, event| match event {
        RunEvent::ExitRequested { .. } => save_exit_once(app_handle),
        RunEvent::Exit => save_exit_once(app_handle),
        _ => {}
    });
}

/// ExitRequested 与 Exit 会连续触发两次，用标记保证退出前只写一次盘
static EXIT_SAVED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn save_exit_once(app_handle: &AppHandle) {
    if EXIT_SAVED.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return;
    }
    remember_manager_now(app_handle);
    save_all(app_handle);
}
