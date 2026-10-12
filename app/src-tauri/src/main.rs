//! ysoul 在桌面上的样子：一层盖住整块屏幕的透明窗口。箱子、猫和它的那些小东西都画在这一层上。
//! 这一层除了画着东西的地方，点击全部穿过去落到下面的程序上。
//! 想事情和记东西交给核心库；这里管三件事：把话送过去拿回来、盯着鼠标决定点不点得穿、每隔一会儿看一眼桌面并告诉界面。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::Duration;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, State, WebviewWindow};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use ysoul::paths::Root;
use ysoul::pulse::{self, Event, Pulse, Sample};
use ysoul::{gitmem, hands, marks, mind, places, sense};

struct Soul {
    root: Root,
    /// 说话的地方那一场对话的会话编号
    session: Mutex<Option<String>>,
    /// 那一场对话上一句是什么时候说的（Unix 秒）。隔久了就另起一场
    last: Mutex<i64>,
}

/// 这一层上"实心"的几块，用逻辑像素表示。鼠标在这些块之外时，点击穿过去。一块都没有时，整层都穿得过。
#[derive(Default)]
struct HitRegions(Mutex<Vec<Rect>>);

/// 就地问一句的快捷键，按想要的程度排。机主要的是 Ctrl+Alt+空格；它被别的程序（Claude 桌面版）占着时先用后面的。
/// 系统级的快捷键谁先注册归谁，抢不过来；硬抢要装键盘钩子，那会让它看到每一次按键，不做。
fn hotkeys() -> [(Shortcut, &'static str); 3] {
    [
        (Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Space), "Ctrl+Alt+空格"),
        (Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyY), "Ctrl+Alt+Y"),
        (Shortcut::new(Some(Modifiers::ALT | Modifiers::SHIFT), Code::KeyY), "Alt+Shift+Y"),
    ]
}

/// 要快捷键：还没拿到最想要的那个，就再试一次；拿到了就把先前凑合用的那个还回去。脉搏每半分钟叫它一次。
fn claim_hotkey(app: &AppHandle) {
    let keys = hotkeys();
    let state = app.state::<Hotkey>();
    let Ok(mut have) = state.0.lock() else { return };
    if have.as_deref() == Some(keys[0].1) {
        return;
    }
    let gs = app.global_shortcut();
    if gs.register(keys[0].0).is_ok() {
        for (k, _) in &keys[1..] {
            let _ = gs.unregister(*k);
        }
        *have = Some(keys[0].1.to_string());
    } else if have.is_none() {
        *have = keys[1..].iter().find(|(k, _)| gs.register(*k).is_ok()).map(|(_, n)| n.to_string());
    }
}

/// 就地问一句的快捷键最后用上了哪个。全被占了就是空。
#[derive(Default)]
struct Hotkey(Mutex<Option<String>>);

/// 机主点了"退出"。脉搏看到后把手头那一段记进日记再走。
#[derive(Default)]
struct Leaving(std::sync::atomic::AtomicBool);

#[derive(Deserialize, Clone, Copy)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

impl Rect {
    fn contains(&self, px: f64, py: f64) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
}

#[derive(Serialize)]
struct Answer {
    text: String,
    noted: Vec<String>,
    /// 这一轮所在的会话。接着这件事往下说时带回来
    session: Option<String>,
}

#[derive(Serialize, Clone)]
struct Cursor {
    x: f64,
    y: f64,
    hit: bool,
}

/// 每隔一会儿告诉界面的一份"此刻"。
#[derive(Serialize, Clone)]
struct Scene {
    screen: sense::Screen,
    /// 能落脚的台面：各个窗口上沿没被挡住的部分
    ledges: Vec<sense::Ledge>,
    /// 前台是哪个程序（机主认得的叫法）；占没占满屏
    fore: Option<String>,
    fullscreen: bool,
    idle: u64,
    away: bool,
    /// 前台程序的程序名（夹条时对得上台面用）
    fore_app: Option<String>,
    /// 它留在屏幕上的记号
    marks: Vec<marks::Mark>,
    /// 正等机主点头的票有几张
    waiting: usize,
    /// 机主此刻在做的事：程序、地方、从什么时候开始（Unix 秒）
    now: Option<(String, String, i64)>,
    mem: u32,
}

