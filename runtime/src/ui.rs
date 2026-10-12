//! 界面操作：看别的程序的窗口里有什么能点的，读里面的文字，替机主点一下、填一格。
//!
//! 走的是 Windows 自带的"界面自动化"（给读屏软件用的那一套），不是认图点坐标：它拿到的是"这是一个叫保存的按钮"，
//! 所以点的是那个按钮本身。真正动手的部分在 `uia.ps1`。
//!
//! 谁把关：
//! - 看和读：聊天、邮件类的窗口不看；密码框不读。
//! - 动手：机主在地图上放开过的程序（"程序:名字"那一行的"运行"），直接动，记一笔；没放开过的，递一张票。
//! - 永远不碰：安全软件的窗口、系统的提权确认、ysoul 自己、密码框。

use crate::paths::Root;
use crate::places::{self, Place};
use crate::{hands, pulse, sense};
use anyhow::{bail, Context as _, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::Write;
use std::process::{Command, Stdio};

const UIA: &str = include_str!("uia.ps1");

/// 这些程序的窗口永远不碰：安全软件、系统的提权确认、凭据窗口。
const NEVER: &[&str] = &["hipsmain", "hipstray", "hipsdaemon", "usysdiag", "wsctrl", "consent", "credentialuibroker", "lsass", "securityhealthsystray", "sechealthui", "ysoul-app"];

/// 窗口里一样能动手的东西。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Thing {
    pub i: usize,
    pub kind: String,
    pub name: String,
    #[serde(default)]
    pub secret: bool,
}

/// 一次要做的界面操作。写在票上，机主点头后照着做。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Act {
    /// 程序名（进程名）
    pub app: String,
    pub idx: usize,
    pub kind: String,
    pub name: String,
    /// click 点 / set 填字 / focus 把焦点放过去 / type 一个键一个键地打 / key 按一个键 / menu 点右键 / drag 拖到另一项上
    pub action: String,
    #[serde(default)]
    pub text: String,
    /// drag：要拖到的那一项
    #[serde(default)]
    pub to: Option<usize>,
    #[serde(default)]
    pub to_name: String,
    /// 这一下做完之后接着做的几步。每一步动手前都重新看一遍窗口，按名字找那一项
    #[serde(default)]
    pub then: Vec<Step>,
}

/// 连着做时的一步：按名字找到那一项，对它做一个动作。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Step {
    pub name: String,
    pub action: String,
    #[serde(default)]
    pub text: String,
}

const ACTIONS: &[&str] = &["click", "set", "focus", "type", "key", "menu", "drag", "scroll"];

/// 只是在窗口里走动、不改任何东西的键：翻页、移光标。
const NAV_KEYS: &[&str] = &["up", "down", "left", "right", "home", "end", "pageup", "pagedown", "ctrl+home", "ctrl+end"];

/// 这一下只是走动吗：滚动、把输入位置放过去、按翻页键。走动不改任何东西，所以不用票，没放开过的程序里也直接做。
/// 读屏幕、读文字本来就不用票；走动和它们是一类。点、填、打字、回车、右键、拖，才是动手。
pub fn is_walking(action: &str, text: &str) -> bool {
    match action {
        "scroll" | "focus" => true,
        "key" => NAV_KEYS.contains(&text.trim().to_lowercase().as_str()),
        _ => false,
    }
}

fn call(root: &Root, request: &Value) -> Result<Value> {
    let tmp = root.local().join("tmp");
    std::fs::create_dir_all(&tmp)?;
    let script = tmp.join("uia.ps1");
    std::fs::write(&script, format!("\u{feff}{UIA}"))?;
    // 用系统自带的 Windows PowerShell：界面自动化的库在它那边一定有
    let mut cmd = Command::new("powershell");
    crate::quiet::quiet(&mut cmd);
    let mut child = cmd.args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File"]).arg(&script).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().context("启动不了 PowerShell")?;
    child.stdin.take().context("拿不到标准输入")?.write_all(request.to_string().as_bytes())?;
    let out = child.wait_with_output()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().rev().find(|l| l.trim_start().starts_with('{')).unwrap_or("");
    let v: Value = serde_json::from_str(line).with_context(|| format!("界面那边没有回话：{}", String::from_utf8_lossy(&out.stderr).trim()))?;
    if v["ok"].as_bool() != Some(true) {
        bail!("{}", v["why"].as_str().unwrap_or("没做成"));
    }
    Ok(v)
}

