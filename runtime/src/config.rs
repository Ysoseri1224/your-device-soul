//! 这台机器上的设置：怎么称呼机主、哪些目录是私人的、第一次设置做过没有。
//!
//! 这些原先是写死的（称呼写在提示词里，根目录写在程序里）。现在第一次启动时由一个设置页问出来，存在本机。
//! 称呼和根目录在拼提示词时填进去（提示词里写的是 {{owner}} 和 {{root}}）。
//! 私人目录仍然只有一份名单（sensors/census/shallow-dirs.txt），普查和那道门共用；设置页改的就是它。

use crate::paths::Root;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    /// 怎么称呼机主
    #[serde(default)]
    pub owner: String,
    /// 第一次设置做过了
    #[serde(default)]
    pub setup_done: bool,
}

impl Default for Config {
    fn default() -> Config {
        Config { owner: String::new(), setup_done: false }
    }
}

fn file(root: &Root) -> std::path::PathBuf {
    root.local().join("config.json")
}

pub fn load(root: &Root) -> Config {
    std::fs::read_to_string(file(root)).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

pub fn save(root: &Root, c: &Config) -> Result<()> {
    std::fs::create_dir_all(root.local())?;
    std::fs::write(file(root), serde_json::to_string_pretty(c)?)?;
    Ok(())
}

/// 私人目录的名单。一行一个完整路径。
pub fn private_dirs(root: &Root) -> Vec<String> {
    std::fs::read_to_string(crate::places::private_file(root)).unwrap_or_default().lines().map(|l| l.trim().trim_start_matches('\u{feff}').to_string()).filter(|l| !l.is_empty() && !l.starts_with('#')).collect()
}

pub fn save_private_dirs(root: &Root, dirs: &[String]) -> Result<()> {
    let f = crate::places::private_file(root);
    if let Some(d) = f.parent() {
        std::fs::create_dir_all(d)?;
    }
    let mut clean: Vec<String> = dirs.iter().map(|d| d.trim().trim_end_matches(['\\', '/']).to_string()).filter(|d| !d.is_empty()).collect();
    clean.dedup();
    std::fs::write(f, format!("# 这些目录是私人的：普查只记\"有这么个目录\"，它连看都不能看。一行一个，写完整路径。\n# 你亲手交给它的那一件（右键、拖进来、话里写了路径）不受这份名单限制。\n{}\n", clean.join("\n")))?;
    Ok(())
}

/// 把提示词里的 {{owner}} 和 {{root}} 填上。没设过称呼就用"你的主人"，免得出现空的。
pub fn fill(root: &Root, text: &str) -> String {
    let c = load(root);
    let owner = if c.owner.trim().is_empty() { "机主" } else { c.owner.trim() };
    text.replace("{{owner}}", owner).replace("{{root}}", &root.path().display().to_string())
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
    fn settings_round_trip_and_default_to_not_set_up() {
        let (_d, r) = root();
        assert!(!load(&r).setup_done);
        save(&r, &Config { owner: "阿猫".into(), setup_done: true }).unwrap();
        assert_eq!(load(&r), Config { owner: "阿猫".into(), setup_done: true });
    }

    #[test]
    fn the_prompt_placeholders_are_filled() {
        let (d, r) = root();
        assert_eq!(fill(&r, "他叫 {{owner}}。"), "他叫 机主。");
        save(&r, &Config { owner: "阿猫".into(), setup_done: true }).unwrap();
        let out = fill(&r, "他叫 {{owner}}，文档在 {{root}}\\docs。");
        assert!(out.starts_with("他叫 阿猫，文档在 ") && out.contains(&d.path().display().to_string()), "{out}");
    }

    #[test]
    fn the_private_list_is_one_file_shared_with_the_gate() {
        let (_d, r) = root();
        save_private_dirs(&r, &["D:\\私人\\".into(), "  ".into(), "D:/photos".into()]).unwrap();
        assert_eq!(private_dirs(&r), ["D:\\私人", "D:/photos"]);
        let gate = crate::places::private_list(&r);
        assert_eq!(gate.len(), 2);
        assert!(!crate::places::allowed(&gate, &r, std::path::Path::new("D:\\photos\\a.jpg"), crate::places::Act::See).0);
    }
}
