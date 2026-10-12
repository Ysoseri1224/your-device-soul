//! 拼出每次思考必带的那一份：框架、我是谁（或种子）、底稿、索引。

use crate::paths::Root;
use anyhow::{Context as _, Result};
use std::path::Path;

/// 拼好的上下文，以及每一段各占多少字，方便看钱花在哪。
#[derive(Debug)]
pub struct Context {
    pub system: String,
    pub parts: Vec<(String, usize)>,
}

/// 去掉 Markdown 文件开头的那段文件头。
pub fn strip_front_matter(text: &str) -> &str {
    let t = text.trim_start_matches('\u{feff}');
    if !t.starts_with("---") {
        return t;
    }
    match t[3..].find("\n---") {
        Some(end) => {
            let rest = &t[3 + end + 4..];
            rest.trim_start_matches(['\r', '\n'])
        }
        None => t,
    }
}

fn read_optional(path: &Path) -> Result<Option<String>> {
    if !path.is_file() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(path).with_context(|| format!("读不了 {}", path.display()))?;
    let body = strip_front_matter(&raw).trim();
    Ok(if body.is_empty() { None } else { Some(body.to_string()) })
}

/// 按固定顺序拼：框架、今天的日期、我是谁或种子、底稿、索引。缺哪段就跳过哪段。
pub fn assemble(root: &Root, today: &str) -> Result<Context> {
    let mut system = String::new();
    let mut parts = Vec::new();
    let mut push = |title: &str, body: &str| {
        if !system.is_empty() {
            system.push_str("\n\n");
        }
        if !title.is_empty() {
            system.push_str(&format!("## {title}\n\n"));
        }
        system.push_str(body);
        parts.push((if title.is_empty() { "框架".to_string() } else { title.to_string() }, body.chars().count()));
    };

    let frame = read_optional(&root.frame())?.context("缺少 prompts/system.md")?;
    push("", &crate::config::fill(root, &frame));
    let via = match std::fs::read_to_string(root.local().join("last-model")) {
        Ok(m) if !m.trim().is_empty() => format!("今天是 {today}。
这次思考经由 Claude Code 调用，上一次用的模型是 {}。", m.trim()),
        _ => format!("今天是 {today}。
这次思考经由 Claude Code 调用，具体模型还不知道。"),
    };
    push("此刻", &via);

    let mem = root.memory();
    match read_optional(&mem.join("me").join("core.md"))? {
        Some(core) => push("我是谁", &core),
        None => {
            let seed = read_optional(&root.seed())?.context("缺少 seed/temperament.md")?;
            push("种子", &seed);
        }
    }
    if let Some(s) = read_optional(&mem.join("standing.md"))? {
        push("底稿", &s);
    }
    if let Some(s) = read_optional(&crate::places::file(root))? {
        push("家的地图", &s);
    }
    // 它答应过、还没做的事。每次思考都带着，免得说了就忘
    let promises: Vec<String> = read_optional(&mem.join("me").join("promises.md"))?.unwrap_or_default().lines().filter(|l| l.trim_start().starts_with("- [ ]")).map(|l| l.to_string()).collect();
    if !promises.is_empty() {
        push("答应过还没做的", &promises.join("\n"));
    }
    let tickets = crate::hands::recent_summary(root, 6);
    if !tickets.is_empty() {
        push("最近的票", &tickets);
    }
    if let Some(s) = read_optional(&mem.join("index.md"))? {
        push("索引", &s);
    }
    Ok(Context { system, parts })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn scaffold() -> (tempfile::TempDir, Root) {
        let t = tempfile::tempdir().unwrap();
        fs::create_dir_all(t.path().join("memory/me")).unwrap();
        fs::create_dir_all(t.path().join("seed")).unwrap();
        fs::create_dir_all(t.path().join("prompts")).unwrap();
        fs::write(t.path().join("prompts/system.md"), "框架文字").unwrap();
        fs::write(t.path().join("seed/temperament.md"), "---\nstatus: draft\n---\n\n种子文字\n").unwrap();
        let root = Root::at(t.path()).unwrap();
        (t, root)
    }

    #[test]
    fn strips_front_matter() {
        assert_eq!(strip_front_matter("---\na: 1\n---\n\n正文"), "正文");
    }

    #[test]
    fn leaves_text_without_front_matter_alone() {
        assert_eq!(strip_front_matter("正文\n---\n别的"), "正文\n---\n别的");
    }

    #[test]
    fn unborn_uses_the_seed() {
        let (_t, root) = scaffold();
        let c = assemble(&root, "2026-10-11").unwrap();
        assert!(c.system.contains("## 种子\n\n种子文字"));
        assert!(!c.system.contains("status: draft"));
        assert!(!c.system.contains("我是谁"));
    }

    #[test]
    fn once_born_core_replaces_the_seed() {
        let (t, root) = scaffold();
        fs::write(t.path().join("memory/me/core.md"), "我自己写的").unwrap();
        let c = assemble(&root, "2026-10-11").unwrap();
        assert!(c.system.contains("## 我是谁\n\n我自己写的"));
        assert!(!c.system.contains("种子文字"));
    }

    #[test]
    fn standing_and_index_are_included_in_order() {
        let (t, root) = scaffold();
        fs::write(t.path().join("memory/standing.md"), "底稿文字").unwrap();
        fs::write(t.path().join("memory/index.md"), "索引文字").unwrap();
        let c = assemble(&root, "2026-10-11").unwrap();
        let names: Vec<&str> = c.parts.iter().map(|p| p.0.as_str()).collect();
        assert_eq!(names, ["框架", "此刻", "种子", "底稿", "索引"]);
        assert!(c.system.find("底稿文字").unwrap() < c.system.find("索引文字").unwrap());
    }

    #[test]
    fn empty_files_are_skipped() {
        let (t, root) = scaffold();
        fs::write(t.path().join("memory/standing.md"), "  \n").unwrap();
        let c = assemble(&root, "2026-10-11").unwrap();
        assert!(!c.system.contains("底稿"));
    }
}
