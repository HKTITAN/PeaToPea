//! System tray icon and menu (Enable / Disable / Exit). Sends commands to main via channel.

#![cfg(windows)]

use std::ptr::null_mut;
use std::sync::atomic::{AtomicPtr, Ordering};

use tokio::sync::mpsc::UnboundedSender;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::GetCursorPos;
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIM_ADD, NIM_DELETE, NIF_ICON, NIF_MESSAGE, NIF_TIP, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::w;
use windows::Win32::UI::WindowsAndMessaging::LoadIconW;

pub enum TrayCommand {
    Enable,
    Disable,
    OpenSettings,
    Exit,
}

const WM_TRAYICON: u32 = WM_USER + 1;
const TRAY_ID: u32 = 1;

static CMD_TX: AtomicPtr<()> = AtomicPtr::new(null_mut());

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
            DestroyMenu(menu);
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
            let _ = tx.send(cmd);
            if matches!(cmd, TrayCommand::Exit) {
                PostQuitMessage(0);
            }
        }
        return LRESULT(0);
    }
    if msg == WM_DESTROY {
        PostQuitMessage(0);
        return LRESULT(0);
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

/// Run the tray icon and message loop in the current thread. Sends commands via `cmd_tx`.
/// Does not return until the user exits (e.g. Exit menu item or window destroyed).
pub fn run_tray(cmd_tx: UnboundedSender<TrayCommand>) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        CMD_TX.store(
            &cmd_tx as *const _ as *mut _,
            Ordering::Release,
        );
        let instance = GetModuleHandleW(None)?;
        let class_name = w!("PeaPodTrayWindow");
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: instance.into(),
            lpszClassName: class_name,
            ..Default::default()
        };
        RegisterClassExW(&wc);
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
            Some(instance.into()),
            None,
        )?;
        // IDI_APPLICATION = 32512; use as resource id for default app icon
        let icon = LoadIconW(
            None,
            windows::core::PCWSTR(32512usize as *const u16),
        )?;
        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: TRAY_ID,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: WM_TRAYICON,
            hIcon: icon.into(),
            ..Default::default()
        };
        let tip = "PeaPod";
        let tip_wide: Vec<u16> = tip.encode_utf16().chain(std::iter::once(0)).collect();
        nid.szTip[..tip_wide.len().min(128)].copy_from_slice(&tip_wide[..tip_wide.len().min(128)]);
        Shell_NotifyIconW(NIM_ADD, &nid)?;

        let mut msg = std::mem::zeroed();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        Shell_NotifyIconW(NIM_DELETE, &nid)?;
        CMD_TX.store(null_mut(), Ordering::Release);
    }
    Ok(())
}
