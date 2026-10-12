//! 指向本机的 MCP 服务：把感官和手交给强模型用。
//!
//! 只走标准输入输出，不开任何网络端口。每行一条 JSON-RPC 消息。
//! 强模型经由 Claude Code 调用时会挂上这个服务；机主自己平时用的 Claude Code 也可以挂（见 00 第三之五节）。

use crate::paths::Root;
use crate::places::{self, Act};
use crate::{hands, pulse, sense};
use anyhow::Result;
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::Path;

/// 它自己的那些工具的清单。Claude Code 经由 MCP 拿到它；别的通道由 agent.rs 直接拿。
pub fn tool_list() -> Value {
    tools()
}

/// 用一次它自己的工具。
pub fn call_tool(root: &Root, name: &str, a: &Value) -> Result<String> {
    call(root, name, a)
}

fn tools() -> Value {
    let s = |props: Value, req: &[&str]| json!({ "type": "object", "properties": props, "required": req });
    json!([
        { "name": "now", "description": "现查此刻：机主前台是哪个程序、在哪个地方干活、多久没动了，以及这台机器的内存和各盘剩余。只读。", "inputSchema": s(json!({}), &[]) },
        { "name": "where", "description": "查家的地图：一个路径属于哪个地方，我能对它看、放、改、扔、运行里的哪几样。动任何东西之前先查。", "inputSchema": s(json!({ "path": { "type": "string" } }), &["path"]) },
        { "name": "open", "description": "把一样东西摆到机主面前：用系统默认的程序打开一个文件、文件夹或网址。机主说「给我看看」时用这个，不要把内容念一遍。", "inputSchema": s(json!({ "path": { "type": "string", "description": "完整路径，或 http(s) 网址" } }), &["path"]) },
        { "name": "launch", "description": "按名字打开这台机器上装的一个程序（「打开 DeepL」「开一下微信」）。名字不用很准。对不上时会告诉你有哪些相近的。", "inputSchema": s(json!({ "name": { "type": "string" } }), &["name"]) },
        { "name": "reveal", "description": "在资源管理器里定位到一个文件，让机主看见它在哪。", "inputSchema": s(json!({ "path": { "type": "string" } }), &["path"]) },
        { "name": "look", "description": "看一眼：把机主此刻前台的那个窗口截成一张图，返回图片路径，接着用 Read 读它。只在机主让你看屏幕（「你看这个」「这是什么意思」），或者靠 now 实在弄不懂他在看什么时用。一次一张。聊天和邮件类的窗口不截。", "inputSchema": s(json!({}), &[]) },
        { "name": "tag", "description": "在机主此刻用的那个窗口上沿夹一张小条，写一句和这个窗口有关的话，他下次看这个窗口时会看到。留半小时，他点一下就没了。四十个字以内。适合：这件事不急着打断他，但和这个窗口有关（「测试还红着两个」「这份文件上次停在第三节」）。不要用它闲聊。", "inputSchema": s(json!({ "text": { "type": "string" } }), &["text"]) },
        { "name": "mark", "description": "在屏幕上指给机主看：圈出一处（ring），或者在一处旁边留一句批注（note）。位置按你刚用 look 截的那张图上的像素来写，所以要先 look。记号留四十五秒。批注四十个字以内。适合：他问「哪里错了」「是哪个按钮」时，直接圈给他看，比用话描述位置强。", "inputSchema": s(json!({
            "kind": { "type": "string", "enum": ["ring", "note"] },
            "x": { "type": "number", "description": "这一处在图上的左边界" }, "y": { "type": "number", "description": "上边界" },
            "w": { "type": "number", "description": "宽" }, "h": { "type": "number", "description": "高" },
            "text": { "type": "string", "description": "ring 可以不写；note 必须写" },
            "anchor": { "type": "string", "description": "只用于 note：他文档里的一小段原文（十来个字，照抄）。给了它，批注就跟着这段字走，他滚动也不会指错；这时不用先 look，x、y、w、h 也不用写。那个程序不把文字交出来时会报错，那就改用 look 加坐标。" }
        }), &["kind"]) },
        { "name": "ui_read", "description": "读机主此刻用的那个窗口里的文字：他正在写的文档、正在看的页面。比 look 截图准，也省。有些程序不把文字交出来，那时再 look。聊天和邮件类的窗口不读。", "inputSchema": s(json!({}), &[]) },
        { "name": "ui_see", "description": "看机主此刻用的那个窗口里有哪些能动手的东西：按钮、输入框、菜单项、页签、链接。每样有个编号，之后用 ui_do 按编号动手。清单按页面上从上到下的次序排，所以「最后一个链接」「倒数第二个链接」就是筛出链接后数末尾。", "inputSchema": s(json!({ "kind": { "type": "string", "description": "只看某一种：Button、Edit、Document、Hyperlink、MenuItem、TabItem、CheckBox、ComboBox、ListItem。不写就是全部" } }), &[]) },
        { "name": "ui_do", "description": "在机主此刻用的那个窗口里动一下：点一个按钮、往一个输入框里填字、把输入位置放过去。要先 ui_see 拿到编号（scroll 除外）。只是走动的——滚动、放焦点、翻页键（方向键、home、end、pageup、pagedown、ctrl+home、ctrl+end）——哪个程序里都直接做，不用票。真动手的（点、填、打字、回车、右键、拖）：机主放开过的程序直接动；没放开过的转成一张票。他在同一个程序里点过两次头，程序就自动把它记成放开了。密码框、安全软件、聊天和邮件类的程序不碰。只在他明说要你替他操作时用。", "inputSchema": s(json!({
            "index": { "type": "integer", "description": "ui_see 列出来的编号" },
            "action": { "type": "string", "enum": ["click", "set", "focus", "type", "key", "menu", "drag", "scroll"], "description": "scroll 滚动（text 写 bottom、top、down、up；不用写 index，也不用先 ui_see）；click 点；set 直接填字；type 一个键一个键地打（set 不行时用）；key 按一个键（text 写键名：enter、tab、esc、ctrl+s、ctrl+a、ctrl+c、ctrl+v、ctrl+z、方向键等）；menu 点右键叫出菜单（之后再 ui_see，菜单项会出现在清单后面）；drag 拖到另一项上（写 to）；focus 把输入位置放过去" },
            "text": { "type": "string", "description": "set、type 要填的字；key 的键名" },
            "to": { "type": "integer", "description": "drag：拖到哪个编号上" },
            "then": { "type": "array", "description": "这一下之后接着做的几步，最多八步。每步按名字找那一项（名字照 ui_see 里的写）。用于已经知道路径的连续操作，比如 点「文件」→ 点「另存为」。哪一步找不到就停。", "items": { "type": "object", "properties": { "name": { "type": "string" }, "action": { "type": "string" }, "text": { "type": "string" } }, "required": ["name", "action"] } }
        }), &["action"]) },
        { "name": "delegate", "description": "把一件活交给这台机器上的另一个 AI 工具（Claude Code、Codex 这类），让它在某个目录里不开界面地做完。适合：要在一个仓库里改好几个文件的编程活，那是它们的本行。交出去的是一张票：机主看得到交给谁、在哪、交的是什么，点了才开始；它做完，票上有它的回话。哪些工具装了，看 body/handles.tsv 里 kind 是 agent 的那几行。", "inputSchema": s(json!({
            "agent": { "type": "string", "description": "claude、codex、gemini、cursor-agent、aider 之一，要是装了的" },
            "cwd": { "type": "string", "description": "在哪个目录里做，完整路径" },
            "task": { "type": "string", "description": "交给它的话。写清楚要什么、不要什么，它看不到你和机主之前说过什么" }
        }), &["agent", "cwd", "task"]) },
        { "name": "run", "description": "在这台机器上跑一条 PowerShell 7 命令。程序会先看这条命令：只读、而且写法简单到程序能完全看懂的（一条管道：Get-ChildItem / Get-Content / Select-String / Get-Process / Get-Service / Get-ItemProperty / git status、log、diff --stat 这类，后面接 Where-Object、Select-Object、Sort-Object 筛选），直接跑，把输出给你。其余的（会改东西的、带变量或多条语句的、程序看不懂的）自动转成一张票，等机主点头。要碰不能看的地方的，直接不跑。所以：想查东西，就写成一条简单的只读管道，路径写死、写全；想改东西，照常写，它会变成票。", "inputSchema": s(json!({
            "command": { "type": "string" },
            "cwd": { "type": "string", "description": "在哪个目录里跑。不写就是你的记忆目录" },
            "why": { "type": "string", "description": "这条命令是做什么的，一句机主读得懂的话。转成票时这就是票的标题" },
            "undo": { "type": "string", "description": "如果它会改东西：撤回用的脚本" }
        }), &["command", "why"]) },
        { "name": "ticket", "description": "要改变这台机器（改注册表、删东西、改设置、停服务等）时，递一张票给机主：写清做什么、在哪、怎么撤回，附上要运行的 PowerShell 脚本。票不会立刻执行；机主在桌面上点了「做」才执行。结果在下一轮的「最近的票」里。撤回脚本尽量给：动之前先把原样导出或备份。", "inputSchema": s(json!({
            "title": { "type": "string", "description": "做什么，一句话，机主读得懂" },
            "place": { "type": "string", "description": "在哪：哪个目录、注册表哪一处、哪个服务" },
            "script": { "type": "string", "description": "要运行的 PowerShell 7 脚本。出错要让它以非零退出" },
            "undo": { "type": "string", "description": "撤回用的 PowerShell 7 脚本。真的撤不回就留空，并在 title 里说明" },
            "elevated": { "type": "boolean", "description": "需不需要管理员权限" }
        }), &["title", "place", "script"]) },
        { "name": "place", "description": "机主说了某个地方以后你能做什么（或者不能做什么）时，递一张改地图的票。机主点了头，程序把这一行写进家的地图，之后才生效。只在他说过之后用；不要替他定。", "inputSchema": s(json!({
            "path": { "type": "string", "description": "这个地方的完整路径" },
            "what": { "type": "string", "description": "这是什么地方，几个字" },
            "see": { "type": "boolean" }, "put": { "type": "boolean" }, "edit": { "type": "boolean" }, "drop": { "type": "boolean" }, "run": { "type": "boolean" },
            "said": { "type": "string", "description": "依据：他的原话，带日期" }
        }), &["path", "what", "see", "put", "edit", "drop", "run", "said"]) },
        { "name": "tickets", "description": "最近几张票后来怎么样了：等点头、做完了、失败、机主说不做、已撤回。", "inputSchema": s(json!({}), &[]) }
    ])
}

