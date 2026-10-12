//! 一轮对话：拼上下文、留底、问强模型、记忆有改动就提交。

use crate::paths::Root;
use crate::{context, gitmem, outbound, strong};
use anyhow::Result;

/// 一轮对话的结果。
#[derive(Debug)]
pub struct Turn {
    pub text: String,
    /// 会话编号，下一轮带上它就是接着聊。
    pub session: Option<String>,
    /// 这一轮里它记下了哪些笔记。
    pub noted: Vec<String>,
}

pub fn now() -> String {
    chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

pub fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d，星期%u").to_string()
}

pub fn converse(root: &Root, user: &str, resume: Option<&str>) -> Result<Turn> {
    // 先由程序翻一遍笔记，翻到的附在他的话前面。留底留的是附完之后、真正发出去的那一份
    let recalled = crate::recall::attach(root, user, true);
    let user = &format!("{recalled}{user}");
    let ctx = context::assemble(root, &today())?;
    outbound::log(root, &now(), "strong", "claude-code", &format!("{}\n\n---\n\n{user}", ctx.system))?;
    let reply = strong::ask(root, &ctx.system, user, resume);
    // 这一轮里它要是看过屏幕，那张图用完就删。它该记的是看懂的那句话，不是图
    let _ = std::fs::remove_file(root.local().join("tmp").join("look.png"));
    crate::marks::forget_frame(root);
    let reply = reply?;
    if let Some(model) = &reply.model {
        let _ = std::fs::create_dir_all(root.local());
        let _ = std::fs::write(root.local().join("last-model"), model);
    }
    let noted = gitmem::commit_memory(root, "对话中更新")?;
    Ok(Turn { text: reply.text.trim().to_string(), session: reply.session_id, noted })
}

/// 它说过的闲话和机主的反应，记在本机。下一次起念头时带上最近几条，它就知道哪种话受欢迎、哪种招人烦。
fn banter_file(root: &Root) -> std::path::PathBuf {
    root.local().join("banter.jsonl")
}

fn banter_recent(root: &Root, n: usize) -> String {
    let text = std::fs::read_to_string(banter_file(root)).unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(n)..]
        .iter()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .map(|v| format!("- 你说：{}　他的反应：{}", v["text"].as_str().unwrap_or(""), v["reaction"].as_str().unwrap_or("没理会")))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 记下一句闲话，或者补记机主对最近那一句的反应（liked 点了一下 / shushed 让它别说了）。
pub fn banter_note(root: &Root, text: Option<&str>, reaction: Option<&str>) -> Result<()> {
    use std::io::Write;
    std::fs::create_dir_all(root.local())?;
    if let Some(text) = text {
        let line = serde_json::json!({ "at": now(), "text": text });
        writeln!(std::fs::OpenOptions::new().create(true).append(true).open(banter_file(root))?, "{line}")?;
    } else if let Some(reaction) = reaction {
        let all = std::fs::read_to_string(banter_file(root)).unwrap_or_default();
        let mut lines: Vec<String> = all.lines().map(|l| l.to_string()).collect();
        if let Some(last) = lines.last_mut() {
            if let Ok(mut v) = serde_json::from_str::<serde_json::Value>(last) {
                v["reaction"] = reaction.into();
                *last = v.to_string();
            }
        }
        std::fs::write(banter_file(root), lines.join("\n") + "\n")?;
    }
    Ok(())
}

/// 模型的回答算不算"不说"。
pub fn is_silence(reply: &str) -> bool {
    let t = reply.trim();
    t.is_empty() || t.contains("NO_REPLY")
}

