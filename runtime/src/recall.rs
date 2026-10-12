//! 回忆：三级，能在前一级解决就不进后一级（06 第六节）。
//!
//! 一级：程序在本地对一遍。机主的话里出现了哪份笔记的标题或别名，就把那份笔记的开头附上。不花钱，不出这台机器。
//! 二级：一级没对上、而这句话像是在问事情，才把笔记目录交给便宜的模型挑一两份。
//! 三级：强模型自己去翻（它本来就能读整个记忆）。前两级是替它省下"先翻目录"的那一步。

use crate::paths::Root;
use crate::{cheap, outbound};
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    /// 相对记忆目录的路径，正斜杠
    pub path: String,
    pub title: String,
    /// 能对上它的字样：标题、别名
    pub keys: Vec<String>,
}

/// 每次思考本来就带着的几份，不用再附。
const ALWAYS: &[&str] = &["standing.md", "index.md", "me/core.md", "us/places.md", "README.md"];

fn front(text: &str) -> (String, Vec<String>) {
    let mut title = String::new();
    let mut aliases = Vec::new();
    if let Some(rest) = text.strip_prefix("---") {
        if let Some(end) = rest.find("\n---") {
            for line in rest[..end].lines() {
                if let Some(v) = line.strip_prefix("title:") {
                    title = v.trim().to_string();
                } else if let Some(v) = line.strip_prefix("aliases:") {
                    aliases = v.trim().trim_start_matches('[').trim_end_matches(']').split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                }
            }
        }
    }
    (title, aliases)
}

fn walk(dir: &Path, base: &Path, out: &mut Vec<Note>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if p.is_dir() {
            // 日记一天一份，按日期找，不按字样对；原始清单不是笔记
            if name != "diary" && name != ".git" && name != "config" && name != "admin" {
                walk(&p, base, out);
            }
        } else if name.ends_with(".md") {
            let rel = p.strip_prefix(base).map(|r| r.to_string_lossy().replace('\\', "/")).unwrap_or_default();
            if ALWAYS.contains(&rel.as_str()) {
                continue;
            }
            let head: String = std::fs::read_to_string(&p).unwrap_or_default().chars().take(600).collect();
            let (title, aliases) = front(&head);
            let mut keys: Vec<String> = aliases;
            if !title.is_empty() {
                keys.push(title.clone());
            }
            out.push(Note { path: rel, title, keys });
        }
    }
}

/// 记忆里所有的笔记，带上各自的标题和别名。
pub fn notes(root: &Root) -> Vec<Note> {
    let mut out = Vec::new();
    walk(&root.memory(), &root.memory(), &mut out);
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

/// 一级：话里出现了哪些笔记的标题或别名。对上的字样越长越靠前，最多三份。两个字以下的字样不算，太容易碰巧。
pub fn local<'a>(notes: &'a [Note], text: &str) -> Vec<&'a Note> {
    let t = text.to_lowercase();
    let mut hits: Vec<(&Note, usize)> = notes.iter().filter_map(|n| n.keys.iter().filter(|k| k.chars().count() >= 2 && t.contains(&k.to_lowercase())).map(|k| k.chars().count()).max().map(|len| (n, len))).collect();
    hits.sort_by(|a, b| b.1.cmp(&a.1));
    hits.into_iter().take(3).map(|(n, _)| n).collect()
}

/// 这句话像不像在问一件要翻笔记才答得上的事。闲聊、指令不算。
pub fn asks(text: &str) -> bool {
    let t = text.trim();
    t.chars().count() >= 6 && ["？", "?", "吗", "什么", "哪", "怎么", "记得", "上次", "之前", "多少", "是不是", "有没有"].iter().any(|w| t.contains(w))
}

/// 二级：让便宜的模型从目录里挑。它只看得到目录（路径、标题、别名）和这句话，看不到笔记内容。
fn cheap_pick<'a>(root: &Root, notes: &'a [Note], text: &str) -> Vec<&'a Note> {
    let list: String = notes.iter().map(|n| format!("{}　{}　{}", n.path, n.title, n.keys.join("、"))).collect::<Vec<_>>().join("\n");
    let user = outbound::redact(&format!("笔记目录（路径　标题　别名）：\n{list}\n\n这句话：{text}"));
    let _ = outbound::log(root, &crate::mind::now(), "cheap", cheap::DEFAULT_MODEL, &user);
    let Ok(reply) = cheap::errand("下面是一份笔记目录和一句话。挑出最多两份最可能有助于回答这句话的笔记，只回路径，一行一个，照目录里的写法。都不相关就只回：无", &user, cheap::DEFAULT_MODEL, 80) else { return Vec::new() };
    notes.iter().filter(|n| reply.lines().any(|l| l.trim().trim_matches('`') == n.path)).take(2).collect()
}

