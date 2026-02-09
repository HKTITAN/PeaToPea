//! System tray icon and menu (Enable / Disable / Exit). Sends commands to main via channel.
//! Tooltip shows state (enabled/disabled) and "Pod: N devices"; main sends TrayStateUpdate and posts WM_TRAY_UPDATE_STATE.

#![cfg(windows)]

use std::ptr::null_mut;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::Mutex;

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use windows::core::w;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HINSTANCE, HMENU, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::GetCursorPos;
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::LoadIconW;
use windows::Win32::UI::WindowsAndMessaging::*;

pub enum TrayCommand {
    Enable,
    Disable,
    OpenSettings,
    SetAutostart(bool),
    Exit,
}

/// State for tooltip and settings: enabled/disabled, peer count, peer device IDs, and autostart.
#[derive(Clone, Debug)]
pub struct TrayStateUpdate {
    pub enabled: bool,
    pub peer_count: u32,
    /// Device IDs of current peers (first 16 bytes each); used by settings window to list pod members.
    pub peer_ids: Vec<[u8; 16]>,
    /// Start PeaPod when I sign in (§7.2).
    pub autostart_enabled: bool,
}

const WM_TRAYICON: u32 = WM_USER + 1;
/// Posted by main to tell the tray thread to drain state_rx and update the tooltip.
pub const WM_TRAY_UPDATE_STATE: u32 = WM_USER + 2;
/// Posted by main when user chose Open settings; tray creates/shows the settings window.
pub const WM_SHOW_SETTINGS: u32 = WM_USER + 3;
const TRAY_ID: u32 = 1;

/// Control IDs for the settings window.
const IDC_CHECK_ENABLED: i32 = 101;
const IDC_LIST_PEERS: i32 = 102;
const IDC_STATIC_PROXY: i32 = 103;
const IDC_CHECK_AUTOSTART: i32 = 104;

// Standard Win32 control styles/messages (not all in windows crate default features).
const BS_AUTOCHECKBOX: u32 = 0x0003;
const BST_CHECKED: i32 = 1;
const LB_ADDSTRING: u32 = 0x0180;
const LB_RESETCONTENT: u32 = 0x0184;
const LBS_NOTIFY: u32 = 0x0001;
const BM_SETCHECK: u32 = 0x00F1;
const BM_GETCHECK: u32 = 0x00F0;

static CMD_TX: AtomicPtr<()> = AtomicPtr::new(null_mut());
static STATE_RX: Mutex<Option<UnboundedReceiver<TrayStateUpdate>>> = Mutex::new(None);
/// Latest state (including peer_ids) for the settings window to read.
static LATEST_STATE: Mutex<Option<TrayStateUpdate>> = Mutex::new(None);
static mut NID_PTR: *mut NOTIFYICONDATAW = null_mut();
static mut SETTINGS_HWND: HWND = HWND(std::ptr::null_mut());

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_TRAYICON {
        if lparam.0 as u32 == WM_RBUTTONUP {
            let menu = CreatePopupMenu().unwrap();
            let _ = AppendMenuW(menu, MF_STRING, 1, w!("Enable"));
            let _ = AppendMenuW(menu, MF_STRING, 2, w!("Disable"));
            let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
            let _ = AppendMenuW(menu, MF_STRING, 3, w!("Open settings"));
            let _ = AppendMenuW(menu, MF_STRING, 4, w!("Exit"));
            let mut pt = std::mem::zeroed();
            let _ = GetCursorPos(&mut pt);
            SetForegroundWindow(hwnd);
            let _ = TrackPopupMenuEx(
                menu,
                TPM_RIGHTALIGN | TPM_BOTTOMALIGN | TPM_NONACTIVATE,
                pt.x,
                pt.y,
                hwnd,
                None,
            );
            let _ = DestroyMenu(menu);
        }
        return LRESULT(0);
    }
    if msg == WM_COMMAND {
        let id = (wparam.0 & 0xFFFF) as u32;
        let tx_ptr = CMD_TX.load(Ordering::Acquire);
        if !tx_ptr.is_null() {
            let tx = &*(tx_ptr as *const UnboundedSender<TrayCommand>);
            let cmd = match id {
                1 => TrayCommand::Enable,
                2 => TrayCommand::Disable,
                3 => TrayCommand::OpenSettings,
                4 => TrayCommand::Exit,
                _ => return DefWindowProcW(hwnd, msg, wparam, lparam),
            };
            let is_exit = matches!(cmd, TrayCommand::Exit);
            let _ = tx.send(cmd);
            if is_exit {
                PostQuitMessage(0);
            }
        }
        return LRESULT(0);
    }
    if msg == WM_TRAY_UPDATE_STATE {
        if let Ok(mut guard) = STATE_RX.lock() {
            if let Some(rx) = guard.as_mut() {
                let mut latest = None;
                while let Ok(s) = rx.try_recv() {
                    latest = Some(s);
                }
                if let Some(s) = latest {
                    if let Ok(mut latest_guard) = LATEST_STATE.lock() {
                        *latest_guard = Some(s.clone());
                    }
                    let tip = format!(
                        "PeaPod – {}\r\nPod: {} device(s)",
                        if s.enabled { "enabled" } else { "disabled" },
                        s.peer_count
                    );
                    let tip_wide: Vec<u16> = tip.encode_utf16().chain(std::iter::once(0)).collect();
                    let len = tip_wide.len().min(128);
                    if !NID_PTR.is_null() {
                        let nid = &mut *NID_PTR;
                        nid.szTip[..len].copy_from_slice(&tip_wide[..len]);
                        let _ = Shell_NotifyIconW(NIM_MODIFY, nid);
                    }
                }
            }
        }
        return LRESULT(0);
    }
    if msg == WM_SHOW_SETTINGS {
        create_or_show_settings_window(hwnd);
        return LRESULT(0);
    }
    if msg == WM_DESTROY {
        PostQuitMessage(0);
        return LRESULT(0);
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

unsafe fn create_or_show_settings_window(tray_hwnd: HWND) {
    use windows::Win32::UI::WindowsAndMessaging::IsWindow;
    if !SETTINGS_HWND.0.is_null() && IsWindow(SETTINGS_HWND).as_bool() {
        let _ = ShowWindow(SETTINGS_HWND, SW_SHOW);
        SetForegroundWindow(SETTINGS_HWND);
        refresh_settings_peer_list();
        return;
    }
    let instance = match GetModuleHandleW(None) {
        Ok(i) => i,
        Err(_) => return,
    };
    let class_name = w!("PeaPodSettings");
    let sw = CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        class_name,
        w!("PeaPod Settings"),
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
        100,
        100,
        380,
        280,
        Some(tray_hwnd),
        None,
        Some(HINSTANCE(instance.0)),
        None,
    );
    if let Ok(hwnd) = sw {
        SETTINGS_HWND = hwnd;
        let _ = ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd);
        refresh_settings_peer_list();
    }
}

