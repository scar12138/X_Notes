//! Windows 原生窗口层级处理。
//!
//! 三种模式：
//! - top      : HWND_TOPMOST，始终在最前
//! - normal   : HWND_NOTOPMOST，参与普通窗口层叠
//! - desktop  : 把窗口挂到桌面宿主窗口（Progman / WorkerW）下，成为其子窗口。
//!              这样它会被普通窗口盖住，按 Win+D 显示桌面时仍然可见，
//!              资源管理器重启后由 watchdog 自动重新挂载。

#![cfg(windows)]

use tauri::WebviewWindow;
use windows_sys::Win32::Foundation::{HWND, LPARAM, POINT};
use windows_sys::Win32::Graphics::Gdi::ScreenToClient;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, FindWindowExW, FindWindowW, GetParent, GetWindowLongPtrW, IsWindow, SetParent,
    SetWindowLongPtrW, SetWindowPos, ShowWindow,
};

const GWL_STYLE: i32 = -16;
const GWL_EXSTYLE: i32 = -20;

const WS_CHILD: isize = 0x4000_0000;
const WS_POPUP: isize = 0x8000_0000u32 as isize;

const WS_EX_NOACTIVATE: isize = 0x0800_0000;
const WS_EX_TOOLWINDOW: isize = 0x0000_0080;

const SWP_NOSIZE: u32 = 0x0001;
const SWP_NOMOVE: u32 = 0x0002;
const SWP_NOACTIVATE: u32 = 0x0010;
const SWP_SHOWWINDOW: u32 = 0x0040;
const SWP_FRAMECHANGED: u32 = 0x0020;
const SWP_NOOWNERZORDER: u32 = 0x0200;

const HWND_BOTTOM_RAW: isize = 1;
const HWND_TOP_RAW: isize = 0;
const HWND_TOPMOST_RAW: isize = -1;
const HWND_NOTOPMOST_RAW: isize = -2;

const SW_SHOWNOACTIVATE: i32 = 4;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe fn raw_hwnd(win: &WebviewWindow) -> Option<HWND> {
    let h = win.hwnd().ok()?.0 as isize;
    if h == 0 {
        None
    } else {
        Some(h as HWND)
    }
}

struct HostSearch {
    found: HWND,
}

unsafe extern "system" fn host_enum_proc(hwnd: HWND, lparam: LPARAM) -> i32 {
    let ctx = &mut *(lparam as *mut HostSearch);
    let class = wide("SHELLDLL_DefView");
    let child = FindWindowExW(hwnd, 0 as HWND, class.as_ptr(), std::ptr::null());
    if !child.is_null() {
        ctx.found = hwnd;
        return 0; // 停止枚举
    }
    1
}

/// 找到承载桌面图标的宿主窗口（Progman 或 WorkerW）
pub fn desktop_host() -> HWND {
    unsafe {
        let progman = FindWindowW(wide("Progman").as_ptr(), std::ptr::null());
        if !progman.is_null() {
            let def_view = FindWindowExW(
                progman,
                0 as HWND,
                wide("SHELLDLL_DefView").as_ptr(),
                std::ptr::null(),
            );
            if !def_view.is_null() {
                let parent = GetParent(def_view);
                if !parent.is_null() {
                    return parent;
                }
                return progman;
            }
        }
        let mut ctx = HostSearch { found: std::ptr::null_mut() };
        let _ = EnumWindows(Some(host_enum_proc), &mut ctx as *mut _ as LPARAM);
        ctx.found
    }
}

pub fn is_alive(win: &WebviewWindow) -> bool {
    unsafe {
        match raw_hwnd(win) {
            Some(hwnd) => IsWindow(hwnd) != 0,
            None => false,
        }
    }
}

fn set_ex_style(hwnd: HWND, add: isize, remove: isize) {
    unsafe {
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let next = (ex | add) & !remove;
        if next != ex {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, next);
        }
    }
}

fn set_style(hwnd: HWND, add: isize, remove: isize) {
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        let next = (style | add) & !remove;
        if next != style {
            SetWindowLongPtrW(hwnd, GWL_STYLE, next);
        }
    }
}

