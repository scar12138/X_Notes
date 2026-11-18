//! 系统托盘

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::AppHandle;

pub fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    let new_note = MenuItem::with_id(app, "tray:new", "新建文字便签", true, None::<&str>)?;
    let add_image = MenuItem::with_id(app, "tray:add-image", "新建图片便签…", true, None::<&str>)?;
    let show_all = MenuItem::with_id(app, "tray:show-all", "全部显示", true, None::<&str>)?;
    let hide_all = MenuItem::with_id(app, "tray:hide-all", "全部隐藏", true, None::<&str>)?;
    let open_manager = MenuItem::with_id(app, "tray:manager", "打开管理窗口", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "tray:settings", "设置…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "tray:quit", "退出", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let sep3 = PredefinedMenuItem::separator(app)?;

    let menu = Menu::with_items(
        app,
        &[
            &new_note,
            &add_image,
            &sep1,
            &show_all,
            &hide_all,
            &sep2,
            &open_manager,
            &settings,
            &sep3,
            &quit,
        ],
    )?;

    // 菜单事件统一交给 Builder::on_menu_event 处理。
    // TrayIconBuilder::on_menu_event 注册的也是全局菜单监听器，
    // 两处都注册会让“新建便签”等托盘动作执行两次。
    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("X Notes")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                crate::show_manager(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    Ok(())
}