/// 一个时机到了，它可以说一句，也可以不说。多数时候不说（见 10 第三节：主动的助手最大的毛病是乱开口）。
/// 返回 None 表示它选择不说。
pub fn muse(root: &Root, occasion: &str) -> Result<Option<String>> {
    let ctx = context::assemble(root, &today())?;
    let diary = std::fs::read_to_string(root.memory().join("diary").join(format!("{}.md", chrono::Local::now().format("%Y-%m-%d")))).unwrap_or_default();
    let lines: Vec<&str> = diary.lines().filter(|l| l.starts_with("- ")).collect();
    let recent = lines[lines.len().saturating_sub(12)..].join("\n");
    let past = banter_recent(root, 6);
    let pending = std::fs::read_to_string(root.memory().join("inbox").join("questions.md")).unwrap_or_default();
    let pending: Vec<&str> = pending.lines().filter(|l| l.trim_start().starts_with("- ") && !l.contains("问过")).take(3).collect();
    let pending = if pending.is_empty() { String::new() } else { format!("\n\n你惦记着、还没问出口的问题（在 inbox/questions.md）：\n{}\n如果这个时机是他刚回到电脑前，你可以不说闲话，改成问其中一个：只问一个，一句话。问了就在那一行末尾加上「（问过 今天的日期）」，这是这一轮唯一允许动的文件。他没接话就算了，不要再问第二次。", pending.join("\n")) };
    let user = format!(
        "（这不是他在跟你说话。这是你自己的一个时机：{occasion}。现在是 {}。）{pending}\n\n今天到现在他在这台机器上的经历：\n{}\n\n你最近说过的闲话：\n{}\n\n想说一句就说一句，像坐在旁边的人随口嘀咕。要求：只一行，三十个字以内；是打趣或搭话，不指点，不汇报；除了上面说的那一种情况，不提问，不复述他在干什么；别重复说过的意思；他让你别说过的那类话不要再说。没有真想说的，就只回 NO_REPLY。多数时候都应该是 NO_REPLY。这一轮不要动任何工具；除了上面说的那一处，不要记笔记。",
        now(),
        if recent.is_empty() { "（还没有记下什么）" } else { &recent },
        if past.is_empty() { "（还没说过）" } else { &past },
    );
    outbound::log(root, &now(), "strong", "claude-code", &format!("{}\n\n---\n\n{user}", ctx.system))?;
    let reply = strong::ask(root, &ctx.system, &user, None)?;
    if is_silence(&reply.text) {
        return Ok(None);
    }
    let line = reply.text.trim().lines().next().unwrap_or("").trim().to_string();
    banter_note(root, Some(&line), None)?;
    Ok(Some(line))
}

/// 反思做到哪了：哪一天、做了几次、当天的日记消化到第几行。存在本机。
#[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq)]
pub struct ReflectState {
    pub date: String,
    pub count: u32,
    pub lines: usize,
}

fn reflect_file(root: &Root) -> std::path::PathBuf {
    root.local().join("reflect.json")
}

fn diary_lines(root: &Root, date: &str) -> usize {
    std::fs::read_to_string(root.memory().join("diary").join(format!("{date}.md"))).unwrap_or_default().lines().filter(|l| l.starts_with("- ")).count()
}

/// 该不该反思：机主走开够久了，新攒的日记够多了，今天还没做满两次。三条都满足才做。
pub fn reflect_due(state: &ReflectState, date: &str, lines_now: usize, away_minutes: i64) -> bool {
    let (count, done) = if state.date == date { (state.count, state.lines) } else { (0, 0) };
    away_minutes >= 30 && count < 2 && lines_now >= done + 12
}

