//! 感官：现查这台机器此刻的样子。全部只读，全部不需要管理员权限。
//!
//! 看的是机主对系统和文件做的事：前台是哪个程序、哪个窗口；桌面上有哪些窗口、各在什么位置；身体撑不撑。
//! 不看的：聊天和邮件类程序的窗口标题（和谁说话是机主的隐私）；任何窗口里的内容；键盘敲了什么。

use crate::paths::Root;
use serde::Serialize;

/// 这些程序的窗口标题不记。标题里常带着聊天对象的名字。
const PRIVATE_APPS: [&str; 22] = [
    "weixin", "wechat", "wechatappex", "qq", "qqnt", "tim", "discord", "telegram", "mailmaster", "ms-teams", "teams", "outlook", "olk", "zoom", "wemeetapp", "dingtalk", "feishu", "lark", "slack", "kook", "wxwork", "thunderbird",
];

pub fn is_private(app: &str) -> bool {
    PRIVATE_APPS.contains(&app)
}

/// 前台窗口。
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Fore {
    pub app: String,
    pub title: String,
    /// 占满了整块屏幕（看视频、打游戏、放幻灯片）
    pub fullscreen: bool,
    /// 窗口的四条边，真实像素
    #[serde(skip)]
    pub rect: (i32, i32, i32, i32),
    /// 是不是最大化的
    pub maximized: bool,
    /// 系统认这个窗口用的编号
    #[serde(skip)]
    pub hwnd: isize,
}

/// 桌面上的一个窗口。坐标是屏幕上的像素。
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Win {
    pub app: String,
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub fore: bool,
}

/// 一段能落脚的台面：某个窗口的上沿里没被别的窗口挡住的那一截。
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Ledge {
    pub y: i32,
    pub x0: i32,
    pub x1: i32,
    pub app: String,
    pub fore: bool,
}

/// 屏幕：整块的范围，和去掉任务栏之后的范围。
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Default)]
pub struct Screen {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    /// 任务栏的上沿。这是高度为零的那条线
    pub floor: i32,
}

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
pub struct Vitals {
    pub mem_used_pct: u32,
    pub mem_total_gb: f32,
    /// 盘符、剩余（GB）、总量（GB）
    pub disks: Vec<(String, f32, f32)>,
}

/// 从一堆窗口里算出能落脚的台面。窗口按从上到下的层叠顺序给。
/// 一个窗口的上沿，要扣掉压在它上面的窗口挡住的部分；剩下的每一截够宽才算。贴着屏幕顶的（最大化的）不算。
pub fn ledges(wins: &[Win], screen: &Screen, min_width: i32) -> Vec<Ledge> {
    let mut out = Vec::new();
    for (i, w) in wins.iter().enumerate() {
        if w.top <= screen.top + 8 || w.top >= screen.floor - 8 {
            continue;
        }
        let mut spans = vec![(w.left.max(screen.left), w.right.min(screen.right))];
        for above in &wins[..i] {
            if above.top < w.top && above.bottom > w.top {
                let mut next = Vec::new();
                for (a, b) in spans {
                    if above.right <= a || above.left >= b {
                        next.push((a, b));
                    } else {
                        if above.left > a {
                            next.push((a, above.left));
                        }
                        if above.right < b {
                            next.push((above.right, b));
                        }
                    }
                }
                spans = next;
            }
        }
        for (a, b) in spans {
            if b - a >= min_width {
                out.push(Ledge { y: w.top, x0: a, x1: b, app: w.app.clone(), fore: w.fore });
            }
        }
    }
    out
}

/// 一个仓库此刻的样子：分支、没提交的改动有几处、最近一次提交的说明。只读，不联网。
pub fn repo_state(path: &std::path::Path) -> Option<String> {
    let git = |args: &[&str]| {
        let mut cmd = std::process::Command::new("git");
        crate::quiet::quiet(&mut cmd);
        cmd.arg("-C").arg(path).args(args).output().ok().filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    };
    let branch = git(&["rev-parse", "--abbrev-ref", "HEAD"])?;
    let dirty = git(&["status", "--porcelain"]).map(|s| s.lines().count()).unwrap_or(0);
    let last = git(&["log", "-1", "--format=%s（%cr）"]).unwrap_or_default();
    Some(format!("分支 {branch}，{}，最近一次提交：{last}", if dirty == 0 { "没有未提交的改动".to_string() } else { format!("有 {dirty} 处改动还没提交") }))
}