fn call(root: &Root, name: &str, a: &Value) -> Result<String> {
    let text = |k: &str| a.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
    match name {
        "now" => {
            let v = sense::vitals();
            let mut out = String::new();
            let mut repo = String::new();
            match sense::foreground() {
                Some(f) => {
                    let place = sense::guess_place(&f.title, &sense::known_places(root));
                    out.push_str(&format!("前台：{}", pulse::friendly(&f.app)));
                    if !place.is_empty() {
                        out.push_str(&format!("，在 {place}"));
                        // 这个地方是个仓库、又不在不能看的地方：顺带说它此刻的样子
                        if let Some(p) = sense::place_path(root, &place).filter(|p| places::allowed(&places::load(root), root, p, Act::See).0) {
                            if let Some(state) = sense::repo_state(&p) {
                                repo = format!("{place} 这个仓库：{state}\n");
                            }
                        }
                    }
                    if sense::is_private(&f.app) {
                        out.push_str("（聊天或邮件类的程序，窗口标题不看）");
                    } else if !f.title.is_empty() {
                        out.push_str(&format!("，窗口标题是〈{}〉", f.title));
                    }
                    out.push('\n');
                }
                None => out.push_str("前台：看不出来\n"),
            }
            out.push_str(&repo);
            out.push_str(&format!("机主 {} 秒没有碰键盘鼠标\n内存用了 {}%（共 {:.1} GB）\n", sense::idle_seconds(), v.mem_used_pct, v.mem_total_gb));
            for (d, free, total) in v.disks {
                out.push_str(&format!("{d} 盘剩 {free:.1} GB，共 {total:.0} GB\n"));
            }
            Ok(out)
        }
        "where" => {
            let path = text("path");
            let map = places::load(root);
            let p = Path::new(&path);
            let lines: Vec<String> = [Act::See, Act::Put, Act::Edit, Act::Drop, Act::Run].iter().map(|act| { let (ok, why) = places::allowed(&map, root, p, *act); format!("{}：{}（{}）", act.word(), if ok { "可以" } else { "不行" }, why) }).collect();
            Ok(lines.join("\n"))
        }
        "launch" => {
            let all = crate::apps::list(root);
            let name = text("name");
            let hits = crate::apps::find(&all, &name);
            match hits.first() {
                Some((app, _)) => {
                    let before = (sense::windows().len(), sense::foreground().map(|f| f.app));
                    crate::apps::launch(app)?;
                    // 等它的窗口出来：多了一个窗口，或者前台换了程序，就算出来了。最多等四秒
                    let mut shown = false;
                    for _ in 0..10 {
                        std::thread::sleep(std::time::Duration::from_millis(400));
                        if sense::windows().len() > before.0 || sense::foreground().map(|f| f.app) != before.1 {
                            shown = true;
                            break;
                        }
                    }
                    Ok(if shown { format!("{} 已经打开，窗口出来了。直接告诉他开了，不用说不确定。", app.name) } else { format!("已经让系统去开 {} 了，四秒内还没见到新窗口（它可能本来就开着，或者启动慢）。照这个实情说。", app.name) })
                }
                None => anyhow::bail!("没找到叫「{name}」的程序。开始菜单里一共有 {} 个程序，换个叫法再试，或者问他具体叫什么。", all.len()),
            }
        }
        "open" | "reveal" => {
            let path = text("path");
            let web = path.starts_with("http://") || path.starts_with("https://");
            if !web {
                let (ok, why) = places::allowed(&places::load(root), root, Path::new(&path), Act::See);
                if !ok {
                    anyhow::bail!(why);
                }
                if !Path::new(&path).exists() {
                    anyhow::bail!("没有这个路径：{path}");
                }
            }
            if name == "open" { hands::open(&path)? } else { hands::reveal(&path)? }
            Ok(format!("已经{}：{path}", if name == "open" { "打开" } else { "在资源管理器里定位到" }))
        }
        "look" => {
            let f = sense::foreground().ok_or_else(|| anyhow::anyhow!("看不出前台是哪个窗口"))?;
            if sense::is_private(&f.app) {
                anyhow::bail!("前台是聊天或邮件类的程序，这个不看");
            }
            let (l, t, r, b) = f.rect;
            let png = hands::snapshot(root, l, t, r, b)?;
            Ok(format!("截好了：{}。这是{}的窗口。用 Read 读这张图。图里要是有密码、密钥之类，不要复述，也不要记。", png.display(), pulse::friendly(&f.app)))
        }
        "tag" => {
            let f = sense::foreground().ok_or_else(|| anyhow::anyhow!("看不出前台是哪个窗口，没处夹"))?;
            crate::marks::add(root, pulse::unix_now(), "tag", &f.app, &text("text"), (0, 0, 0, 0))?;
            Ok(format!("夹在{}的窗口上了。", pulse::friendly(&f.app)))
        }
        "mark" if !text("anchor").trim().is_empty() => {
            if text("kind") != "note" {
                anyhow::bail!("只有批注（note）能跟着字走");
            }
            let anchor = text("anchor");
            let rect = crate::ui::find(root, anchor.trim())?.ok_or_else(|| anyhow::anyhow!("这段字现在不在屏幕上（滚出去了）"))?;
            let app = sense::foreground().map(|f| f.app).unwrap_or_default();
            let m = crate::marks::add(root, pulse::unix_now(), "note", &app, &text("text"), rect)?;
            crate::marks::follow(root, &m.id, anchor.trim())?;
            Ok("批注留在那段字旁边了，会跟着它走，留十分钟。".into())
        }
        "mark" => {
            let frame = crate::marks::load_frame(root).ok_or_else(|| anyhow::anyhow!("先用 look 看一眼，再按那张图上的位置来指"))?;
            let n = |k: &str| a.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0);
            let rect = crate::marks::to_screen(frame, n("x"), n("y"), n("w"), n("h"));
            let kind = text("kind");
            crate::marks::add(root, pulse::unix_now(), &kind, "", &text("text"), rect)?;
            Ok(if kind == "ring" { "圈出来了，留四十五秒。".into() } else { "批注留在那一处旁边了，留四十五秒。".into() })
        }
        "ui_read" => crate::ui::read(root),
        "ui_see" => crate::ui::see(root, text("kind").trim()),
        "ui_do" => {
            let then: Vec<crate::ui::Step> = a.get("then").cloned().map(serde_json::from_value).transpose()?.unwrap_or_default();
            crate::ui::act(root, &crate::mind::now(), a.get("index").and_then(|v| v.as_u64()).unwrap_or(u64::MAX) as usize, &text("action"), &text("text"), a.get("to").and_then(|v| v.as_u64()).map(|v| v as usize), then)
        }
        "delegate" => {
            let t = hands::issue_delegate(root, &crate::mind::now(), &text("agent"), &text("cwd"), &text("task"))?;
            Ok(format!("票 {} 递过去了，机主点了才开始。告诉他票递过去了，以及你交的是什么。", t.id))
        }
        "run" => {
            let cwd = text("cwd");
            match crate::shell::run(root, &crate::mind::now(), &text("command"), Some(cwd.as_str()), &text("why"), &text("undo"))? {
                crate::shell::Outcome::Ran(ok, out) => Ok(format!("{}\n{}", if ok { "跑完了。输出：" } else { "跑了，但它报了错。输出：" }, if out.is_empty() { "（没有输出）" } else { &out })),
                crate::shell::Outcome::Ticket(t, reason) => Ok(format!("这条命令没有直接跑：{reason}。已经转成票 {}，等机主在桌面上点头。告诉他票递过去了；不要说已经跑了。", t.id)),
                crate::shell::Outcome::Refused(reason) => anyhow::bail!("没有跑：{reason}"),
            }
        }
        "ticket" => {
            let t = hands::issue(root, &crate::mind::now(), &text("title"), &text("place"), &text("script"), &text("undo"), a.get("elevated").and_then(|v| v.as_bool()).unwrap_or(false))?;
            Ok(format!("票 {} 已递出，正等机主在桌面上点头。它还没有执行。这一轮只告诉机主「票递过去了，你看一眼」，不要说已经做完。结果在下一轮的「最近的票」里。", t.id))
        }
        "place" => {
            let b = |k: &str| a.get(k).and_then(|v| v.as_bool()).unwrap_or(false);
            let place = places::Place { path: text("path"), what: text("what"), see: b("see"), put: b("put"), edit: b("edit"), drop: b("drop"), run: b("run"), source: text("said") };
            let t = hands::issue_map(root, &crate::mind::now(), place)?;
            Ok(format!("票 {} 已递出。机主点头之前，地图不变，这个地方你还是不能动。", t.id))
        }
        "tickets" => {
            let s = hands::recent_summary(root, 8);
            Ok(if s.is_empty() { "还没有递过票。".into() } else { s })
        }
        other => anyhow::bail!("没有叫 {other} 的工具"),
    }
}