pub fn reflect_state(root: &Root) -> ReflectState {
    std::fs::read_to_string(reflect_file(root)).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

/// 趁机主走开，把今天攒下的经历理一理。返回它改了哪些笔记。
pub fn reflect(root: &Root) -> Result<Vec<String>> {
    let date = chrono::Local::now().format("%Y-%m-%d").to_string();
    let ctx = context::assemble(root, &today())?;
    let past = banter_recent(root, 12);
    let user = format!(
        "（这不是他在跟你说话。他走开了，这是你自己理一理今天的时候。）\n\n读 `diary/{date}.md`。那里每一行是脉搏自动记下的一段：几点到几点、哪个程序、哪个地方、停在哪个窗口。然后做五件事，每件都可以是「没有可做的」：\n\n1. 写（或改写）`days/{date}.md`（没有就建，标题写「{date} 这一天」，别名写上日期）：三五句人话，说他今天在这台机器上大致做了什么、在哪件事上花得最久、停在哪。只写看得出来的，不猜他为什么。日记那份原始的行不要动，也不要抄过来。\n2. 有没有值得记的规律：同样的事在几天的日记里反复出现才算。有就记进 `you/` 下合适的笔记，标明是你看出来的、哪几天的依据；只有今天一天的不算规律，不记。有想问他的，写进 `inbox/questions.md`，一条一行。\n3. 你最近说过的闲话和他的反应在下面。有没有明显的倾向（比如某类话他总让你别说）？有，就在 `us/banter.md` 里记一两条给以后的自己看。没有就不记。\n4. 读 `learned/unsorted.md`（没有这份文件就跳过）。有没有一个主题同时满足：条目多到在里面找起来费劲；条目之间开始有先后、因果、修正这样的结构。两条都满足，才往 `inbox/questions.md` 写一条，问他要不要给这个主题单开一科、打算叫什么英文名。不要自己立。\n5. 看「答应过还没做的」（在 `me/promises.md`）。现在就能做、而且只动你自己记忆的，做掉并把那一行的 `[ ]` 改成 `[x]`、写上哪天做的；要他在场或者要动机器的，留着，下次他跟你说话时你会看到。放了三天以上还做不了的，往 `inbox/questions.md` 写一条问他还要不要。\n\n{}\n\n规矩照旧：他的个人信息不猜不记；聊天和邮件相关的不碰；只动你记忆里的文件。做完用一两句话说你改了什么，没改就说没改。",
        if past.is_empty() { "（还没说过闲话）".to_string() } else { format!("最近的闲话：\n{past}") },
    );
    outbound::log(root, &now(), "strong", "claude-code", &format!("{}\n\n---\n\n{user}", ctx.system))?;
    strong::ask(root, &ctx.system, &user, None)?;
    let noted = gitmem::commit_memory(root, "反思")?;
    let prev = reflect_state(root);
    let state = ReflectState { count: if prev.date == date { prev.count + 1 } else { 1 }, lines: diary_lines(root, &date), date };
    std::fs::create_dir_all(root.local())?;
    std::fs::write(reflect_file(root), serde_json::to_string(&state)?)?;
    Ok(noted)
}

/// 现在该不该反思（按本机存的状态和今天的日记算）。
pub fn reflect_due_now(root: &Root, away_minutes: i64) -> bool {
    let date = chrono::Local::now().format("%Y-%m-%d").to_string();
    reflect_due(&reflect_state(root), &date, diary_lines(root, &date), away_minutes)
}

/// 好奇：他进了一个它没见过的地方。它自己去看一眼那是什么，记一条，不出声。
/// 一天最多三次；看的时候照样过那道门，不能看的地方它看不了。
pub fn wonder(root: &Root, place: &str, path: &str) -> Result<Vec<String>> {
    let ctx = context::assemble(root, &today())?;
    let user = format!(
        "（这不是他在跟你说话。他刚进了一个你没见过的地方：{place}，路径是 {path}。这是你自己好奇的时候。）\n\n用 run 跑几条只读的命令看一眼它是什么：列一下顶层目录、看一眼 README 的开头、git log --oneline -8。然后在 body/places-seen.md 里添一段（没有这份文件就建，标题和别名写好）：这个地方叫什么、是什么、用什么写的、最近在做什么，三四句，只写看到的。被拦下的就写一句「这里我看不了」。不要跟他说话，不要递票，不要记他个人的事。做完只回一个字：好。"
    );
    outbound::log(root, &now(), "strong", "claude-code", &format!("{}\n\n---\n\n{user}", ctx.system))?;
    strong::ask(root, &ctx.system, &user, None)?;
    gitmem::commit_memory(root, "好奇")
}

/// 好奇的另一种：他在用一个它没见过的程序。它去自己的清单里查这是什么，记两句，不出声。
pub fn wonder_tool(root: &Root, app: &str) -> Result<Vec<String>> {
    let ctx = context::assemble(root, &today())?;
    let user = format!(
        "（这不是他在跟你说话。他正在用一个你没见过的程序，进程名是 {app}。这是你自己好奇的时候。）\n\n在你的清单里查它是什么：用 Grep 在 body/software.tsv、body/app-paths.tsv、body/store-apps.tsv、body/startup.tsv 里找这个名字。然后在 body/tools-seen.md 里添一小段（没有这份文件就建，标题和别名写好）：进程名、它是什么程序、谁出的、装在哪。两三句，只写查到的；查不到就写「清单里没有，不知道是什么」。不要跟他说话，不要递票，不要去看它的窗口。做完只回一个字：好。"
    );
    outbound::log(root, &now(), "strong", "claude-code", &format!("{}\n\n---\n\n{user}", ctx.system))?;
    strong::ask(root, &ctx.system, &user, None)?;
    gitmem::commit_memory(root, "好奇")
}

/// 它见过哪些地方，今天好奇过几次。存在本机。
#[derive(serde::Serialize, serde::Deserialize, Default)]
struct Seen {
    places: Vec<String>,
    date: String,
    count: u32,
}

/// 这个地方它见过没有。没见过就记下，并回答今天还该不该为它好奇一次（一天三次为限）。
pub fn first_time(root: &Root, place: &str) -> bool {
    let file = root.local().join("seen.json");
    let mut seen: Seen = std::fs::read_to_string(&file).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    if place.is_empty() || seen.places.iter().any(|p| p.eq_ignore_ascii_case(place)) {
        return false;
    }
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    if seen.date != today {
        seen.date = today;
        seen.count = 0;
    }
    seen.places.push(place.to_string());
    let go = seen.count < 3;
    if go {
        seen.count += 1;
    }
    let _ = std::fs::create_dir_all(root.local());
    let _ = std::fs::write(&file, serde_json::to_string(&seen).unwrap_or_default());
    go
}

/// 顺藤摸瓜：普查的第二遍。第一遍只是把东西列出来；这一遍顺着清单里指向的东西去看它到底是什么。
pub fn follow(root: &Root) -> Result<Vec<String>> {
    let ctx = context::assemble(root, &today())?;
    let user = "（这不是他在跟你说话。这是普查的第二遍：顺藤摸瓜。）\n\n第一遍把这台机器列成了清单，在 body/ 下。清单只说「有这么一项、指向哪」，没说那东西到底是什么。这一遍你顺着去看。\n\n1. 读 body/startup.tsv、body/tasks.tsv、body/services.tsv、body/path.tsv。\n2. 挑出指向**非系统目录**的条目（不在 C:\\Windows、不是微软自带的），最多十二条，优先挑：开机就启动的、你从名字看不出是什么的、指向的路径看着已经不在了的。\n3. 每一条用 run 跑只读命令去看：Test-Path 它还在不在；Get-Item 看大小和日期；是脚本就 Get-Content -TotalCount 25 看开头；是程序就 Get-AuthenticodeSignature 看谁签的名。被拦下的地方就记「看不了」，不要绕。\n4. 写成 body/leads.md（标题、别名写好）：每条一小段——它是什么、谁装的或谁签的、还在不在、每次开机或定时在做什么。最后单列两类：「指向的东西已经不在了的」和「我看不出是什么、想问他的」（后一类同时写进 inbox/questions.md，一条一行）。\n5. 在 index.md 里加一行指向它。\n\n只看，不改机器上的任何东西，不递票。个人信息不记。做完用三四句话说你看了几条、发现了什么。";
    let user = user.replace("\\n", "\n");
    outbound::log(root, &now(), "strong", "claude-code", &format!("{}\n\n---\n\n{user}", ctx.system))?;
    let reply = strong::ask(root, &ctx.system, &user, None)?;
    println!("{}", reply.text.trim());
    gitmem::commit_memory(root, "顺藤摸瓜")
}

/// 上一次会话的编号。出生时和每轮对话后都会更新。
pub fn last_session(root: &Root) -> Option<String> {
    std::fs::read_to_string(root.local().join("last-session")).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

pub fn remember_session(root: &Root, id: &str) {
    let _ = std::fs::create_dir_all(root.local());
    let _ = std::fs::write(root.local().join("last-session"), id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_is_recognised() {
        assert!(is_silence("NO_REPLY"));
        assert!(is_silence("  \n"));
        assert!(is_silence("NO_REPLY。"));
        assert!(!is_silence("这个报错我见过三回了。"));
    }

    #[test]
    fn a_reaction_is_attached_to_the_latest_banter() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("memory")).unwrap();
        std::fs::create_dir_all(d.path().join("seed")).unwrap();
        let r = Root::at(d.path()).unwrap();
        banter_note(&r, Some("第一句"), None).unwrap();
        banter_note(&r, Some("第二句"), None).unwrap();
        banter_note(&r, None, Some("shushed")).unwrap();
        let recent = banter_recent(&r, 6);
        assert!(recent.contains("第一句　他的反应：没理会"), "{recent}");
        assert!(recent.contains("第二句　他的反应：shushed"), "{recent}");
    }

    #[test]
    fn reflection_waits_for_absence_enough_new_lines_and_a_daily_cap() {
        let fresh = ReflectState::default();
        assert!(!reflect_due(&fresh, "2026-10-12", 30, 10), "走开不够久");
        assert!(!reflect_due(&fresh, "2026-10-12", 5, 60), "攒得不够多");
        assert!(reflect_due(&fresh, "2026-10-12", 12, 30));
        let once = ReflectState { date: "2026-10-12".into(), count: 1, lines: 20 };
        assert!(!reflect_due(&once, "2026-10-12", 25, 60), "上次之后新攒的不够");
        assert!(reflect_due(&once, "2026-10-12", 32, 60));
        let twice = ReflectState { date: "2026-10-12".into(), count: 2, lines: 20 };
        assert!(!reflect_due(&twice, "2026-10-12", 90, 60), "一天最多两次");
        assert!(reflect_due(&twice, "2026-10-13", 12, 60), "新的一天重新算");
    }
}