/// 一个地方的名字对应的仓库路径（名字是目录名）。
pub fn place_path(root: &Root, place: &str) -> Option<std::path::PathBuf> {
    let text = std::fs::read_to_string(root.memory().join("body").join("repos.tsv")).ok()?;
    text.lines().skip(1).filter_map(|l| l.split('\t').next()).map(std::path::PathBuf::from).find(|p| p.file_name().is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(place)))
}

/// 一块画面"有多满"：相邻两个像素亮度差得多的地方占多大比例。一行行的字很满，空白和照片不满。
/// buf 是一行一行的 BGRA。
pub fn edginess(buf: &[u8], w: usize, h: usize) -> f32 {
    if w < 2 || h == 0 || buf.len() < w * h * 4 {
        return 0.0;
    }
    let lum = |i: usize| (buf[i] as i32 * 29 + buf[i + 1] as i32 * 150 + buf[i + 2] as i32 * 77) >> 8;
    let mut edges = 0usize;
    for y in 0..h {
        let row = y * w * 4;
        let mut prev = lum(row);
        for x in 1..w {
            let cur = lum(row + x * 4);
            if (cur - prev).abs() > 48 {
                edges += 1;
            }
            prev = cur;
        }
    }
    edges as f32 / ((w - 1) * h) as f32
}

/// 把一块画面从上到下切成一条一条（每条 band 个像素高），各自"有多满"。
/// 一行字所在的那几条很满，行与行之间、空白处的那几条是空的。这样既认得出"这里有一行字"，也认得出"这行字上面是空的"。
pub fn bands(buf: &[u8], w: usize, h: usize, band: usize) -> Vec<f32> {
    let band = band.max(1);
    (0..h / band).map(|i| edginess(&buf[i * band * w * 4..((i + 1) * band * w * 4).min(buf.len())], w, band)).collect()
}

/// 从窗口标题里认出机主在哪个地方干活：标题里出现了哪个已知仓库的名字。认不出就是空的。
pub fn guess_place(title: &str, places: &[String]) -> String {
    let lower = title.to_lowercase();
    places.iter().filter(|p| p.chars().count() >= 3 && lower.contains(&p.to_lowercase())).max_by_key(|p| p.chars().count()).cloned().unwrap_or_default()
}

/// 普查认出的那些仓库的名字（目录名）。用来从窗口标题里认地方。
pub fn known_places(root: &Root) -> Vec<String> {
    let text = std::fs::read_to_string(root.memory().join("body").join("repos.tsv")).unwrap_or_default();
    let mut names: Vec<String> = text.lines().skip(1).filter_map(|l| l.split('\t').next()).filter_map(|p| p.rsplit(['\\', '/']).next()).map(|s| s.to_string()).filter(|s| !s.is_empty()).collect();
    names.sort();
    names.dedup();
    names
}

