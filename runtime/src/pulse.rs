//! 脉搏：把一下一下的"此刻"并成一段一段的经历。
//!
//! 它不调模型，也不带日程。它只做三件事：记住机主此刻在哪个程序、哪个地方；
//! 看出一段活动什么时候开始、什么时候结束；看出机主什么时候走开、什么时候回来。
//! "此刻"是一张随写随擦的便签；留下来的只有每段活动结束后的那一行（见 07）。

use crate::paths::Root;
use anyhow::Result;
use chrono::{Local, TimeZone};
use serde::Serialize;
use std::io::Write;

/// 多久没有输入算走开了（秒）。机主定的是 5 分钟。
pub const AWAY_AFTER: u64 = 300;
/// 换到另一个程序之后要待满这么久，才算真的换了。短暂切过去看一眼不算。
pub const SETTLE: i64 = 20;
/// 短于这么久的一段不记，太碎。
pub const MIN_LEN: i64 = 60;

/// 某一刻看到的情况。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Sample {
    /// 时间，Unix 秒
    pub at: i64,
    /// 前台是哪个程序（可执行文件的名字，小写，不带扩展名）
    pub app: String,
    /// 窗口标题。聊天和邮件类的程序这里是空的
    pub title: String,
    /// 在哪个地方干活（认得出的仓库名；认不出就是空的）
    pub place: String,
    /// 机主多少秒没有碰键盘鼠标了
    pub idle: u64,
}

/// 一段经历。
#[derive(Debug, Clone, PartialEq, Serialize, serde::Deserialize)]
pub struct Segment {
    pub start: i64,
    pub end: i64,
    pub app: String,
    pub place: String,
    pub title: String,
}

/// 脉搏每跳一下可能发生的事。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Event {
    /// 一段活动结束了，值得记一行
    Closed(Segment),
    /// 机主走开了（从这个时间起）
    Away(i64),
    /// 机主回来了
    Back(i64),
    /// 机主换了正在做的事：程序、地方
    Now(String, String),
}

#[derive(Default)]
pub struct Pulse {
    cur: Option<Segment>,
    /// 刚切过去、还没待够时间的那个去处：程序、地方、切过去的时间、标题
    pending: Option<(String, String, i64, String)>,
    away: bool,
}

impl Pulse {
    pub fn new() -> Pulse {
        Pulse::default()
    }

    /// 机主此刻在做的事：程序、地方。走开了或还不知道就是空的。
    pub fn now(&self) -> Option<(&str, &str)> {
        if self.away {
            return None;
        }
        self.cur.as_ref().map(|c| (c.app.as_str(), c.place.as_str()))
    }

    /// 手头正在进行的那一段（还没结束、还没记进日记的）。
    pub fn open(&self) -> Option<&Segment> {
        self.cur.as_ref()
    }

    pub fn is_away(&self) -> bool {
        self.away
    }

    fn close(&mut self, end: i64, out: &mut Vec<Event>) {
        if let Some(mut c) = self.cur.take() {
            c.end = end.max(c.start);
            if c.end - c.start >= MIN_LEN {
                out.push(Event::Closed(c));
            }
        }
    }

    /// 喂进一刻的情况，返回这一下发生了什么。
    pub fn feed(&mut self, s: &Sample) -> Vec<Event> {
        let mut out = Vec::new();
        if s.idle >= AWAY_AFTER {
            if !self.away {
                self.away = true;
                // 走开的时间是最后一次输入的时间，不是发现走开的时间
                let left = s.at - s.idle as i64;
                self.close(left, &mut out);
                self.pending = None;
                out.push(Event::Away(left));
            }
            return out;
        }
        if self.away {
            self.away = false;
            out.push(Event::Back(s.at));
        }
        let same = matches!(&self.cur, Some(c) if c.app == s.app && c.place == s.place);
        if self.cur.is_none() {
            self.cur = Some(Segment { start: s.at, end: s.at, app: s.app.clone(), place: s.place.clone(), title: s.title.clone() });
            out.push(Event::Now(s.app.clone(), s.place.clone()));
        } else if same {
            if let Some(c) = &mut self.cur {
                c.end = s.at;
                if !s.title.is_empty() {
                    c.title = s.title.clone();
                }
            }
            self.pending = None;
        } else {
            let waited = matches!(&self.pending, Some(p) if p.0 == s.app && p.1 == s.place);
            if !waited {
                self.pending = Some((s.app.clone(), s.place.clone(), s.at, s.title.clone()));
            } else if let Some(p) = self.pending.clone() {
                if s.at - p.2 >= SETTLE {
                    self.close(p.2, &mut out);
                    self.cur = Some(Segment { start: p.2, end: s.at, app: p.0.clone(), place: p.1.clone(), title: s.title.clone() });
                    self.pending = None;
                    out.push(Event::Now(p.0, p.1));
                }
            }
        }
        out
    }