/// 处理一条消息。通知类的消息没有回应。
pub fn handle(root: &Root, req: &Value) -> Option<Value> {
    let id = req.get("id").cloned()?;
    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let ok = |result: Value| Some(json!({ "jsonrpc": "2.0", "id": id, "result": result }));
    match method {
        "initialize" => {
            let ver = req.pointer("/params/protocolVersion").and_then(|v| v.as_str()).unwrap_or("2024-11-05");
            ok(json!({ "protocolVersion": ver, "capabilities": { "tools": {} }, "serverInfo": { "name": "ysoul", "version": env!("CARGO_PKG_VERSION") } }))
        }
        "ping" => ok(json!({})),
        "tools/list" => ok(json!({ "tools": tools() })),
        "tools/call" => {
            let name = req.pointer("/params/name").and_then(|v| v.as_str()).unwrap_or("");
            let empty = json!({});
            let args = req.pointer("/params/arguments").unwrap_or(&empty);
            match call(root, name, args) {
                Ok(text) => ok(json!({ "content": [{ "type": "text", "text": text }], "isError": false })),
                Err(e) => ok(json!({ "content": [{ "type": "text", "text": format!("{e:#}") }], "isError": true })),
            }
        }
        _ => Some(json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32601, "message": format!("不支持 {method}") } })),
    }
}