/// 对它说一句话。想的过程要十来秒，所以放到后台线程去等。
/// 两种场合不混在一起：说话的地方是一场连着的对话（隔了四十五分钟没说就另起一场）；
/// 就地问的一句（fresh）每次都是新的一场，免得它以为还在聊上一件事。session 是"接着这件事往下说"时带回来的编号。
#[tauri::command]
async fn say(soul: State<'_, Soul>, text: String, fresh: Option<bool>, session: Option<String>, program: Option<bool>) -> Result<Answer, String> {
    let root = soul.root.clone();
    let (fresh, by_program) = (fresh.unwrap_or(false), program.unwrap_or(false));
    let now = pulse::unix_now();
    let own_talk = !fresh && session.is_none();
    let resume = match session {
        Some(id) => Some(id),
        None if fresh => None,
        None => {
            let stale = now - *soul.last.lock().map_err(|e| e.to_string())? > 45 * 60;
            if stale { None } else { soul.session.lock().map_err(|e| e.to_string())?.clone() }
        }
    };
    let turn = tauri::async_runtime::spawn_blocking(move || {
        let text = if by_program {
            // 这一句是程序接着往下推的（票办完了），不是他说的：不从里面找路径，也不带选中的字
            text
        } else {
            // 他在自己的话里写了路径（或者交来了文件），就是让它看那一件
            ysoul::places::grant_from_words(&root, &text);
            // 他要是在别的窗口里选中了一段字，一并带上：他说的"这个"多半指它
            match ysoul::ui::selection(&root) {
                Some((app, sel)) => format!("〔他此刻在{app}里选中的字：\n{sel}\n〕\n\n{text}"),
                None => text,
            }
        };
        mind::converse(&root, &text, resume.as_deref())
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("{e:#}"))?;
    if own_talk {
        if let Some(id) = &turn.session {
            mind::remember_session(&soul.root, id);
            *soul.session.lock().map_err(|e| e.to_string())? = Some(id.clone());
        }
        *soul.last.lock().map_err(|e| e.to_string())? = now;
    }
    Ok(Answer { text: turn.text, noted: turn.noted, session: turn.session })
}

/// 它出生时想好的第一段话。还没给机主看过才返回。
#[tauri::command]
fn opening(soul: State<'_, Soul>) -> Option<String> {
    let local = soul.root.local();
    if local.join("opening.seen").exists() {
        return None;
    }
    std::fs::read_to_string(local.join("opening.md")).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

#[tauri::command]
fn opening_seen(soul: State<'_, Soul>) {
    let _ = std::fs::write(soul.root.local().join("opening.seen"), "");
}

#[tauri::command]
fn set_hit_regions(regions: State<'_, HitRegions>, list: Vec<Rect>) {
    if let Ok(mut r) = regions.0.lock() {
        *r = list;
    }
}

/// 箱子放在哪、是不是勿扰。存在本机，不进仓库。
#[tauri::command]
fn ui_state_load(soul: State<'_, Soul>) -> Option<String> {
    std::fs::read_to_string(soul.root.local().join("ui-state.json")).ok()
}

#[tauri::command]
fn ui_state_save(soul: State<'_, Soul>, json: String) {
    let _ = std::fs::create_dir_all(soul.root.local());
    let _ = std::fs::write(soul.root.local().join("ui-state.json"), json);
}

/// 今天的经历：日记里今天那一份的每一行。
#[tauri::command]
fn diary_today(soul: State<'_, Soul>) -> Vec<String> {
    let file = soul.root.memory().join("diary").join(format!("{}.md", mind::today()));
    std::fs::read_to_string(file).unwrap_or_default().lines().filter(|l| l.starts_with("- ")).map(|l| l[2..].to_string()).collect()
}

/// 用系统默认的程序打开它记忆里的一份笔记，或者整个笔记文件夹。只限记忆目录之内。
#[tauri::command]
fn open_memory(soul: State<'_, Soul>, rel: String) -> Result<(), String> {
    if rel.contains("..") || rel.contains(':') {
        return Err("只能打开记忆目录里的东西".into());
    }
    // "diary/today" 是今天那一份日记
    let rel = if rel == "diary/today" { format!("diary/{}.md", chrono::Local::now().format("%Y-%m-%d")) } else { rel };
    let path = soul.root.memory().join(rel);
    if !path.exists() {
        return Err("没有这份笔记".into());
    }
    hands::open(&path.to_string_lossy()).map_err(|e| e.to_string())
}

/// 机主打的这句话如果就是要打开某个程序，直接开，返回程序的名字；不是就返回空，交给模型。
#[tauri::command]
async fn quick_launch(soul: State<'_, Soul>, text: String) -> Result<Option<String>, String> {
    let root = soul.root.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let all = ysoul::apps::list(&root);
        match ysoul::apps::pick(&all, &text) {
            Some(app) => ysoul::apps::launch(app).map(|_| Some(app.name.clone())).map_err(|e| format!("{e:#}")),
            None => Ok(None),
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 一个时机到了，让它想想要不要说一句。它选择不说时返回空。
#[tauri::command]
async fn muse(soul: State<'_, Soul>, occasion: String) -> Result<Option<String>, String> {
    let root = soul.root.clone();
    tauri::async_runtime::spawn_blocking(move || mind::muse(&root, &occasion)).await.map_err(|e| e.to_string())?.map_err(|e| format!("{e:#}"))
}

/// 机主对刚才那句闲话的反应。
#[tauri::command]
fn banter_reaction(soul: State<'_, Soul>, reaction: String) {
    let _ = mind::banter_note(&soul.root, None, Some(&reaction));
}

/// 机主点掉了一个记号。
#[tauri::command]
fn mark_clear(soul: State<'_, Soul>, id: String) {
    let _ = marks::clear(&soul.root, &id);
}

/// 今天办过的票，新的在前。最近五张画成爪印，全部的在箱子摊开里能看到。
#[tauri::command]
fn tickets_today(soul: State<'_, Soul>) -> Vec<hands::Ticket> {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    hands::list(&soul.root).into_iter().filter(|t| t.at.starts_with(&today) && ["done", "failed", "undone"].contains(&t.status.as_str())).take(40).collect()
}

#[derive(Serialize)]
struct MapView {
    rows: Vec<places::Place>,
    /// 机主列为私人的地方有几处。名字不给界面
    private: usize,
}

fn map_view(root: &Root) -> MapView {
    MapView { rows: places::parse(&std::fs::read_to_string(places::file(root)).unwrap_or_default()), private: places::private_list(root).len() }
}

/// 家的地图，给机主看的。
#[tauri::command]
fn places_view(soul: State<'_, Soul>) -> MapView {
    map_view(&soul.root)
}

/// 机主在地图上点了一格：把这个地方的这一样权限翻过来。这是机主亲手改的，不用票。
#[tauri::command]
fn place_toggle(soul: State<'_, Soul>, path: String, act: String) -> Result<MapView, String> {
    let rows = places::parse(&std::fs::read_to_string(places::file(&soul.root)).unwrap_or_default());
    let mut row = rows.into_iter().find(|p| places::norm(&p.path) == places::norm(&path)).ok_or("地图上没有这一行")?;
    match act.as_str() {
        "see" => row.see = !row.see,
        "put" => row.put = !row.put,
        "edit" => row.edit = !row.edit,
        "drop" => row.drop = !row.drop,
        "run" => row.run = !row.run,
        _ => return Err("不认识的权限".into()),
    }
    row.source = format!("你在地图上改的 {}", chrono::Local::now().format("%Y-%m-%d"));
    places::upsert(&soul.root, &row).map_err(|e| e.to_string())?;
    let _ = gitmem::commit_memory(&soul.root, "机主改了地图");
    Ok(map_view(&soul.root))
}

/// 屏幕上几块地方，各自从上到下一条一条"有多满"（见 sense::profiles）。界面用它判断猫是不是挡住了字、哪里有空地、哪行字上面是空的。
/// 传进来的是这一层窗口里的位置（逻辑像素），band 是每条的高（逻辑像素）；这里换成屏幕上的真实像素。
#[tauri::command]
fn under(window: WebviewWindow, rects: Vec<Rect>, band: f64) -> Vec<Vec<f32>> {
    let (Ok(pos), Ok(scale), Ok(hwnd)) = (window.outer_position(), window.scale_factor(), window.hwnd()) else { return vec![Vec::new(); rects.len()] };
    let px: Vec<(i32, i32, i32, i32)> = rects.iter().map(|r| (pos.x + (r.x * scale) as i32, pos.y + (r.y * scale) as i32, pos.x + ((r.x + r.w) * scale) as i32, pos.y + ((r.y + r.h) * scale) as i32)).collect();
    sense::profiles(hwnd.0 as isize, &px, (band * scale).round().max(1.0) as usize)
}

#[derive(Serialize)]
struct SetupView {
    owner: String,
    private_dirs: Vec<String>,
    /// 各个盘上的顶层目录，供他点选
    suggest: Vec<String>,
    channels: Vec<ysoul::channels::Channel>,
    strong: Vec<String>,
    cheap: Option<String>,
}

/// 设置页要的东西：现在的称呼、私人目录、发现的模型通道和现在的选择。
#[tauri::command]
async fn setup_load(soul: State<'_, Soul>) -> Result<SetupView, String> {
    let root = soul.root.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut suggest = Vec::new();
        for letter in b'C'..=b'Z' {
            let drive = format!("{}:\\", letter as char);
            let Ok(entries) = std::fs::read_dir(&drive) else { continue };
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if e.path().is_dir() && !name.starts_with('$') && !["Windows", "Program Files", "Program Files (x86)", "ProgramData", "System Volume Information", "Recovery", "PerfLogs", "Config.Msi", "Documents and Settings"].contains(&name.as_str()) {
                    suggest.push(format!("{drive}{name}"));
                }
            }
        }
        let all = ysoul::channels::discover();
        let choice = ysoul::channels::choice(&root);
        SetupView { owner: ysoul::config::load(&root).owner, private_dirs: ysoul::config::private_dirs(&root), suggest, strong: if choice.strong.is_empty() { ysoul::channels::strong_order(&all, &choice).into_iter().map(|c| c.id).collect() } else { choice.strong.clone() }, cheap: choice.cheap.clone(), channels: all }
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn setup_save(soul: State<'_, Soul>, owner: String, private_dirs: Vec<String>, strong: Vec<String>, cheap: Option<String>) -> Result<(), String> {
    let r = &soul.root;
    ysoul::config::save(r, &ysoul::config::Config { owner, setup_done: true }).map_err(|e| e.to_string())?;
    ysoul::config::save_private_dirs(r, &private_dirs).map_err(|e| e.to_string())?;
    ysoul::channels::save_choice(r, &ysoul::channels::Choice { strong, cheap }).map_err(|e| e.to_string())
}

fn open_setup(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("setup") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// 把键盘还给机主之前在用的那个窗口。
#[tauri::command]
fn yield_focus() {
    sense::yield_focus();
}

/// 正等机主点头的票。
#[tauri::command]
fn tickets_waiting(soul: State<'_, Soul>) -> Vec<hands::Ticket> {
    hands::list(&soul.root).into_iter().filter(|t| t.status == "waiting").collect()
}

/// 机主对一张票的决定：do 做、no 不做、undo 撤回。执行可能要一会儿，放到后台线程。
#[tauri::command]
async fn ticket_decide(soul: State<'_, Soul>, id: String, what: String) -> Result<hands::Ticket, String> {
    let root = soul.root.clone();
    tauri::async_runtime::spawn_blocking(move || match what.as_str() {
        "do" => hands::decide(&root, &mind::now(), &id, true),
        "no" => hands::decide(&root, &mind::now(), &id, false),
        "undo" => hands::undo(&root, &mind::now(), &id),
        "always" => hands::decide_always(&root, &mind::now(), &id),
        _ => Err(anyhow::anyhow!("不认识的决定")),
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("{e:#}"))
}

fn build_menu(app: &AppHandle, dnd: bool, size: &str, outline: &str) -> tauri::Result<Menu<tauri::Wry>> {
    let talk = MenuItem::with_id(app, "talk", "说话", true, None::<&str>)?;
    let key = app.state::<Hotkey>().0.lock().ok().and_then(|k| k.clone());
    let quick = MenuItem::with_id(app, "quick", match &key { Some(k) => format!("就地问一句（{k}）"), None => "就地问一句（快捷键都被占了）".to_string() }, true, None::<&str>)?;
    let map = MenuItem::with_id(app, "map", "家的地图", true, None::<&str>)?;
    let home = MenuItem::with_id(app, "home", "叫它回箱子", true, None::<&str>)?;
    let dnd_item = CheckMenuItem::with_id(app, "dnd", "这阵子别出声", true, dnd, None::<&str>)?;
    // 猫的大小和描边。小的那档是为了让它在终端里也踩得上字：身子矮了，两行空白就站得下
    let pick = |id: &str, label: &str, on: bool| CheckMenuItem::with_id(app, id, label, true, on, None::<&str>);
    let sizes = tauri::menu::Submenu::with_items(app, "猫的大小", true, &[&pick("size-s", "小（终端里也能踩字）", size == "s")?, &pick("size-m", "中", size == "m")?, &pick("size-l", "大", size == "l")?])?;
    let outlines = tauri::menu::Submenu::with_items(app, "猫的描边", true, &[&pick("outline-0", "不描", outline == "0")?, &pick("outline-1", "细白边", outline == "1")?, &pick("outline-2", "粗白边", outline == "2")?])?;
    let setup = MenuItem::with_id(app, "setup", "设置（称呼、私人目录、模型）", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    Menu::with_items(app, &[&quick, &talk, &map, &home, &PredefinedMenuItem::separator(app)?, &sizes, &outlines, &dnd_item, &setup, &PredefinedMenuItem::separator(app)?, &quit])
}

/// 在箱子上点右键时弹出的菜单。
#[tauri::command]
fn show_menu(app: AppHandle, window: WebviewWindow, dnd: bool, size: Option<String>, outline: Option<String>) -> Result<(), String> {
    let menu = build_menu(&app, dnd, size.as_deref().unwrap_or(""), outline.as_deref().unwrap_or("")).map_err(|e| e.to_string())?;
    window.popup_menu(&menu).map_err(|e| e.to_string())
}

/// 盯着鼠标：告诉界面它在哪，并决定此刻点击该不该穿过这一层。
fn watch_cursor(app: AppHandle) {
    std::thread::spawn(move || {
        let mut last = (f64::MIN, f64::MIN);
        let mut last_hit: Option<bool> = None;
        let mut was_down = false;
        loop {
            std::thread::sleep(Duration::from_millis(40));
            let Some(win) = app.get_webview_window("pet") else { continue };
            let (Ok(cur), Ok(pos), Ok(scale)) = (app.cursor_position(), win.outer_position(), win.scale_factor()) else { continue };
            let x = (cur.x - f64::from(pos.x)) / scale;
            let y = (cur.y - f64::from(pos.y)) / scale;
            let hit = app.state::<HitRegions>().0.lock().map(|r| r.iter().any(|b| b.contains(x, y))).unwrap_or(false);
            if last_hit != Some(hit) {
                let _ = win.set_ignore_cursor_events(!hit);
                last_hit = Some(hit);
            }
            // 机主在它的东西之外按了一下鼠标：告诉界面，摊开的那几块该收起来了
            let down = sense::left_down();
            if down && !was_down && !hit {
                let _ = app.emit("outside", ());
            }
            was_down = down;
            if (x - last.0).abs() > 0.5 || (y - last.1).abs() > 0.5 {
                last = (x, y);
                let _ = app.emit("cursor", Cursor { x, y, hit });
            }
        }
    });
}

/// 脉搏：每隔一会儿看一眼桌面。把看到的告诉界面；把机主的活动并成一段一段，一段结束就往日记里记一行。
fn watch_scene(app: AppHandle, root: Root) {
    std::thread::spawn(move || {
        let places = sense::known_places(&root);
        // 上次要是被强行结束了，手头那一段在这里补记
        pulse::recover(&root);
        let mut pulse = Pulse::new();
        let mut since = 0i64;
        let mut unsaved = false;
        let mut left = 0i64;
        let reflecting = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        loop {
            std::thread::sleep(Duration::from_millis(700));
            let at = pulse::unix_now();
            if app.state::<Leaving>().0.load(std::sync::atomic::Ordering::Relaxed) {
                for ev in pulse.flush(at) {
                    if let Event::Closed(seg) = ev {
                        let _ = pulse::append_diary(&root, &seg);
                    }
                }
                pulse::checkpoint(&root, None, at);
                let _ = gitmem::commit_memory(&root, "日记");
                app.exit(0);
                return;
            }
            let screen = sense::screen();
            let wins = sense::windows();
            let fore = sense::foreground();
            let idle = sense::idle_seconds();
            if let Some(f) = &fore {
                let sample = Sample { at, app: f.app.clone(), title: f.title.clone(), place: sense::guess_place(&f.title, &places), idle };
                for ev in pulse.feed(&sample) {
                    match ev {
                        Event::Closed(seg) => {
                            if pulse::append_diary(&root, &seg).is_ok() {
                                unsaved = true;
                            }
                            // 时机之一：在一件事上连着待了很久，刚换地方
                            let mins = (seg.end - seg.start) / 60;
                            if mins >= 40 {
                                let _ = app.emit("occasion", format!("他刚在{}{}连着待了 {mins} 分钟，现在换地方了", pulse::friendly(&seg.app), if seg.place.is_empty() { String::new() } else { format!("的 {} ", seg.place) }));
                            }
                        }
                        Event::Now(now_app, place) => {
                            since = at;
                            // 好奇：他进了一个它没见过的地方（一个已知的仓库）。它自己去看一眼是什么，不出声
                            if let Some(path) = sense::place_path(&root, &place) {
                                if mind::first_time(&root, &place) {
                                    let r = root.clone();
                                    std::thread::spawn(move || {
                                        let _ = mind::wonder(&r, &place, &path.to_string_lossy());
                                    });
                                }
                            } else if !sense::is_private(&now_app) && mind::first_time(&root, &format!("程序:{now_app}")) {
                                // 他在用一个它没见过的程序：去自己的清单里查这是什么
                                let r = root.clone();
                                std::thread::spawn(move || {
                                    let _ = mind::wonder_tool(&r, &now_app);
                                });
                            }
                        }
                        // 机主走开了：正好把攒下的日记提交掉
                        Event::Away(t) => {
                            left = t;
                            if unsaved && gitmem::commit_memory(&root, "日记").is_ok() {
                                unsaved = false;
                            }
                        }
                        // 时机之二：离开了一阵，刚回来
                        Event::Back(t) => {
                            let mins = (t - left) / 60;
                            if left > 0 && mins >= 10 {
                                let _ = app.emit("occasion", format!("他离开了 {mins} 分钟，刚回到电脑前"));
                            }
                        }
                    }
                }
            }
            if at % 30 == 0 {
                claim_hotkey(&app);
                pulse::checkpoint(&root, pulse.open(), at);
            }
            // 有人从右键菜单把一个文件交给它了
            let handoff = root.local().join("handoff.txt");
            if let Ok(path) = std::fs::read_to_string(&handoff) {
                let _ = std::fs::remove_file(&handoff);
                let _ = app.emit("handoff", path.trim().to_string());
            }
            // 反思：不看钟点，看他走开了多久、攒了多少。每分钟问一次该不该
            if pulse.is_away() && left > 0 && !reflecting.load(std::sync::atomic::Ordering::Relaxed) && at % 60 == 0 && mind::reflect_due_now(&root, (at - left) / 60) {
                reflecting.store(true, std::sync::atomic::Ordering::Relaxed);
                let (r, flag) = (root.clone(), reflecting.clone());
                std::thread::spawn(move || {
                    let _ = mind::reflect(&r);
                    flag.store(false, std::sync::atomic::Ordering::Relaxed);
                });
            }
            let now = pulse.now().map(|(a, p)| (pulse::friendly(a), p.to_string(), since));
            let scene = Scene {
                ledges: sense::ledges(&wins, &screen, 180),
                screen,
                fullscreen: fore.as_ref().map(|f| f.fullscreen).unwrap_or(false),
                fore_app: fore.as_ref().map(|f| f.app.clone()),
                marks: marks::list(&root, at),
                fore: fore.map(|f| pulse::friendly(&f.app)),
                idle,
                away: pulse.is_away(),
                now,
                mem: sense::vitals().mem_used_pct,
                waiting: hands::list(&root).iter().filter(|t| t.status == "waiting").count(),
            };
            let _ = app.emit("scene", scene);
        }
    });
}

/// 跟着字走的批注：隔一会儿去问那段字现在在屏幕上的哪里。只在有这种批注时才问，问一次要起一个进程。
fn watch_anchors(root: Root) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(1200));
        for m in marks::list(&root, pulse::unix_now()).into_iter().filter(|m| !m.anchor.is_empty()) {
            // 找不到（切到别的窗口了、滚出去了）就记成全零，界面上先不画
            let found = ysoul::ui::find_in(&root, &m.app, &m.anchor);
            if let Err(e) = &found {
                let _ = std::fs::write(root.local().join("tmp").join("anchor-error.txt"), format!("{e:#}"));
            }
            let rect = found.ok().flatten().unwrap_or((0, 0, 0, 0));
            let _ = marks::moved(&root, &m.id, rect);
        }
    });
}

/// 让这一层盖住整块主屏。高度少一个像素：正好盖满的窗口会被系统当成"全屏程序"，影响任务栏和免打扰。
fn cover_screen(win: &WebviewWindow) {
    if let Ok(Some(mon)) = win.primary_monitor() {
        let (p, s) = (mon.position(), mon.size());
        let _ = win.set_position(PhysicalPosition::new(p.x, p.y));
        let _ = win.set_size(PhysicalSize::new(s.width, s.height.saturating_sub(1)));
    }
}

fn main() {
    let root = Root::discover(None).or_else(|_| Root::near_exe()).expect("找不到项目根目录：程序要放在同时有 memory 和 seed 两个目录的文件夹里（或者它下面）");
    let session = Mutex::new(mind::last_session(&root));
    let hotkeys = hotkeys();
    let scene_root = root.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            // 已经有一只在跑了：不再开第二只，只把已有的那只叫出来。带 --quick 的（开始菜单里那个快捷方式）叫出的是就地问的那一行
            if args.iter().any(|a| a == "--setup") {
                open_setup(app);
            } else {
                let _ = app.emit("menu", if args.iter().any(|a| a == "--quick") { "quick" } else { "talk" });
            }
        }))
        .manage(Soul { root, session, last: Mutex::new(0) })
        .manage(HitRegions::default())
        .manage(Leaving::default())
        .manage(Hotkey::default())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if hotkeys.iter().any(|(k, _)| shortcut == k) && event.state() == ShortcutState::Pressed {
                        let _ = app.emit("menu", "quick");
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![say, opening, opening_seen, set_hit_regions, ui_state_load, ui_state_save, diary_today, open_memory, tickets_waiting, ticket_decide, muse, banter_reaction, yield_focus, mark_clear, tickets_today, places_view, place_toggle, quick_launch, under, setup_load, setup_save, show_menu])
        .on_menu_event(|app, event| match event.id.as_ref() {
            "quit" => {
                app.state::<Leaving>().0.store(true, std::sync::atomic::Ordering::Relaxed);
                // 脉搏万一没在跑，两秒后自己走
                let app = app.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_secs(2));
                    app.exit(0);
                });
            }
            "setup" => open_setup(app),
            id => {
                let _ = app.emit("menu", id);
            }
        })
        .setup(move |app| {
            // 快捷键被别的程序占了也不要紧，点箱子一样能叫它
            claim_hotkey(app.handle());

            if let Some(win) = app.get_webview_window("pet") {
                // 先让整层点得穿，再铺开。否则铺开的一瞬间它会挡住整块屏幕
                let _ = win.set_ignore_cursor_events(true);
                cover_screen(&win);
            }

            // 托盘上的菜单启动时建一次，不知道后来改成了哪一档，所以大小和描边都不打勾
            let menu = build_menu(app.handle(), false, "", "")?;
            // 托盘图标：左键点一下是就地问一句，右键是菜单
            let mut tray = TrayIconBuilder::new().tooltip("ysoul：点一下问一句").menu(&menu).show_menu_on_left_click(false).on_tray_icon_event(|tray, event| {
                if let tauri::tray::TrayIconEvent::Click { button: tauri::tray::MouseButton::Left, button_state: tauri::tray::MouseButtonState::Up, .. } = event {
                    let _ = tray.app_handle().emit("menu", "quick");
                }
            });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;

            // 第一次启动（还没做过设置）：先把设置页打开
            if !ysoul::config::load(&scene_root).setup_done || std::env::args().any(|a| a == "--setup") {
                open_setup(app.handle());
            }
            // 设置页点叉只是藏起来，不是退出
            if let Some(w) = app.get_webview_window("setup") {
                let w2 = w.clone();
                w.on_window_event(move |e| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = e {
                        api.prevent_close();
                        let _ = w2.hide();
                    }
                });
            }
            watch_cursor(app.handle().clone());
            watch_anchors(scene_root.clone());
            watch_scene(app.handle().clone(), scene_root);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("ysoul 没能启动");
}