    /// 要退出了：把手上这一段收尾。
    pub fn flush(&mut self, at: i64) -> Vec<Event> {
        let mut out = Vec::new();
        self.close(at, &mut out);
        out
    }
}

fn checkpoint_file(root: &Root) -> std::path::PathBuf {
    root.local().join("open-segment.json")
}

/// 把手头那一段存个底（算到此刻为止）。程序被任务管理器结束、或者直接关机时，靠它补记。
pub fn checkpoint(root: &Root, seg: Option<&Segment>, now: i64) {
    match seg {
        Some(s) => {
            let mut s = s.clone();
            s.end = now;
            let _ = std::fs::create_dir_all(root.local());
            let _ = std::fs::write(checkpoint_file(root), serde_json::to_string(&s).unwrap_or_default());
        }
        None => {
            let _ = std::fs::remove_file(checkpoint_file(root));
        }
    }
}

/// 启动时：上次要是没来得及正常收尾，把留下的那一段补记进日记。满一分钟的才记。返回补记了没有。
pub fn recover(root: &Root) -> bool {
    let Ok(text) = std::fs::read_to_string(checkpoint_file(root)) else { return false };
    let _ = std::fs::remove_file(checkpoint_file(root));
    match serde_json::from_str::<Segment>(&text) {
        Ok(seg) if seg.end - seg.start >= 60 => append_diary(root, &seg).is_ok(),
        _ => false,
    }
}

/// 别的 AI 工具干完一轮活，来说了一声。往日记里记一行：哪个工具、在哪个地方。只记地方，不记它让工具干了什么。
/// 同一个地方十分钟内只记一次；ysoul 自己思考用的那个目录不记；地图上不能看的地方不记。
pub fn heard(root: &Root, tool: &str, cwd: &str) {
    let dir = std::path::Path::new(cwd);
    if cwd.is_empty() || crate::places::real(dir).starts_with(&crate::places::real(root.path())) || !crate::places::allowed(&crate::places::load(root), root, dir, crate::places::Act::See).0 {
        return;
    }
    let now = unix_now();
    let file = root.local().join("heard.json");
    let mut last: std::collections::HashMap<String, i64> = std::fs::read_to_string(&file).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    let key = crate::places::real(dir);
    if last.get(&key).is_some_and(|t| now - t < 600) {
        return;
    }
    last.insert(key, now);
    let _ = std::fs::create_dir_all(root.local());
    let _ = std::fs::write(&file, serde_json::to_string(&last).unwrap_or_default());
    let place = dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let _ = append_diary(root, &Segment { start: now, end: now, app: tool.to_string(), place, title: "干完一轮活".into() });
}

/// 现在的时间，Unix 秒。
pub fn unix_now() -> i64 {
    Local::now().timestamp()
}

