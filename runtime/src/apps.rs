//! 器物：这台机器上装的程序，按名字找到并打开。
//!
//! 机主说"打开 DeepL"或者只说"DeepL"，它应该直接把程序开起来，而不是把路径讲一遍。
//! 清单来自系统自己的"开始"菜单（`Get-StartApps`），桌面程序和商店应用都在里面；存一份在本机，一天更新一次。
//! 打开程序没有后果可撤回的问题，所以不走票。

use crate::paths::Root;
use anyhow::{bail, Context as _, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct App {
    pub name: String,
    /// 系统认这个程序用的编号，打开时用它
    pub id: String,
}

fn cache(root: &Root) -> std::path::PathBuf {
    root.local().join("apps.json")
}

/// 这台机器上能从"开始"菜单打开的程序。
pub fn list(root: &Root) -> Vec<App> {
    let fresh = std::fs::metadata(cache(root)).and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).is_some_and(|age| age.as_secs() < 86_400);
    if fresh {
        if let Some(apps) = std::fs::read_to_string(cache(root)).ok().and_then(|s| serde_json::from_str::<Vec<App>>(&s).ok()) {
            return apps;
        }
    }
    let apps = scan().unwrap_or_default();
    if !apps.is_empty() {
        let _ = std::fs::create_dir_all(root.local());
        let _ = std::fs::write(cache(root), serde_json::to_string(&apps).unwrap_or_default());
    }
    apps
}

fn scan() -> Result<Vec<App>> {
    let mut cmd = Command::new("pwsh");
    crate::quiet::quiet(&mut cmd);
    let out = cmd
        .args(["-NoProfile", "-NonInteractive", "-Command", "[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false); Get-StartApps | ForEach-Object { \"$($_.Name)`t$($_.AppID)\" }"])
        .output()
        .context("启动不了 PowerShell 7")?;
    Ok(String::from_utf8_lossy(&out.stdout).lines().filter_map(|l| l.split_once('\t')).map(|(n, i)| App { name: n.trim().to_string(), id: i.trim().to_string() }).filter(|a| !a.name.is_empty() && !a.id.is_empty()).collect())
}

fn squash(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace() && *c != '-' && *c != '_').flat_map(|c| c.to_lowercase()).collect()
}

/// 名字对得有多准：3 一字不差，2 以它开头，1 里面含着它，0 对不上。
pub fn score(app: &str, query: &str) -> u8 {
    let (a, q) = (squash(app), squash(query));
    if q.is_empty() {
        0
    } else if a == q {
        3
    } else if a.starts_with(&q) {
        2
    } else if a.contains(&q) {
        1
    } else {
        0
    }
}

/// 按名字找程序。对得最准的在前；一样准的，名字短的在前。卸载程序、帮助文档这类不算。
pub fn find<'a>(apps: &'a [App], query: &str) -> Vec<(&'a App, u8)> {
    let junk = ["卸载", "uninstall", "帮助", "help", "readme", "说明", "website", "官网", "更新日志"];
    let mut hits: Vec<(&App, u8)> = apps.iter().filter(|a| !junk.iter().any(|j| a.name.to_lowercase().contains(j))).map(|a| (a, score(&a.name, query))).filter(|(_, s)| *s > 0).collect();
    hits.sort_by(|x, y| y.1.cmp(&x.1).then(x.0.name.chars().count().cmp(&y.0.name.chars().count())));
    hits
}

/// 机主打的这句话是不是"打开某个程序"。是的话返回程序名的那一部分，以及他有没有明说"打开"。
/// 只认短的、像名字的；长句子不认，交给模型去理解。
pub fn wants(text: &str) -> Option<(String, bool)> {
    let t = text.trim().trim_end_matches(['。', '.', '！', '!', '吧']).trim();
    let lower = t.to_lowercase();
    let prefixes = ["帮我打开", "我想打开", "我要打开", "给我打开", "打开一下", "开一下", "打开", "启动", "运行", "open ", "launch ", "start "];
    let (name, said) = match prefixes.iter().find(|p| lower.starts_with(**p)) {
        Some(p) => (t[p.len()..].trim().trim_end_matches("一下").trim(), true),
        None => (t, false),
    };
    let words = name.split_whitespace().count();
    if name.is_empty() || name.chars().count() > 24 || words > 3 || name.chars().any(|c| "，,。？?；;：:\\/".contains(c)) {
        return None;
    }
    Some((name.to_string(), said))
}

/// 该不该不问模型就直接打开：他明说了"打开"，对得上就开；他只打了个名字，要一字不差或者以它开头才开。
pub fn pick<'a>(apps: &'a [App], text: &str) -> Option<&'a App> {
    let (name, said) = wants(text)?;
    let hits = find(apps, &name);
    let (app, s) = hits.first()?;
    (said || *s >= 2).then_some(*app)
}

pub fn launch(app: &App) -> Result<()> {
    if app.id.chars().any(|c| c.is_control() || c == '"') {
        bail!("这个程序的编号不对劲，不打开");
    }
    Command::new("explorer").arg(format!("shell:AppsFolder\\{}", app.id)).spawn().context("打不开")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apps() -> Vec<App> {
        ["DeepL", "DeepL 卸载", "Visual Studio Code", "Visual Studio Installer", "微信", "Typora", "Google Chrome", "Chrome 应用"].iter().map(|n| App { name: n.to_string(), id: format!("id.{n}") }).collect()
    }

    #[test]
    fn names_are_matched_loosely_but_ranked() {
        let a = apps();
        assert_eq!(find(&a, "deepl")[0].0.name, "DeepL");
        assert_eq!(find(&a, "deepl").len(), 1, "卸载程序不算");
        assert_eq!(find(&a, "visual studio")[0].0.name, "Visual Studio Code", "一样准时名字短的在前");
        assert_eq!(find(&a, "chrome")[0].0.name, "Chrome 应用", "以它开头的排在只是含着它的前面");
        assert!(find(&a, "photoshop").is_empty());
    }

    #[test]
    fn a_bare_name_must_match_well_but_an_explicit_open_may_match_loosely() {
        let a = apps();
        assert_eq!(pick(&a, "deepL").unwrap().name, "DeepL");
        assert_eq!(pick(&a, "我想打开deepL").unwrap().name, "DeepL");
        assert_eq!(pick(&a, "打开 code").unwrap().name, "Visual Studio Code");
        assert!(pick(&a, "code").is_none(), "只打了个词、又只是含着，不能当成要打开程序");
        assert_eq!(pick(&a, "微信").unwrap().name, "微信");
        assert_eq!(pick(&a, "打开typora吧").unwrap().name, "Typora");
    }

    #[test]
    fn sentences_are_left_to_the_model() {
        let a = apps();
        assert!(pick(&a, "打开 D:\\SOUL\\docs 里那份需求文档").is_none());
        assert!(pick(&a, "deepl 和谷歌翻译哪个好？").is_none());
        assert!(pick(&a, "昨晚我弄到哪了").is_none());
        assert!(wants("").is_none());
    }
}