fn excerpt(root: &Root, note: &Note) -> String {
    let text = std::fs::read_to_string(root.memory().join(&note.path)).unwrap_or_default();
    let body = match text.strip_prefix("---").and_then(|r| r.find("\n---").map(|i| &r[i + 4..])) {
        Some(b) => b,
        None => &text,
    };
    let body = body.trim();
    if body.chars().count() > 700 {
        format!("{}\n……（后面还有，要看全文就去读这份笔记）", body.chars().take(700).collect::<String>())
    } else {
        body.to_string()
    }
}

/// 程序替它先翻到的笔记，写成附在机主那句话前面的一段。没翻到就是空的。
pub fn attach(root: &Root, text: &str, allow_cheap: bool) -> String {
    let all = notes(root);
    let mut found = local(&all, text);
    let mut how = "话里提到了";
    if found.is_empty() && allow_cheap && asks(text) {
        found = cheap_pick(root, &all, text);
        how = "便宜的模型从目录里挑的";
    }
    if found.is_empty() {
        return String::new();
    }
    let mut out = format!("〔程序先替你翻到的笔记（{how}）。用得上就用，用不上不必提：\n");
    for n in found {
        out.push_str(&format!("\n—— {} ——\n{}\n", n.path, excerpt(root, n)));
    }
    out.push_str("〕\n\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> (tempfile::TempDir, Root) {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("memory").join("learned")).unwrap();
        std::fs::create_dir_all(d.path().join("memory").join("diary")).unwrap();
        std::fs::create_dir_all(d.path().join("seed")).unwrap();
        let m = d.path().join("memory");
        std::fs::write(m.join("standing.md"), "---\ntitle: 底稿\n---\n每次都带").unwrap();
        std::fs::write(m.join("learned").join("rust.md"), "---\ntitle: Rust 的借用\naliases: [借用检查, borrow checker]\n---\n\n# Rust 的借用\n\n同一时间只能有一个可变借用。").unwrap();
        std::fs::write(m.join("learned").join("tauri.md"), "---\ntitle: Tauri 的窗口\naliases: [点击穿透]\n---\n\n透明窗口要自己管点击穿透。").unwrap();
        std::fs::write(m.join("diary").join("2026-10-12.md"), "---\ntitle: 日记\naliases: [借用检查]\n---\n不该被翻到").unwrap();
        let r = Root::at(d.path()).unwrap();
        (d, r)
    }

    #[test]
    fn notes_are_listed_with_titles_and_aliases_but_not_the_always_loaded_or_the_diary() {
        let (_d, r) = root();
        let all = notes(&r);
        assert_eq!(all.iter().map(|n| n.path.as_str()).collect::<Vec<_>>(), ["learned/rust.md", "learned/tauri.md"]);
        assert!(all[0].keys.contains(&"borrow checker".to_string()));
    }

    #[test]
    fn a_title_or_alias_in_the_sentence_finds_the_note() {
        let (_d, r) = root();
        let all = notes(&r);
        assert_eq!(local(&all, "上次那个 Borrow Checker 的报错是怎么回事")[0].path, "learned/rust.md");
        assert_eq!(local(&all, "点击穿透又坏了")[0].path, "learned/tauri.md");
        assert!(local(&all, "今天天气不错").is_empty());
    }

    #[test]
    fn the_attachment_carries_the_body_without_the_front_matter() {
        let (_d, r) = root();
        let a = attach(&r, "借用检查是什么来着", false);
        assert!(a.contains("learned/rust.md") && a.contains("同一时间只能有一个可变借用"), "{a}");
        assert!(!a.contains("aliases"), "{a}");
        assert_eq!(attach(&r, "你好", false), "");
    }

    #[test]
    fn only_real_questions_go_to_the_second_level() {
        assert!(asks("我上次改 DNS 是为了什么？"));
        assert!(asks("还记得那个脚本放哪了吗"));
        assert!(!asks("打开 DeepL"));
        assert!(!asks("好的"));
        assert!(!asks("嗯？"));
    }
}