/// 一直读标准输入，直到对方关掉。
pub fn serve(root: &Root) -> Result<()> {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let Ok(req) = serde_json::from_str::<Value>(&line) else { continue };
        if let Some(resp) = handle(root, &req) {
            writeln!(out, "{resp}")?;
            out.flush()?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> (tempfile::TempDir, Root) {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("memory")).unwrap();
        std::fs::create_dir_all(d.path().join("seed")).unwrap();
        let r = Root::at(d.path()).unwrap();
        (d, r)
    }

    #[test]
    fn initialize_echoes_the_protocol_version() {
        let (_d, r) = root();
        let resp = handle(&r, &json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "protocolVersion": "2025-06-18" } })).unwrap();
        assert_eq!(resp["result"]["protocolVersion"], "2025-06-18");
        assert_eq!(resp["result"]["serverInfo"]["name"], "ysoul");
    }

    #[test]
    fn notifications_get_no_reply() {
        let (_d, r) = root();
        assert!(handle(&r, &json!({ "jsonrpc": "2.0", "method": "notifications/initialized" })).is_none());
    }

    #[test]
    fn the_tool_list_names_every_tool_with_a_schema() {
        let (_d, r) = root();
        let resp = handle(&r, &json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" })).unwrap();
        let tools = resp["result"]["tools"].as_array().unwrap();
        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert_eq!(names, ["now", "where", "open", "launch", "reveal", "look", "tag", "mark", "ui_read", "ui_see", "ui_do", "delegate", "run", "ticket", "place", "tickets"]);
        assert!(tools.iter().all(|t| t["inputSchema"]["type"] == "object"));
    }

    #[test]
    fn issuing_a_ticket_through_the_tool_leaves_it_waiting() {
        let (_d, r) = root();
        let resp = handle(&r, &json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": { "name": "ticket", "arguments": { "title": "试一下", "place": "临时目录", "script": "echo 1" } } })).unwrap();
        assert_eq!(resp["result"]["isError"], false);
        assert_eq!(hands::list(&r)[0].status, "waiting");
    }

    #[test]
    fn opening_something_in_a_private_place_is_refused() {
        let (_d, r) = root();
        std::fs::create_dir_all(r.memory().join("us")).unwrap();
        std::fs::write(places::file(&r), "| 地方 | 是什么 | 看 | 放 | 改 | 扔 | 运行 | 依据 |\n|---|---|---|---|---|---|---|---|\n| `D:\\personal` | 私人的 | – | – | – | – | – | 你说的 |\n").unwrap();
        let resp = handle(&r, &json!({ "jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": { "name": "open", "arguments": { "path": "D:\\personal\\x.md" } } })).unwrap();
        assert_eq!(resp["result"]["isError"], true);
    }

    #[test]
    fn an_unknown_tool_is_an_error_result_not_a_crash() {
        let (_d, r) = root();
        let resp = handle(&r, &json!({ "jsonrpc": "2.0", "id": 5, "method": "tools/call", "params": { "name": "nope", "arguments": {} } })).unwrap();
        assert_eq!(resp["result"]["isError"], true);
    }
}