/// 程序名换成机主认得的叫法。认不出的原样返回。
pub fn friendly(app: &str) -> String {
    let known = [
        ("code", "VS Code"), ("code - insiders", "VS Code Insiders"), ("windowsterminal", "终端"), ("powershell", "终端"), ("pwsh", "终端"), ("cmd", "终端"),
        ("explorer", "资源管理器"), ("firefox", "Firefox"), ("chrome", "Chrome"), ("msedge", "Edge"), ("typora", "Typora"), ("obsidian", "Obsidian"),
        ("weixin", "微信"), ("wechat", "微信"), ("qq", "QQ"), ("discord", "Discord"), ("mailmaster", "邮箱大师"), ("ms-teams", "Teams"), ("zoom", "Zoom"), ("wemeetapp", "腾讯会议"),
        ("claude", "Claude"), ("codex", "Codex"), ("cursor", "Cursor"), ("windsurf", "Windsurf"), ("trae", "Trae"), ("winword", "Word"), ("excel", "Excel"), ("powerpnt", "PowerPoint"),
        ("acrobat", "Acrobat"), ("potplayermini64", "PotPlayer"), ("honeyview", "Honeyview"), ("qqmusic", "QQ音乐"), ("steam", "Steam"), ("notion", "Notion"), ("zotero", "Zotero"),
        ("jianyingpro", "剪映"), ("docker desktop", "Docker"), ("everything", "Everything"), ("notepad++", "Notepad++"), ("notepad", "记事本"), ("taskmgr", "任务管理器"), ("regedit", "注册表编辑器"),
    ];
    known.iter().find(|(k, _)| *k == app).map(|(_, v)| v.to_string()).unwrap_or_else(|| app.to_string())
}

fn hm(ts: i64) -> String {
    Local.timestamp_opt(ts, 0).single().map(|d| d.format("%H:%M").to_string()).unwrap_or_default()
}

/// 一段经历在日记里的那一行。
pub fn diary_line(seg: &Segment) -> String {
    let mut line = format!("- {}–{}　{}", hm(seg.start), hm(seg.end), friendly(&seg.app));
    if !seg.place.is_empty() {
        line.push_str(&format!("　{}", seg.place));
    }
    if !seg.title.is_empty() {
        line.push_str(&format!("　〈{}〉", seg.title.replace(['\r', '\n'], " ")));
    }
    line
}