unsafe fn refresh_settings_peer_list() {
    if SETTINGS_HWND.0.is_null() {
        return;
    }
    let list = match GetDlgItem(SETTINGS_HWND, IDC_LIST_PEERS) {
        Ok(h) => h,
        Err(_) => return,
    };
    let _ = SendMessageW(list, LB_RESETCONTENT, WPARAM(0), LPARAM(0));
    if let Ok(guard) = LATEST_STATE.lock() {
        if let Some(ref s) = *guard {
            for id in &s.peer_ids {
                let hex = format!("{:02x}{:02x}{:02x}{:02x}...", id[0], id[1], id[2], id[3]);
                let wide: Vec<u16> = hex.encode_utf16().chain(std::iter::once(0)).collect();
                let _ = SendMessageW(
                    list,
                    LB_ADDSTRING,
                    WPARAM(0),
                    LPARAM(wide.as_ptr() as isize),
                );
            }
        }
    }
}

unsafe extern "system" fn settings_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_CREATE {
        let instance = GetModuleHandleW(None).unwrap();
        let hinstance = HINSTANCE(instance.0);
        let _ = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("BUTTON"),
            w!("PeaPod enabled"),
            WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | BS_AUTOCHECKBOX),
            16,
            16,
            200,
            24,
            hwnd,
            Some(HMENU(IDC_CHECK_ENABLED as _)),
            Some(hinstance),
            None,
        );
        let _ = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("BUTTON"),
            w!("Start PeaPod when I sign in"),
            WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | BS_AUTOCHECKBOX),
            16,
            40,
            260,
            24,
            hwnd,
            Some(HMENU(IDC_CHECK_AUTOSTART as _)),
            Some(hinstance),
            None,
        );
        let _ = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!("Proxy: 127.0.0.1:3128"),
            WS_CHILD | WS_VISIBLE,
            16,
            68,
            300,
            20,
            hwnd,
            Some(HMENU(IDC_STATIC_PROXY as _)),
            Some(hinstance),
            None,
        );
        let _ = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("LISTBOX"),
            PCWSTR::null(),
            WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | WS_BORDER.0 | LBS_NOTIFY),
            16,
            92,
            340,
            168,
            hwnd,
            Some(HMENU(IDC_LIST_PEERS as _)),
            Some(hinstance),
            None,
        );
        if let Ok(guard) = LATEST_STATE.lock() {
            if let Some(ref s) = *guard {
                if let Ok(check) = GetDlgItem(hwnd, IDC_CHECK_ENABLED) {
                    let _ = SendMessageW(
                        check,
                        BM_SETCHECK,
                        if s.enabled {
                            WPARAM(BST_CHECKED as _)
                        } else {
                            WPARAM(0)
                        },
                        LPARAM(0),
                    );
                }
                if let Ok(autostart) = GetDlgItem(hwnd, IDC_CHECK_AUTOSTART) {
                    let _ = SendMessageW(
                        autostart,
                        BM_SETCHECK,
                        if s.autostart_enabled {
                            WPARAM(BST_CHECKED as _)
                        } else {
                            WPARAM(0)
                        },
                        LPARAM(0),
                    );
                }
            }
        }
        return LRESULT(0);
    }
    if msg == WM_SHOWWINDOW {
        if wparam.0 != 0 {
            refresh_settings_peer_list();
            if let Ok(guard) = LATEST_STATE.lock() {
                if let Some(ref s) = *guard {
                    if let Ok(check) = GetDlgItem(hwnd, IDC_CHECK_ENABLED) {
                        let _ = SendMessageW(
                            check,
                            BM_SETCHECK,
                            if s.enabled {
                                WPARAM(BST_CHECKED as _)
                            } else {
                                WPARAM(0)
                            },
                            LPARAM(0),
                        );
                    }
                    if let Ok(autostart) = GetDlgItem(hwnd, IDC_CHECK_AUTOSTART) {
                        let _ = SendMessageW(
                            autostart,
                            BM_SETCHECK,
                            if s.autostart_enabled {
                                WPARAM(BST_CHECKED as _)
                            } else {
                                WPARAM(0)
                            },
                            LPARAM(0),
                        );
                    }
                }
            }
        }
        return LRESULT(0);
    }
    if msg == WM_COMMAND {
        let id = (wparam.0 & 0xFFFF) as i32;
        if id == IDC_CHECK_ENABLED {
            if let Ok(check) = GetDlgItem(hwnd, IDC_CHECK_ENABLED) {
                let state = SendMessageW(check, BM_GETCHECK, WPARAM(0), LPARAM(0));
                let enabled = state.0 == BST_CHECKED as _;
                let tx_ptr = CMD_TX.load(Ordering::Acquire);
                if !tx_ptr.is_null() {
                    let tx = &*(tx_ptr as *const UnboundedSender<TrayCommand>);
                    let _ = tx.send(if enabled {
                        TrayCommand::Enable
                    } else {
                        TrayCommand::Disable
                    });
                }
            }
        } else if id == IDC_CHECK_AUTOSTART {
            if let Ok(check) = GetDlgItem(hwnd, IDC_CHECK_AUTOSTART) {
                let state = SendMessageW(check, BM_GETCHECK, WPARAM(0), LPARAM(0));
                let enabled = state.0 == BST_CHECKED as _;
                let tx_ptr = CMD_TX.load(Ordering::Acquire);
                if !tx_ptr.is_null() {
                    let tx = &*(tx_ptr as *const UnboundedSender<TrayCommand>);
                    let _ = tx.send(TrayCommand::SetAutostart(enabled));
                }
            }
        }
        return LRESULT(0);
    }
    if msg == WM_DESTROY {
        SETTINGS_HWND = HWND(std::ptr::null_mut());
        return LRESULT(0);
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

/// Run the tray icon and message loop in the current thread. Sends commands via `cmd_tx`.
/// Receives tooltip state updates on `state_rx`; when main posts WM_TRAY_UPDATE_STATE, tooltip is updated.
/// Sends `hwnd` on `hwnd_tx` once the icon is created so main can post update messages.
pub fn run_tray(
    cmd_tx: UnboundedSender<TrayCommand>,
    mut state_rx: UnboundedReceiver<TrayStateUpdate>,
    hwnd_tx: tokio::sync::oneshot::Sender<HWND>,
) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        CMD_TX.store(&cmd_tx as *const _ as *mut _, Ordering::Release);
        if let Ok(mut guard) = STATE_RX.lock() {
            *guard = Some(state_rx);
        }
        let instance = GetModuleHandleW(None)?;
        let hinstance = HINSTANCE(instance.0);
        let class_name = w!("PeaPodTrayWindow");
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: hinstance,
            lpszClassName: class_name,
            ..Default::default()
        };
        RegisterClassExW(&wc);
        let settings_class = w!("PeaPodSettings");
        let wc_settings = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(settings_wnd_proc),
            hInstance: hinstance,
            lpszClassName: settings_class,
            ..Default::default()
        };
        RegisterClassExW(&wc_settings);
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class_name,
            w!("PeaPod"),
            WINDOW_STYLE::default(),
            0,
            0,
            0,
            0,
            None,
            None,
            Some(hinstance),
            None,
        )?;
        // IDI_APPLICATION = 32512; use as resource id for default app icon
        let icon = LoadIconW(None, windows::core::PCWSTR(32512usize as *const u16))?;
        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: TRAY_ID,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: WM_TRAYICON,
            hIcon: icon,
            ..Default::default()
        };
        let tip = "PeaPod – enabled\r\nPod: 0 device(s)";
        let tip_wide: Vec<u16> = tip.encode_utf16().chain(std::iter::once(0)).collect();
        nid.szTip[..tip_wide.len().min(128)].copy_from_slice(&tip_wide[..tip_wide.len().min(128)]);
        NID_PTR = &mut nid;
        let _ = Shell_NotifyIconW(NIM_ADD, &nid);
        let _ = hwnd_tx.send(hwnd);

        let mut msg = std::mem::zeroed();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        NID_PTR = null_mut();
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
        CMD_TX.store(null_mut(), Ordering::Release);
        if let Ok(mut guard) = STATE_RX.lock() {
            *guard = None;
        }
    }
    Ok(())
}