#[cfg(windows)]
mod win {
    use super::*;
    use std::ffi::c_void;
    use windows_sys::Win32::Foundation::{CloseHandle, BOOL, HWND, LPARAM, RECT};
    use windows_sys::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS};
    use windows_sys::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTOPRIMARY};
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    use windows_sys::Win32::System::SystemInformation::{GetTickCount, GlobalMemoryStatusEx, MEMORYSTATUSEX};
    use windows_sys::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetClassNameW, GetForegroundWindow, GetWindow, GetWindowLongW, GetWindowTextLengthW, GetWindowTextW, GW_HWNDNEXT, GetWindowThreadProcessId, IsIconic, IsWindowVisible, IsZoomed, SetForegroundWindow, GWL_EXSTYLE, WS_EX_TOOLWINDOW,
    };

    fn text(buf: &[u16], n: i32) -> String {
        String::from_utf16_lossy(&buf[..n.max(0) as usize])
    }

    fn pid_of(h: HWND) -> u32 {
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(h, &mut pid) };
        pid
    }

    /// 窗口属于哪个程序：可执行文件的名字，小写，不带扩展名。
    fn app_of(pid: u32) -> String {
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if h.is_null() {
                return String::new();
            }
            let mut buf = [0u16; 520];
            let mut len = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(h, 0, buf.as_mut_ptr(), &mut len);
            CloseHandle(h);
            if ok == 0 {
                return String::new();
            }
            let path = String::from_utf16_lossy(&buf[..len as usize]);
            let file = path.rsplit('\\').next().unwrap_or("");
            file.strip_suffix(".exe").or_else(|| file.strip_suffix(".EXE")).unwrap_or(file).to_lowercase()
        }
    }

    fn class_of(h: HWND) -> String {
        let mut buf = [0u16; 128];
        let n = unsafe { GetClassNameW(h, buf.as_mut_ptr(), buf.len() as i32) };
        text(&buf, n)
    }

    fn frame(h: HWND) -> Option<RECT> {
        let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        let hr = unsafe { DwmGetWindowAttribute(h, DWMWA_EXTENDED_FRAME_BOUNDS as u32, &mut r as *mut RECT as *mut c_void, std::mem::size_of::<RECT>() as u32) };
        (hr == 0 && r.right > r.left && r.bottom > r.top).then_some(r)
    }

    /// 声明"我按真实像素算"。桌面上那只由框架声明过了，命令行这边要自己说。
    pub fn dpi_aware() {
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::SetProcessDPIAware();
        }
    }

    pub fn screen() -> Screen {
        unsafe {
            let mon = MonitorFromWindow(std::ptr::null_mut(), MONITOR_DEFAULTTOPRIMARY);
            let mut info: MONITORINFO = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            if GetMonitorInfoW(mon, &mut info) == 0 {
                return Screen::default();
            }
            Screen { left: info.rcMonitor.left, top: info.rcMonitor.top, right: info.rcMonitor.right, bottom: info.rcMonitor.bottom, floor: info.rcWork.bottom }
        }
    }

    /// 这个窗口是不是 ysoul 自己的：本进程的，或者桌面上那只的。
    unsafe fn is_ours(h: HWND) -> bool {
        let pid = pid_of(h);
        pid == std::process::id() || app_of(pid) == "ysoul-app"
    }

    /// 把键盘还给机主：前台要是 ysoul 自己的窗口，就把它下面那个窗口叫回前台。
    /// 盖屏的那一层只在机主跟它说话时才该拿着键盘。
    pub fn yield_focus() {
        unsafe {
            let mut h = GetForegroundWindow();
            if h.is_null() || !is_ours(h) {
                return;
            }
            for _ in 0..200 {
                h = GetWindow(h, GW_HWNDNEXT);
                if h.is_null() {
                    return;
                }
                if IsWindowVisible(h) != 0 && IsIconic(h) == 0 && GetWindowTextLengthW(h) > 0 && !is_ours(h) && (GetWindowLongW(h, GWL_EXSTYLE) as u32 & WS_EX_TOOLWINDOW) == 0 {
                    SetForegroundWindow(h);
                    return;
                }
            }
        }
    }

    /// 屏幕上几块地方各自"有多满"。用来让猫知道自己是不是挡住了字。
    /// 量的时候把 ysoul 自己那一层从画面里拿掉（只在量的这几十毫秒里），这样量到的是猫身后的东西。
    /// 只算出一个数，像素不留、不存、不出这个函数。
    pub fn busyness(own: isize, rects: &[(i32, i32, i32, i32)]) -> Vec<f32> {
        profiles(own, rects, 0).into_iter().map(|v| v.first().copied().unwrap_or(0.0)).collect()
    }

    /// 同上，但每一块从上到下切成一条一条分别量（band 是每条的高，真实像素）。band 为 0 就是整块算一个数。
    pub fn profiles(own: isize, rects: &[(i32, i32, i32, i32)], band: usize) -> Vec<Vec<f32>> {
        use windows_sys::Win32::Graphics::Gdi::{BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, SRCCOPY};
        use windows_sys::Win32::UI::WindowsAndMessaging::SetWindowDisplayAffinity;
        let mut out = Vec::with_capacity(rects.len());
        unsafe {
            let own = own as HWND;
            if !own.is_null() {
                SetWindowDisplayAffinity(own, 0x11);
                std::thread::sleep(std::time::Duration::from_millis(45));
            }
            let screen = GetDC(std::ptr::null_mut());
            for &(l, t, r, b) in rects {
                let (w, h) = (r - l, b - t);
                if w < 4 || h < 4 || w > 4000 || h > 4000 {
                    out.push(Vec::new());
                    continue;
                }
                let mem = CreateCompatibleDC(screen);
                let bmp = CreateCompatibleBitmap(screen, w, h);
                let old = SelectObject(mem, bmp);
                BitBlt(mem, 0, 0, w, h, screen, l, t, SRCCOPY);
                SelectObject(mem, old);
                let mut info: BITMAPINFO = std::mem::zeroed();
                info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
                info.bmiHeader.biWidth = w;
                info.bmiHeader.biHeight = -h;
                info.bmiHeader.biPlanes = 1;
                info.bmiHeader.biBitCount = 32;
                let mut buf = vec![0u8; (w * h * 4) as usize];
                GetDIBits(screen, bmp, 0, h as u32, buf.as_mut_ptr() as *mut c_void, &mut info, DIB_RGB_COLORS);
                out.push(if band > 0 { bands(&buf, w as usize, h as usize, band) } else { vec![edginess(&buf, w as usize, h as usize)] });
                DeleteObject(bmp);
                DeleteDC(mem);
            }
            ReleaseDC(std::ptr::null_mut(), screen);
            if !own.is_null() {
                SetWindowDisplayAffinity(own, 0);
            }
        }
        out
    }

    /// 鼠标左键此刻是不是按着。只问按没按，不问点的是什么。
    pub fn left_down() -> bool {
        unsafe { (windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(1) as u16 & 0x8000) != 0 }
    }

    pub fn foreground() -> Option<Fore> {
        unsafe {
            let h = GetForegroundWindow();
            if h.is_null() {
                return None;
            }
            // 前台是它自己（机主刚点了猫、正在跟它说话）时不算数：往下找机主在那之前用的那个窗口
            let mut h = h;
            let mut tries = 0;
            while is_ours(h) {
                h = GetWindow(h, GW_HWNDNEXT);
                while !h.is_null() && (IsWindowVisible(h) == 0 || GetWindowTextLengthW(h) == 0 || is_ours(h)) {
                    h = GetWindow(h, GW_HWNDNEXT);
                }
                tries += 1;
                if h.is_null() || tries > 3 {
                    return None;
                }
            }
            let app = app_of(pid_of(h));
            if app.is_empty() {
                return None;
            }
            let mut buf = [0u16; 512];
            let n = GetWindowTextW(h, buf.as_mut_ptr(), buf.len() as i32);
            let title = if is_private(&app) { String::new() } else { text(&buf, n) };
            let class = class_of(h);
            let s = screen();
            let shell = class == "Progman" || class == "WorkerW" || class == "Shell_TrayWnd";
            let fr = frame(h);
            let fullscreen = !shell && fr.map(|r| r.left <= s.left && r.top <= s.top && r.right >= s.right && r.bottom >= s.bottom).unwrap_or(false);
            let rect = fr.map(|r| (r.left.max(s.left), r.top.max(s.top), r.right.min(s.right), r.bottom.min(s.bottom))).unwrap_or_default();
            Some(Fore { app, title, fullscreen, maximized: IsZoomed(h) != 0, rect, hwnd: h as isize })
        }
    }

    struct Collect {
        wins: Vec<Win>,
        fore: HWND,
        me: u32,
    }

    unsafe extern "system" fn each(h: HWND, l: LPARAM) -> BOOL {
        let c = &mut *(l as *mut Collect);
        if IsWindowVisible(h) == 0 || IsIconic(h) != 0 {
            return 1;
        }
        if (GetWindowLongW(h, GWL_EXSTYLE) as u32) & WS_EX_TOOLWINDOW != 0 {
            return 1;
        }
        let mut cloaked = 0u32;
        DwmGetWindowAttribute(h, DWMWA_CLOAKED as u32, &mut cloaked as *mut u32 as *mut c_void, 4);
        if cloaked != 0 {
            return 1;
        }
        let class = class_of(h);
        if class == "Progman" || class == "WorkerW" || class == "Shell_TrayWnd" || class == "Shell_SecondaryTrayWnd" {
            return 1;
        }
        let pid = pid_of(h);
        if pid == c.me {
            return 1;
        }
        let mut t = [0u16; 8];
        if GetWindowTextW(h, t.as_mut_ptr(), t.len() as i32) == 0 {
            return 1;
        }
        if let Some(r) = frame(h) {
            c.wins.push(Win { app: app_of(pid), left: r.left, top: r.top, right: r.right, bottom: r.bottom, fore: h == c.fore });
        }
        1
    }

    /// 桌面上看得见的窗口，从最上面一层往下排。不含它自己的窗口，不含任务栏和桌面。
    pub fn windows() -> Vec<Win> {
        let mut c = Collect { wins: Vec::new(), fore: unsafe { GetForegroundWindow() }, me: std::process::id() };
        unsafe { EnumWindows(Some(each), &mut c as *mut Collect as LPARAM) };
        c.wins
    }

    /// 机主多少秒没有碰键盘鼠标了。只问系统"距上一次输入过了多久"，不知道也不关心输入的是什么。
    pub fn idle_seconds() -> u64 {
        let mut info = LASTINPUTINFO { cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32, dwTime: 0 };
        unsafe {
            if GetLastInputInfo(&mut info) == 0 {
                return 0;
            }
            u64::from(GetTickCount().wrapping_sub(info.dwTime)) / 1000
        }
    }

    pub fn vitals() -> Vitals {
        let mut v = Vitals::default();
        unsafe {
            let mut m: MEMORYSTATUSEX = std::mem::zeroed();
            m.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
            if GlobalMemoryStatusEx(&mut m) != 0 {
                v.mem_used_pct = m.dwMemoryLoad;
                v.mem_total_gb = m.ullTotalPhys as f32 / 1073741824.0;
            }
            for letter in ['C', 'D'] {
                let path: Vec<u16> = format!("{letter}:\\").encode_utf16().chain(std::iter::once(0)).collect();
                let (mut free, mut total) = (0u64, 0u64);
                if GetDiskFreeSpaceExW(path.as_ptr(), &mut free, &mut total, std::ptr::null_mut()) != 0 {
                    v.disks.push((letter.to_string(), free as f32 / 1073741824.0, total as f32 / 1073741824.0));
                }
            }
        }
        v
    }
}

