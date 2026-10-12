//! 出生：体检、请便宜模型做摘要、认识自己的身体、写下自述。
//!
//! 每一步做完都提交一次，所以中途断了可以从断的那一步接着来。

use crate::paths::Root;
use crate::{cheap, context, gitmem, outbound, strong};
use anyhow::{bail, Context as _, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// 送给便宜模型的一份清单最多这么多字，再长就截断。
const DIGEST_INPUT_LIMIT: usize = 60_000;

/// 已经出生的标志：它自己写的内核在不在。
pub fn is_born(root: &Root) -> bool {
    root.memory().join("me").join("core.md").is_file()
}

/// 第一步：体检。把这台机器的三层列成纯文本快照，放进 `memory/body/`。
pub fn census(root: &Root) -> Result<Vec<String>> {
    let script = root.path().join("sensors").join("census").join("census.ps1");
    let mut cmd = Command::new("pwsh");
    crate::quiet::quiet(&mut cmd);
    let status = cmd
        .args(["-NoProfile", "-File"])
        .arg(&script)
        .arg("-OutDir")
        .arg(root.memory().join("body"))
        // 普查会打印一张进度表；出生时标准输出要留给它的第一段话，所以表改走标准错误
        .stdout(std::io::stderr())
        .status()
        .context("运行不了 pwsh")?;
    if !status.success() {
        bail!("普查没有全部成功，见 memory/body/_errors.txt");
    }
    gitmem::commit_memory(root, "第一次体检")
}

/// `body/` 下所有的清单，按路径排序。
pub fn snapshot_files(body: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![body.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.is_dir() {
                stack.push(path);
            } else if matches!(path.extension().and_then(|e| e.to_str()), Some("tsv" | "txt")) {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

/// 太长的清单截到上限，并注明截掉了多少。
pub fn clip(text: &str, limit: usize) -> String {
    let total = text.chars().count();
    if total <= limit {
        return text.to_string();
    }
    let head: String = text.chars().take(limit).collect();
    format!("{head}\n\n（清单太长，以上是前 {limit} 字，后面还有 {} 字没有给你。）", total - limit)
}

fn label(body: &Path, file: &Path) -> String {
    file.strip_prefix(body).unwrap_or(file).to_string_lossy().replace('\\', "/")
}

/// 第二步：每份清单请便宜模型做一份摘要。做过的留在 `local/digest/`，重跑时不再花钱。
pub fn digests(root: &Root, now: &str) -> Result<Vec<(String, String)>> {
    let body = root.memory().join("body");
    let cache = root.local().join("digest");
    std::fs::create_dir_all(&cache)?;
    let system = std::fs::read_to_string(root.prompt("digest.md")).context("缺少 prompts/digest.md")?;
    let mut out = Vec::new();
    for file in snapshot_files(&body)? {
        let name = label(&body, &file);
        let cached = cache.join(format!("{}.md", name.replace('/', "__")));
        let digest = if cached.is_file() {
            std::fs::read_to_string(&cached)?
        } else {
            let content = clip(&std::fs::read_to_string(&file)?, DIGEST_INPUT_LIMIT);
            let user = format!("清单：{name}\n\n{content}");
            outbound::log(root, now, "cheap", cheap::DEFAULT_MODEL, &user)?;
            eprintln!("  摘要 {name} …");
            let d = cheap::errand(&system, &user, cheap::DEFAULT_MODEL, 700).with_context(|| format!("给 {name} 做摘要时出错"))?;
            std::fs::write(&cached, &d)?;
            d
        };
        out.push((name, digest));
    }
    Ok(out)
}

fn ask_strong(root: &Root, now: &str, today: &str, user: &str, resume: Option<&str>) -> Result<strong::Reply> {
    let ctx = context::assemble(root, today)?;
    outbound::log(root, now, "strong", "claude-code", &format!("{}\n\n---\n\n{user}", ctx.system))?;
    strong::ask(root, &ctx.system, user, resume)
}

/// 第三步：读摘要，写下对身体的认识、对他的初步印象、想问的问题。
pub fn body(root: &Root, now: &str, today: &str, digests: &[(String, String)]) -> Result<strong::Reply> {
    let mut user = std::fs::read_to_string(root.prompt("birth-body.md")).context("缺少 prompts/birth-body.md")?;
    for (name, d) in digests {
        user.push_str(&format!("### {name}\n\n{}\n\n", d.trim()));
    }
    let reply = ask_strong(root, now, today, &user, None)?;
    gitmem::commit_memory(root, "认识自己的身体")?;
    Ok(reply)
}

/// 第四步：写下内核、经历、底稿、索引，并说出见面的第一段话。
pub fn selfhood(root: &Root, now: &str, today: &str, session: Option<&str>) -> Result<strong::Reply> {
    let user = std::fs::read_to_string(root.prompt("birth-self.md")).context("缺少 prompts/birth-self.md")?;
    let reply = ask_strong(root, now, today, &user, session)?;
    if !is_born(root) {
        bail!("它没有写下 me/core.md，出生没有完成。它的回答是：{}", reply.text);
    }
    gitmem::commit_memory(root, "写下自述")?;
    let local = root.local();
    std::fs::create_dir_all(&local)?;
    std::fs::write(local.join("opening.md"), reply.text.trim())?;
    if let Some(id) = &reply.session_id {
        std::fs::write(local.join("last-session"), id)?;
    }
    Ok(reply)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn clip_leaves_short_text_alone() {
        assert_eq!(clip("短", 10), "短");
    }

    #[test]
    fn clip_cuts_long_text_and_says_how_much_is_missing() {
        let out = clip(&"字".repeat(25), 10);
        assert!(out.starts_with(&"字".repeat(10)));
        assert!(out.contains("后面还有 15 字"));
    }

    #[test]
    fn snapshot_files_finds_tsv_and_txt_recursively_in_order() {
        let t = tempfile::tempdir().unwrap();
        fs::create_dir_all(t.path().join("config")).unwrap();
        fs::write(t.path().join("b.tsv"), "").unwrap();
        fs::write(t.path().join("a.txt"), "").unwrap();
        fs::write(t.path().join("config").join("c.txt"), "").unwrap();
        fs::write(t.path().join(".gitkeep"), "").unwrap();
        fs::write(t.path().join("overview.md"), "").unwrap();
        let names: Vec<String> = snapshot_files(t.path()).unwrap().iter().map(|f| label(t.path(), f)).collect();
        assert_eq!(names, ["a.txt", "b.tsv", "config/c.txt"]);
    }

    #[test]
    fn born_means_core_exists() {
        let t = tempfile::tempdir().unwrap();
        fs::create_dir_all(t.path().join("memory/me")).unwrap();
        fs::create_dir_all(t.path().join("seed")).unwrap();
        let root = Root::at(t.path()).unwrap();
        assert!(!is_born(&root));
        fs::write(t.path().join("memory/me/core.md"), "我").unwrap();
        assert!(is_born(&root));
    }
}
