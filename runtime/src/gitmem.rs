//! 记忆有改动就提交。提交说明以 `mem:` 开头，和代码的提交区分开。

use crate::paths::Root;
use anyhow::{bail, Context as _, Result};
use std::process::Command;

fn git(root: &Root, args: &[&str]) -> Result<String> {
    let mut cmd = Command::new("git");
    crate::quiet::quiet(&mut cmd);
    let out = cmd.arg("-C").arg(root.path()).args(args).output().context("运行不了 git")?;
    if !out.status.success() {
        bail!("git {} 失败：{}", args.join(" "), String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// 记忆目录里有哪些文件变了。
pub fn changed(root: &Root) -> Result<Vec<String>> {
    let out = git(root, &["status", "--porcelain", "--untracked-files=all", "--", "memory"])?;
    Ok(out.lines().filter(|l| l.len() > 3).map(|l| l[3..].trim().trim_matches('"').to_string()).collect())
}

/// 记忆有改动就提交，返回这次提交了哪些文件；没有改动就什么都不做。
pub fn commit_memory(root: &Root, reason: &str) -> Result<Vec<String>> {
    let files = changed(root)?;
    if files.is_empty() {
        return Ok(files);
    }
    git(root, &["add", "--", "memory"])?;
    git(root, &["commit", "--quiet", "-m", &format!("mem: {reason}"), "--", "memory"])?;
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn repo() -> (tempfile::TempDir, Root) {
        let t = tempfile::tempdir().unwrap();
        fs::create_dir_all(t.path().join("memory/you")).unwrap();
        fs::create_dir_all(t.path().join("seed")).unwrap();
        fs::write(t.path().join("seed/temperament.md"), "x").unwrap();
        let root = Root::at(t.path()).unwrap();
        for args in [
            vec!["init", "--quiet", "-b", "main"],
            vec!["config", "user.name", "test"],
            vec!["config", "user.email", "test@example.com"],
            vec!["config", "commit.gpgsign", "false"],
            vec!["add", "-A"],
            vec!["commit", "--quiet", "-m", "chore: init"],
        ] {
            git(&root, &args).unwrap();
        }
        (t, root)
    }

    #[test]
    fn nothing_changed_means_no_commit() {
        let (_t, root) = repo();
        assert!(commit_memory(&root, "没事").unwrap().is_empty());
        assert_eq!(git(&root, &["rev-list", "--count", "HEAD"]).unwrap().trim(), "1");
    }

    #[test]
    fn a_new_note_is_committed_with_the_mem_prefix() {
        let (t, root) = repo();
        fs::write(t.path().join("memory/you/said.md"), "2026-10-11 周六是出门，不是休息\n").unwrap();
        let files = commit_memory(&root, "记下他说的话").unwrap();
        assert_eq!(files, ["memory/you/said.md"]);
        assert_eq!(git(&root, &["log", "-1", "--format=%s"]).unwrap().trim(), "mem: 记下他说的话");
        assert!(changed(&root).unwrap().is_empty());
    }

    #[test]
    fn changes_outside_memory_are_left_alone() {
        let (t, root) = repo();
        fs::write(t.path().join("seed/temperament.md"), "改了").unwrap();
        fs::write(t.path().join("memory/you/said.md"), "一条\n").unwrap();
        commit_memory(&root, "只提交记忆").unwrap();
        let rest = git(&root, &["status", "--porcelain"]).unwrap();
        assert!(rest.contains("seed/temperament.md"));
    }
}