#[cfg(windows)]
pub use win::{busyness, profiles, left_down, yield_focus, dpi_aware, foreground, idle_seconds, screen, vitals, windows};

#[cfg(not(windows))]
pub fn dpi_aware() {}
#[cfg(not(windows))]
pub fn yield_focus() {}
#[cfg(not(windows))]
pub fn profiles(_own: isize, rects: &[(i32, i32, i32, i32)], _band: usize) -> Vec<Vec<f32>> {
    vec![Vec::new(); rects.len()]
}
#[cfg(not(windows))]
pub fn busyness(_own: isize, rects: &[(i32, i32, i32, i32)]) -> Vec<f32> {
    vec![0.0; rects.len()]
}
#[cfg(not(windows))]
pub fn left_down() -> bool {
    false
}

#[cfg(not(windows))]
pub fn foreground() -> Option<Fore> {
    None
}
#[cfg(not(windows))]
pub fn windows() -> Vec<Win> {
    Vec::new()
}
#[cfg(not(windows))]
pub fn screen() -> Screen {
    Screen::default()
}
#[cfg(not(windows))]
pub fn idle_seconds() -> u64 {
    0
}
#[cfg(not(windows))]
pub fn vitals() -> Vitals {
    Vitals::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(app: &str, l: i32, t: i32, r: i32, b: i32) -> Win {
        Win { app: app.into(), left: l, top: t, right: r, bottom: b, fore: false }
    }
    const SCREEN: Screen = Screen { left: 0, top: 0, right: 2000, bottom: 1200, floor: 1150 };

    #[test]
    fn an_unobstructed_window_gives_its_whole_top_edge() {
        let got = ledges(&[w("code", 100, 300, 900, 1000)], &SCREEN, 120);
        assert_eq!(got, vec![Ledge { y: 300, x0: 100, x1: 900, app: "code".into(), fore: false }]);
    }

    #[test]
    fn a_window_on_top_cuts_a_piece_out_of_the_edge_below_it() {
        let got = ledges(&[w("firefox", 400, 100, 700, 600), w("code", 100, 300, 900, 1000)], &SCREEN, 120);
        let code: Vec<_> = got.iter().filter(|l| l.app == "code").map(|l| (l.x0, l.x1)).collect();
        assert_eq!(code, vec![(100, 400), (700, 900)]);
    }

    #[test]
    fn pieces_too_narrow_to_stand_on_are_dropped() {
        let got = ledges(&[w("firefox", 150, 100, 900, 600), w("code", 100, 300, 900, 1000)], &SCREEN, 120);
        assert!(got.iter().all(|l| l.app != "code"));
    }

    #[test]
    fn a_maximized_window_has_no_ledge() {
        assert!(ledges(&[w("code", 0, 0, 2000, 1150)], &SCREEN, 120).is_empty());
    }

    #[test]
    fn a_window_that_sits_lower_does_not_hide_the_edge_above_it() {
        let got = ledges(&[w("top", 100, 500, 900, 900), w("code", 100, 300, 900, 1000)], &SCREEN, 120);
        assert!(got.iter().any(|l| l.app == "code" && l.x0 == 100 && l.x1 == 900));
    }

    #[test]
    fn lines_of_text_are_busy_and_blank_or_smooth_areas_are_not() {
        let (w, h) = (120usize, 40usize);
        let blank = vec![30u8; w * h * 4];
        assert_eq!(edginess(&blank, w, h), 0.0);
        // 一行行的"字"：隔行有字，字是亮暗相间的小竖条
        let mut text = vec![20u8; w * h * 4];
        for y in (0..h).filter(|y| y % 8 < 5) {
            for x in (0..w).filter(|x| x % 4 < 2) {
                let i = (y * w + x) * 4;
                text[i..i + 3].copy_from_slice(&[230, 230, 230]);
            }
        }
        assert!(edginess(&text, w, h) > 0.2, "{}", edginess(&text, w, h));
        // 渐变（照片、壁纸那样慢慢变的）：不算满
        let mut fade = vec![0u8; w * h * 4];
        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) * 4;
                fade[i..i + 3].copy_from_slice(&[(x * 2) as u8, (x * 2) as u8, (x * 2) as u8]);
            }
        }
        assert!(edginess(&fade, w, h) < 0.01);
    }

    #[test]
    fn a_line_of_text_shows_up_as_a_busy_band_with_empty_bands_around_it() {
        // 60 行高的一块，只有第 24–35 行有"字"
        let (w, h) = (100usize, 60usize);
        let mut buf = vec![20u8; w * h * 4];
        for y in 24..36 {
            for x in (0..w).filter(|x| x % 4 < 2) {
                let i = (y * w + x) * 4;
                buf[i..i + 3].copy_from_slice(&[230, 230, 230]);
            }
        }
        let b = bands(&buf, w, h, 12);
        assert_eq!(b.len(), 5);
        assert!(b[0] < 0.01 && b[1] < 0.01 && b[3] < 0.01 && b[4] < 0.01, "{b:?}");
        assert!(b[2] > 0.3, "{b:?}");
        // 整块一起算就被冲淡了：这正是原先躲不开稀疏几行字的原因
        assert!(edginess(&buf, w, h) < b[2] / 3.0);
    }

    #[test]
    fn guess_place_picks_the_longest_known_name_in_the_title() {
        let places = vec!["SOUL".to_string(), "my-control-plane".to_string(), "kb".to_string()];
        assert_eq!(guess_place("main.rs - SOUL - Visual Studio Code", &places), "SOUL");
        assert_eq!(guess_place("bridge - my-control-plane", &places), "my-control-plane");
        assert_eq!(guess_place("某个网页 — Mozilla Firefox", &places), "");
    }

    #[test]
    fn chat_and_mail_programs_are_private() {
        assert!(is_private("weixin") && is_private("mailmaster"));
        assert!(!is_private("firefox") && !is_private("code"));
    }
}
