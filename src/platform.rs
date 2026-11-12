use std::{path::Path, ptr::null_mut};
use windows_sys::Win32::{
    Foundation::*,
    System::Threading::*,
    UI::{Shell::*, WindowsAndMessaging::*},
};

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
pub struct Instance(HANDLE);
impl Instance {
    pub fn acquire() -> Option<Self> {
        unsafe {
            let handle = CreateMutexW(
                std::ptr::null(),
                0,
                wide("Local\\RaceDesk-DesktopWidget-v1").as_ptr(),
            );
            if handle.is_null() {
                return None;
            }
            if GetLastError() == ERROR_ALREADY_EXISTS {
                CloseHandle(handle);
                return None;
            }
            Some(Self(handle))
        }
    }
}
impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

pub struct Desktop {
    pub attached: bool,
    pub host: HWND,
    original_style: isize,
}

pub struct WindowDrag {
    origin: [i32; 2],
    pointer_origin: [i32; 2],
}
impl WindowDrag {
    pub fn begin(hwnd: HWND) -> Option<Self> {
        unsafe {
            let mut rect = std::mem::zeroed::<RECT>();
            let mut pointer = POINT { x: 0, y: 0 };
            if GetWindowRect(hwnd, &mut rect) == 0 || GetCursorPos(&mut pointer) == 0 {
                return None;
            }
            Some(Self {
                origin: [rect.left, rect.top],
                pointer_origin: [pointer.x, pointer.y],
            })
        }
    }
    pub fn update(&self, hwnd: HWND) -> Option<[i32; 2]> {
        unsafe {
            let mut pointer = POINT { x: 0, y: 0 };
            if GetCursorPos(&mut pointer) == 0 {
                return None;
            }
            let position = racedesk::model::drag_position(
                self.origin,
                self.pointer_origin,
                [pointer.x, pointer.y],
            );
            SetWindowPos(
                hwnd,
                HWND_TOP,
                position[0],
                position[1],
                0,
                0,
                SWP_NOSIZE | SWP_NOACTIVATE,
            );
            Some(position)
        }
    }
}
impl Default for Desktop {
    fn default() -> Self {
        Self {
            attached: false,
            host: null_mut(),
            original_style: 0,
        }
    }
}
impl Desktop {
    pub fn alive(&self) -> bool {
        self.attached && unsafe { IsWindow(self.host) != 0 }
    }
    pub fn keep_below_apps(&self, hwnd: HWND) {
        if self.attached {
            unsafe {
                SetWindowPos(
                    hwnd,
                    HWND_BOTTOM,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
        }
    }
    pub fn set(&mut self, hwnd: HWND, attached: bool) -> Result<(), String> {
        unsafe {
            if attached == self.attached && (!attached || self.alive()) {
                return Ok(());
            }
            let mut rect = std::mem::zeroed::<RECT>();
            GetWindowRect(hwnd, &mut rect);
            if attached {
                let prog = FindWindowW(wide("Progman").as_ptr(), std::ptr::null());
                if prog.is_null() {
                    return Err(
                        "Explorer desktop is unavailable. Widget remains a normal window.".into(),
                    );
                }
                // Own the top-level widget from Explorer rather than reparenting its
                // OpenGL surface. Reparenting WGL windows breaks composition on modern
                // Windows 11. An Explorer-owned tool window stays above the wallpaper,
                // below ordinary apps, and remains visible with Show Desktop.
                self.original_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                SetLastError(0);
                SetWindowLongPtrW(hwnd, GWLP_HWNDPARENT, prog as isize);
                if GetLastError() != 0 {
                    return Err(format!("Desktop attachment failed: {}", GetLastError()));
                }
                SetWindowLongPtrW(
                    hwnd,
                    GWL_EXSTYLE,
                    (self.original_style & !(WS_EX_APPWINDOW as isize)) | WS_EX_TOOLWINDOW as isize,
                );
                SetWindowPos(
                    hwnd,
                    HWND_BOTTOM,
                    rect.left,
                    rect.top,
                    0,
                    0,
                    SWP_NOSIZE | SWP_FRAMECHANGED | SWP_SHOWWINDOW | SWP_NOACTIVATE,
                );
                self.host = prog;
                self.attached = true;
            } else {
                SetWindowLongPtrW(hwnd, GWLP_HWNDPARENT, 0);
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, self.original_style);
                SetWindowPos(
                    hwnd,
                    HWND_TOP,
                    rect.left,
                    rect.top,
                    0,
                    0,
                    SWP_NOSIZE | SWP_FRAMECHANGED | SWP_SHOWWINDOW,
                );
                self.attached = false;
                self.host = null_mut();
            }
            Ok(())
        }
    }
}
pub fn startup_enabled() -> bool {
    winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
        .open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
        .and_then(|k| k.get_value::<String, _>("RaceDesk"))
        .is_ok()
}
pub fn set_startup(enabled: bool) -> Result<(), String> {
    let (key, _) = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
        .create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
        .map_err(|e| e.to_string())?;
    if enabled {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        key.set_value("RaceDesk", &format!("\"{}\"", exe.display()))
            .map_err(|e| e.to_string())
    } else {
        match key.delete_value("RaceDesk") {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}
fn copy_wide<const N: usize>(dest: &mut [u16; N], s: &str) {
    for (d, c) in dest.iter_mut().take(N - 1).zip(s.encode_utf16()) {
        *d = c;
    }
}
pub struct Notifier {
    hwnd: HWND,
    added: bool,
}
impl Default for Notifier {
    fn default() -> Self {
        Self {
            hwnd: null_mut(),
            added: false,
        }
    }
}
impl Notifier {
    pub fn show(&mut self, hwnd: HWND, title: &str, body: &str, sound: bool) -> Result<(), String> {
        unsafe {
            self.hwnd = hwnd;
            let mut info = std::mem::zeroed::<NOTIFYICONDATAW>();
            info.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
            info.hWnd = hwnd;
            info.uID = 901;
            info.uFlags = NIF_INFO | NIF_TIP | NIF_ICON;
            info.hIcon = LoadIconW(null_mut(), IDI_INFORMATION);
            info.dwInfoFlags = NIIF_INFO | if sound { 0 } else { NIIF_NOSOUND };
            copy_wide(&mut info.szTip, "RaceDesk race reminders");
            copy_wide(&mut info.szInfoTitle, title);
            copy_wide(&mut info.szInfo, body);
            if Shell_NotifyIconW(if self.added { NIM_MODIFY } else { NIM_ADD }, &info) == 0 {
                return Err("Windows could not accept the notification.".into());
            }
            self.added = true;
            Ok(())
        }
    }
}
impl Drop for Notifier {
    fn drop(&mut self) {
        if self.added {
            unsafe {
                let mut info = std::mem::zeroed::<NOTIFYICONDATAW>();
                info.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
                info.hWnd = self.hwnd;
                info.uID = 901;
                Shell_NotifyIconW(NIM_DELETE, &info);
            }
        }
    }
}
pub fn open_path(path: &Path) -> Result<(), String> {
    std::process::Command::new("explorer.exe")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}