/// 把窗口从桌面宿主上摘下来，恢复成普通顶层窗口
fn detach(hwnd: HWND) {
    unsafe {
        if (GetWindowLongPtrW(hwnd, GWL_STYLE) & WS_CHILD) != 0 {
            SetParent(hwnd, 0 as HWND);
            set_style(hwnd, WS_POPUP, WS_CHILD);
            SetWindowPos(
                hwnd,
                HWND_NOTOPMOST_RAW as HWND,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
        }
    }
}

/// 挂到桌面宿主；返回是否成功
fn attach(hwnd: HWND) -> bool {
    let host = desktop_host();
    if host.is_null() {
        return false;
    }
    unsafe {
        set_style(hwnd, WS_CHILD, WS_POPUP);
        set_ex_style(hwnd, WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW, 0);
        SetParent(hwnd, host);
        SetWindowPos(
            hwnd,
            HWND_TOP_RAW as HWND,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW | SWP_FRAMECHANGED,
        );
        ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        GetParent(hwnd) == host
    }
}

/// 桌面模式下按屏幕逻辑坐标摆放子窗口
pub fn place_on_desktop(win: &WebviewWindow, x: f64, y: f64, w: f64, h: f64) -> bool {
    unsafe {
        let hwnd = match raw_hwnd(win) {
            Some(h) => h,
            None => return false,
        };
        let host = GetParent(hwnd);
        if host.is_null() {
            return false;
        }
        let scale = win.scale_factor().unwrap_or(1.0);
        let mut pt = POINT {
            x: (x * scale).round() as i32,
            y: (y * scale).round() as i32,
        };
        // 屏幕坐标 -> 宿主客户区坐标
        ScreenToClient(host, &mut pt);
        let pw = (w * scale).round() as i32;
        let ph = (h * scale).round() as i32;
        SetWindowPos(
            hwnd,
            HWND_TOP_RAW as HWND,
            pt.x,
            pt.y,
            pw,
            ph,
            SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_SHOWWINDOW,
        );
        true
    }
}

/// 应用层级设置。返回实际生效的模式（desktop 挂载失败时会退回到普通层）。
pub fn apply_layer(win: &WebviewWindow, layer: crate::models::Layer) -> crate::models::Layer {
    use crate::models::Layer;
    unsafe {
        let hwnd = match raw_hwnd(win) {
            Some(h) => h,
            None => return layer,
        };
        // 所有便签窗口都不出现在任务栏 / Alt+Tab
        set_ex_style(hwnd, WS_EX_TOOLWINDOW, 0);

        match layer {
            Layer::Top => {
                detach(hwnd);
                set_ex_style(hwnd, 0, WS_EX_NOACTIVATE);
                let _ = win.set_always_on_top(true);
                SetWindowPos(
                    hwnd,
                    HWND_TOPMOST_RAW as HWND,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
                );
                Layer::Top
            }
            Layer::Normal => {
                detach(hwnd);
                set_ex_style(hwnd, 0, WS_EX_NOACTIVATE);
                let _ = win.set_always_on_top(false);
                SetWindowPos(
                    hwnd,
                    HWND_NOTOPMOST_RAW as HWND,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
                );
                Layer::Normal
            }
            Layer::Desktop => {
                let _ = win.set_always_on_top(false);
                if attach(hwnd) {
                    Layer::Desktop
                } else {
                    // 宿主导不到时退回“普通窗口 + 压到最底层”，至少不挡工作窗口
                    detach(hwnd);
                    set_ex_style(hwnd, WS_EX_NOACTIVATE, 0);
                    SetWindowPos(
                        hwnd,
                        HWND_BOTTOM_RAW as HWND,
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
                    );
                    Layer::Normal
                }
            }
        }
    }
}

/// 桌面模式下检查挂载关系是否还有效，失效则重新挂载
pub fn ensure_desktop(win: &WebviewWindow) -> bool {
    unsafe {
        let hwnd = match raw_hwnd(win) {
            Some(h) => h,
            None => return false,
        };
        if IsWindow(hwnd) == 0 {
            return false;
        }
        let host = desktop_host();
        if host.is_null() {
            return false;
        }
        if GetParent(hwnd) != host {
            return attach(hwnd);
        }
        // 已经被别的窗口盖到下面（例如资源管理器重启、Win+D 后）时重新抬到宿主顶部
        SetWindowPos(
            hwnd,
            HWND_TOP_RAW as HWND,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
        );
        true
    }
}