/// 把一段经历记进当天的日记。返回写进了哪个文件（相对 memory）。
pub fn append_diary(root: &Root, seg: &Segment) -> Result<String> {
    let day = Local.timestamp_opt(seg.start, 0).single().map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default();
    let dir = root.memory().join("diary");
    std::fs::create_dir_all(&dir)?;
    let file = dir.join(format!("{day}.md"));
    if !file.exists() {
        std::fs::write(&file, format!("---\ntitle: {day} 的经历\naliases: [{day}]\nsource: observed\nstatus: active\n---\n\n# {day}\n\n每一行是一段：几点到几点，在哪个程序、哪个地方，最后停在哪个窗口。由脉搏自动记下，不是它想出来的。\n\n"))?;
    }
    let mut f = std::fs::OpenOptions::new().append(true).open(&file)?;
    writeln!(f, "{}", crate::outbound::redact(&diary_line(seg)))?;
    Ok(format!("diary/{day}.md"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(at: i64, app: &str, place: &str, idle: u64) -> Sample {
        Sample { at, app: app.into(), title: format!("{app} 的窗口"), place: place.into(), idle }
    }

    #[test]
    fn the_first_sample_starts_a_segment_and_reports_now() {
        let mut p = Pulse::new();
        assert_eq!(p.feed(&s(1000, "code", "SOUL", 0)), vec![Event::Now("code".into(), "SOUL".into())]);
        assert_eq!(p.now(), Some(("code", "SOUL")));
    }

    #[test]
    fn staying_in_the_same_place_extends_the_segment_quietly() {
        let mut p = Pulse::new();
        p.feed(&s(1000, "code", "SOUL", 0));
        assert!(p.feed(&s(1100, "code", "SOUL", 3)).is_empty());
    }

    #[test]
    fn a_brief_glance_elsewhere_does_not_end_the_segment() {
        let mut p = Pulse::new();
        p.feed(&s(1000, "code", "SOUL", 0));
        assert!(p.feed(&s(1200, "firefox", "", 0)).is_empty());
        assert!(p.feed(&s(1210, "code", "SOUL", 0)).is_empty());
        assert_eq!(p.now(), Some(("code", "SOUL")));
    }

    #[test]
    fn settling_elsewhere_closes_the_old_segment_at_the_moment_of_switching() {
        let mut p = Pulse::new();
        p.feed(&s(1000, "code", "SOUL", 0));
        p.feed(&s(1300, "firefox", "", 0));
        let ev = p.feed(&s(1325, "firefox", "", 0));
        assert_eq!(ev.len(), 2);
        assert!(matches!(&ev[0], Event::Closed(seg) if seg.start == 1000 && seg.end == 1300 && seg.app == "code"));
        assert_eq!(ev[1], Event::Now("firefox".into(), "".into()));
    }

    #[test]
    fn a_segment_shorter_than_a_minute_is_not_recorded() {
        let mut p = Pulse::new();
        p.feed(&s(1000, "code", "SOUL", 0));
        p.feed(&s(1030, "firefox", "", 0));
        let ev = p.feed(&s(1055, "firefox", "", 0));
        assert_eq!(ev, vec![Event::Now("firefox".into(), "".into())]);
    }

    #[test]
    fn going_idle_closes_the_segment_at_the_last_input_and_reports_away() {
        let mut p = Pulse::new();
        p.feed(&s(1000, "code", "SOUL", 0));
        p.feed(&s(1200, "code", "SOUL", 0));
        let ev = p.feed(&s(1600, "code", "SOUL", 310));
        assert!(matches!(&ev[0], Event::Closed(seg) if seg.end == 1290));
        assert_eq!(ev[1], Event::Away(1290));
        assert!(p.is_away());
        assert_eq!(p.now(), None);
        assert!(p.feed(&s(1700, "code", "SOUL", 410)).is_empty());
    }

    #[test]
    fn coming_back_reports_back_and_starts_a_new_segment() {
        let mut p = Pulse::new();
        p.feed(&s(1000, "code", "SOUL", 0));
        p.feed(&s(1600, "code", "SOUL", 400));
        let ev = p.feed(&s(2000, "code", "SOUL", 1));
        assert_eq!(ev, vec![Event::Back(2000), Event::Now("code".into(), "SOUL".into())]);
    }

    #[test]
    fn flush_closes_what_is_in_hand() {
        let mut p = Pulse::new();
        p.feed(&s(1000, "code", "SOUL", 0));
        p.feed(&s(1500, "code", "SOUL", 0));
        assert!(matches!(&p.flush(1600)[0], Event::Closed(seg) if seg.end == 1600));
    }

    #[test]
    fn a_diary_line_names_the_program_the_place_and_the_window() {
        let seg = Segment { start: 0, end: 600, app: "code".into(), place: "SOUL".into(), title: "06-memory.md".into() };
        let line = diary_line(&seg);
        assert!(line.contains("VS Code") && line.contains("SOUL") && line.contains("〈06-memory.md〉"), "{line}");
    }

    #[test]
    fn append_diary_creates_the_day_file_once_and_adds_lines() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("memory")).unwrap();
        std::fs::create_dir_all(dir.path().join("seed")).unwrap();
        let root = Root::at(dir.path()).unwrap();
        let seg = Segment { start: 86_400 * 20_000, end: 86_400 * 20_000 + 600, app: "firefox".into(), place: "".into(), title: "某个网页".into() };
        let rel = append_diary(&root, &seg).unwrap();
        append_diary(&root, &seg).unwrap();
        let text = std::fs::read_to_string(dir.path().join("memory").join(&rel)).unwrap();
        assert_eq!(text.matches("Firefox").count(), 2);
        assert_eq!(text.matches("title:").count(), 1);
    }

    #[test]
    fn an_unfinished_segment_is_recovered_after_a_crash() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("memory")).unwrap();
        std::fs::create_dir_all(d.path().join("seed")).unwrap();
        let r = Root::at(d.path()).unwrap();
        assert!(!recover(&r), "没有存底就没有可补的");
        let seg = Segment { start: 1_791_700_000, end: 1_791_700_000, app: "code".into(), place: "SOUL".into(), title: "main.rs".into() };
        checkpoint(&r, Some(&seg), 1_791_700_900);
        assert!(recover(&r));
        let day = std::fs::read_dir(r.memory().join("diary")).unwrap().next().unwrap().unwrap().path();
        assert!(std::fs::read_to_string(day).unwrap().contains("SOUL"));
        assert!(!recover(&r), "补过一次就不再补");
        checkpoint(&r, Some(&seg), 1_791_700_020);
        assert!(!recover(&r), "不满一分钟的不记");
        checkpoint(&r, Some(&seg), 1_791_700_900);
        checkpoint(&r, None, 0);
        assert!(!recover(&r), "正常收尾之后没有存底");
    }
}
