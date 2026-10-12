//! 项目根目录和各处位置。

use anyhow::{bail, Result};
use std::path::{Path, PathBuf};

/// 项目的根目录，也就是同时放着 `memory/` 和 `seed/` 的那一层。
#[derive(Debug, Clone)]
pub struct Root(PathBuf);

impl Root {
    /// 明确给了就用给的；否则从当前目录往上找。
    pub fn discover(explicit: Option<PathBuf>) -> Result<Root> {
        if let Some(p) = explicit {
            return Root::at(&p);
        }
        let mut dir = std::env::current_dir()?;
        loop {
            if is_root(&dir) {
                return Ok(Root(dir));
            }
            if !dir.pop() {
                bail!("没找到项目根目录。请在 D:\\SOUL 里运行，或用 --root 指定。");
            }
        }
    }

    /// 从程序自己所在的目录往上找。桌面上那只是被双击启动的，当前目录不一定在项目里。
    pub fn near_exe() -> Result<Root> {
        let mut dir = std::env::current_exe()?;
        while dir.pop() {
            if is_root(&dir) {
                return Ok(Root(dir));
            }
        }
        bail!("没找到项目根目录：程序所在的目录往上，没有一层同时有 memory 和 seed 两个目录。")
    }

    /// 两个程序各在哪。发布包里它们并排放在根目录；开发时在各自的 target 下。先找并排的，再找开发时的位置。
    fn exe(&self, name: &str, dev: &[&str]) -> Option<PathBuf> {
        let mut places = vec![self.0.join(name)];
        if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.to_path_buf())) {
            places.push(dir.join(name));
        }
        for profile in ["release", "debug"] {
            places.push(dev.iter().fold(self.0.clone(), |p, s| p.join(s)).join("target").join(profile).join(name));
        }
        places.into_iter().find(|p| p.exists())
    }

    /// 命令行程序（也是那道门和本机 MCP 的本体）。
    pub fn cli_exe(&self) -> Option<PathBuf> {
        self.exe("ysoul.exe", &["runtime"])
    }

    /// 桌面上那只。
    pub fn app_exe(&self) -> Option<PathBuf> {
        self.exe("ysoul-app.exe", &["app", "src-tauri"])
    }

    pub fn at(path: &Path) -> Result<Root> {
        if !is_root(path) {
            bail!("{} 不是项目根目录：里面要同时有 memory 和 seed 两个目录。", path.display());
        }
        Ok(Root(path.to_path_buf()))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
    pub fn memory(&self) -> PathBuf {
        self.0.join("memory")
    }
    pub fn seed(&self) -> PathBuf {
        self.0.join("seed").join("temperament.md")
    }
    pub fn frame(&self) -> PathBuf {
        self.0.join("prompts").join("system.md")
    }
    pub fn prompt(&self, name: &str) -> PathBuf {
        self.0.join("prompts").join(name)
    }
    pub fn local(&self) -> PathBuf {
        self.0.join("local")
    }
}

fn is_root(dir: &Path) -> bool {
    dir.join("memory").is_dir() && dir.join("seed").is_dir()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn at_accepts_a_directory_with_memory_and_seed() {
        let t = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(t.path().join("memory")).unwrap();
        std::fs::create_dir_all(t.path().join("seed")).unwrap();
        let root = Root::at(t.path()).unwrap();
        assert_eq!(root.memory(), t.path().join("memory"));
    }

    #[test]
    fn at_rejects_a_directory_without_them() {
        let t = tempfile::tempdir().unwrap();
        assert!(Root::at(t.path()).is_err());
    }
}