/// 机主此刻用的那个窗口。不能碰的直接报错。
fn target(look_only: bool) -> Result<sense::Fore> {
    let f = sense::foreground().context("看不出前台是哪个窗口")?;
    if NEVER.contains(&f.app.as_str()) {
        bail!("这个窗口不碰（安全软件、系统确认这一类）");
    }
    if sense::is_private(&f.app) {
        bail!("{}", if look_only { "前台是聊天或邮件类的程序，这个不看" } else { "聊天和邮件类的程序不替他操作" });
    }
    Ok(f)
}

fn things_file(root: &Root) -> std::path::PathBuf {
    root.local().join("tmp").join("ui-things.json")
}

/// 看前台窗口里有哪些能动手的东西。看到的清单记在本机，之后动手时用编号指。
/// 可以只看某一种（比如只看链接）。太多时给开头和末尾，中间略去。
pub fn see(root: &Root, kind: &str) -> Result<String> {
    let f = target(true)?;
    let v = call(root, &json!({ "op": "see", "hwnd": f.hwnd }))?;
    let things: Vec<Thing> = serde_json::from_value(v["things"].clone()).unwrap_or_default();
    std::fs::write(things_file(root), json!({ "app": f.app, "things": things }).to_string())?;
    let shown: Vec<&Thing> = things.iter().filter(|t| kind.is_empty() || t.kind.eq_ignore_ascii_case(kind)).collect();
    let mut out = format!("{}的窗口，里面能动手的有 {} 样{}：\n", pulse::friendly(&f.app), shown.len(), if kind.is_empty() { String::new() } else { format!("（只看 {kind}，全部共 {} 样）", things.len()) });
    let line = |t: &Thing| format!("{}　{}　{}{}\n", t.i, t.kind, if t.name.is_empty() { "（没有名字）" } else { &t.name }, if t.secret { "　〔密码框，不碰〕" } else { "" });
    if shown.len() > 130 {
        shown[..50].iter().for_each(|t| out.push_str(&line(t)));
        out.push_str(&format!("……（中间略去 {} 样；要看就用 kind 筛）\n", shown.len() - 130));
        shown[shown.len() - 80..].iter().for_each(|t| out.push_str(&line(t)));
    } else {
        shown.iter().for_each(|t| out.push_str(&line(t)));
    }
    out.push_str("编号按窗口里从上到下、从左到右的次序排；弹出的菜单排在最后。");
    Ok(out)
}

/// 读前台窗口里的文字：他正在写的、正在看的。
pub fn read(root: &Root) -> Result<String> {
    let f = target(true)?;
    let v = call(root, &json!({ "op": "read", "hwnd": f.hwnd }))?;
    Ok(v["text"].as_str().unwrap_or("").to_string())
}

/// 机主此刻在前台窗口里选中的字。没选、读不到、或者是不看的窗口，都返回 None。
/// 他选中一段字再跟它说话，说的"这个""这句"指的就是这段字。
pub fn selection(root: &Root) -> Option<(String, String)> {
    let f = target(true).ok()?;
    let v = call(root, &json!({ "op": "selection", "hwnd": f.hwnd })).ok()?;
    let text = v["text"].as_str().unwrap_or("").trim().to_string();
    (!text.is_empty()).then(|| (pulse::friendly(&f.app), text))
}

/// 一段字此刻在屏幕上的位置。有这段字但滚出去了，返回 None。
pub fn find(root: &Root, text: &str) -> Result<Option<(i32, i32, i32, i32)>> {
    let f = target(true)?;
    let v = call(root, &json!({ "op": "find", "hwnd": f.hwnd, "text": text }))?;
    Ok(v["rect"].as_array().filter(|a| a.len() == 4).map(|a| { let n = |i: usize| a[i].as_i64().unwrap_or(0) as i32; (n(0), n(1), n(2), n(3)) }))
}

/// 同上，但只在前台还是那个程序时才找。批注是留在某个程序里的，机主切走了就不该去别的窗口里找同样的字。
pub fn find_in(root: &Root, app: &str, text: &str) -> Result<Option<(i32, i32, i32, i32)>> {
    if sense::foreground().map(|f| f.app) != Some(app.to_string()) {
        return Ok(None);
    }
    find(root, text)
}

/// 地图上那一行的写法。
pub fn app_row(app: &str) -> String {
    format!("程序:{app}")
}

/// 机主有没有在地图上放开过这个程序。
pub fn app_allowed(map: &[Place], app: &str) -> bool {
    let key = places::norm(&app_row(app));
    map.iter().any(|p| places::norm(&p.path) == key && p.run)
}

pub fn describe(act: &Act) -> String {
    let what = if act.name.is_empty() { format!("那个{}", act.kind) } else { format!("「{}」", act.name) };
    match act.action.as_str() {
        "set" | "type" => format!("在{}里往{what}填上：{}", pulse::friendly(&act.app), act.text.chars().take(30).collect::<String>()),
        "key" => format!("在{}里对着{what}按 {}", pulse::friendly(&act.app), act.text),
        "menu" => format!("在{}里对着{what}点右键", pulse::friendly(&act.app)),
        "drag" => format!("在{}里把{what}拖到「{}」上", pulse::friendly(&act.app), act.to_name),
        "focus" => format!("在{}里把输入位置放到{what}", pulse::friendly(&act.app)),
        _ => format!("在{}里点{what}", pulse::friendly(&act.app)),
    }
}

/// 真的动手。前台必须还是那个程序，那一项必须还是那一项。
pub fn perform(root: &Root, act: &Act) -> Result<String> {
    let f = target(false)?;
    if f.app != act.app {
        bail!("前台已经不是{}了，没有动手", pulse::friendly(&act.app));
    }
    let request = |idx: usize| json!({ "op": if act.action == "scroll" { "scroll" } else { "do" }, "hwnd": f.hwnd, "idx": idx, "name": act.name, "kind": act.kind, "action": act.action, "text": act.text, "to": act.to.map(|t| t as i64).unwrap_or(-1), "to_name": act.to_name });
    let v = match call(root, &request(if act.action == "scroll" { 0 } else { act.idx })) {
        // 从它看到、到机主点头，中间界面可能动过，编号对不上了：重新看一遍，按名字和种类再找
        Err(e) if e.to_string().contains("界面变了") => {
            let seen = call(root, &json!({ "op": "see", "hwnd": f.hwnd }))?;
            let things: Vec<Thing> = serde_json::from_value(seen["things"].clone()).unwrap_or_default();
            let t = things.iter().find(|t| t.name == act.name && t.kind == act.kind && !t.secret).with_context(|| format!("「{}」已经不在窗口里了", act.name))?;
            call(root, &request(t.i))?
        }
        other => other?,
    };
    let mut done = vec![v["how"].as_str().unwrap_or("做了").to_string()];
    // 接着做的几步：每一步都重新看一遍窗口，按名字找。哪一步没成就停在那一步
    for (n, step) in act.then.iter().enumerate() {
        std::thread::sleep(std::time::Duration::from_millis(450));
        let f = target(false)?;
        if f.app != act.app {
            bail!("做到第 {} 步时前台已经不是{}了，后面的没做。已经做了的：{}", n + 2, pulse::friendly(&act.app), done.join("、"));
        }
        let seen = call(root, &json!({ "op": "see", "hwnd": f.hwnd }))?;
        let things: Vec<Thing> = serde_json::from_value(seen["things"].clone()).unwrap_or_default();
        if step.action == "scroll" {
            let v = call(root, &json!({ "op": "scroll", "hwnd": f.hwnd, "text": step.text }))?;
            done.push(v["how"].as_str().unwrap_or("滚了").to_string());
            continue;
        }
        let Some(t) = things.iter().find(|t| t.name == step.name && !t.secret) else { bail!("第 {} 步要找的「{}」不在窗口里，停在这一步。已经做了的：{}", n + 2, step.name, done.join("、")) };
        let v = call(root, &json!({ "op": "do", "hwnd": f.hwnd, "idx": t.i, "name": t.name, "kind": t.kind, "action": step.action, "text": step.text, "to": -1, "to_name": "" }))?;
        done.push(format!("{}：{}", step.name, v["how"].as_str().unwrap_or("做了")));
    }
    Ok(done.join("；"))
}

/// 它要在前台窗口里动一下。放开过的程序直接动并记一笔；没放开过的递票。
pub fn act(root: &Root, now: &str, idx: usize, action: &str, text: &str, to: Option<usize>, then: Vec<Step>) -> Result<String> {
    if !ACTIONS.contains(&action) || then.iter().any(|s| !ACTIONS.contains(&s.action.as_str()) || s.action == "drag") {
        bail!("动作只有这几种：{}（连着做的后几步里不能有 drag）", ACTIONS.join("、"));
    }
    if then.len() > 8 {
        bail!("一次最多连着做九步");
    }
    let f = target(false)?;
    // 滚动不用先点名哪一项，也不用先看过
    if action == "scroll" {
        let act = Act { app: f.app.clone(), idx: 0, kind: String::new(), name: String::new(), action: action.into(), text: text.into(), to: None, to_name: String::new(), then: Vec::new() };
        return perform(root, &act);
    }
    let seen: Value = std::fs::read_to_string(things_file(root)).ok().and_then(|s| serde_json::from_str(&s).ok()).context("先用 ui_see 看一遍窗口里有什么，再动手")?;
    if seen["app"].as_str() != Some(f.app.as_str()) {
        bail!("你上次看的是另一个程序的窗口。重新 ui_see");
    }
    let things: Vec<Thing> = serde_json::from_value(seen["things"].clone()).unwrap_or_default();
    let thing = things.iter().find(|t| t.i == idx).context("没有这个编号。重新 ui_see")?;
    if thing.secret {
        bail!("这是密码框，不碰");
    }
    let to_name = match (action, to) {
        ("drag", Some(t)) => things.iter().find(|x| x.i == t).context("要拖到的那个编号不在清单里")?.name.clone(),
        ("drag", None) => bail!("drag 要写 to：拖到哪一项上"),
        _ => String::new(),
    };
    let act = Act { app: f.app.clone(), idx, kind: thing.kind.clone(), name: thing.name.clone(), action: action.into(), text: text.into(), to, to_name, then };
    let mut title = describe(&act);
    if !act.then.is_empty() {
        title.push_str(&format!("，然后{}", act.then.iter().map(|s| format!("「{}」", s.name)).collect::<Vec<_>>().join("、")));
    }
    // 只是走动（滚动、放焦点、翻页键）：不改任何东西，不用票，也不留爪印
    if is_walking(&act.action, &act.text) && act.then.iter().all(|s| is_walking(&s.action, &s.text)) {
        return perform(root, &act);
    }
    if app_allowed(&places::load(root), &f.app) {
        let how = perform(root, &act)?;
        hands::record(root, now, &title, &pulse::friendly(&f.app), &how)?;
        return Ok(format!("{how}。（{title}）"));
    }
    let t = hands::issue_ui(root, now, &title, act)?;
    Ok(format!("{}这个程序机主还没放开让你操作，所以这一下转成了票 {}，等他点头。他可以只同意这一次，也可以点「这个程序以后都行」。告诉他票递过去了；不要说已经点了。", pulse::friendly(&f.app), t.id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_app_is_open_only_if_its_row_says_run() {
        let map = places::parse("| 地方 | 是什么 | 看 | 放 | 改 | 扔 | 运行 | 依据 |\n|---|---|---|---|---|---|---|---|\n| `程序:notepad` | 记事本 | – | – | – | – | ✓ | 你点头的 |\n| `程序:code` | VS Code | – | – | – | – | – | 你点头的 |\n| `D:\\notepad` | 一个目录 | ✓ | ✓ | ✓ | ✓ | ✓ | 你说的 |\n");
        assert!(app_allowed(&map, "notepad"));
        assert!(app_allowed(&map, "Notepad"));
        assert!(!app_allowed(&map, "code"));
        assert!(!app_allowed(&map, "chrome"));
    }

    #[test]
    fn an_action_is_described_in_words_the_owner_can_read() {
        let click = Act { app: "notepad".into(), idx: 3, kind: "Button".into(), name: "保存".into(), action: "click".into(), text: String::new(), to: None, to_name: String::new(), then: Vec::new() };
        assert!(describe(&click).contains("点「保存」"));
        let set = Act { action: "set".into(), text: "你好".into(), name: String::new(), kind: "Edit".into(), ..click };
        assert!(describe(&set).contains("那个Edit") && describe(&set).contains("你好"));
    }

    #[test]
    fn walking_around_is_free_but_touching_is_not() {
        assert!(is_walking("scroll", "bottom"));
        assert!(is_walking("focus", ""));
        assert!(is_walking("key", "Ctrl+End"));
        assert!(is_walking("key", "pagedown"));
        assert!(!is_walking("key", "enter"), "回车会提交东西");
        assert!(!is_walking("key", "ctrl+w"), "关页签是动手");
        assert!(!is_walking("click", ""));
        assert!(!is_walking("type", "x"));
    }

    #[test]
    fn security_windows_are_on_the_never_list() {
        assert!(NEVER.contains(&"consent"));
        assert!(NEVER.contains(&"hipsmain"));
        assert!(NEVER.contains(&"ysoul-app"));
    }
}
