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
                bail!("没找到它的数据目录。在它的目录里运行，或用 --root 指定。");
            }
        }
    }

    /// 安装版的数据放在哪：用户目录下的 YourDeviceSoul。记忆是给人看、给人改的，所以放在好找的地方；卸载和升级程序不会动它。
    pub fn data_dir() -> PathBuf {
        let base = std::env::var_os("USERPROFILE").map(PathBuf::from).unwrap_or_else(|| std::env::temp_dir());
        base.join("YourDeviceSoul")
    }

    /// 找数据目录的完整次序：明确给了的；当前目录往上；程序所在的目录往上；安装版的数据目录。
    pub fn locate(explicit: Option<PathBuf>) -> Result<Root> {
        if explicit.is_some() {
            return Root::discover(explicit);
        }
        Root::discover(None).or_else(|_| Root::near_exe()).or_else(|_| Root::at(&Root::data_dir()))
    }

    /// 安装版每次启动时做的事：把随程序带来的初始文件（程序旁边的 skeleton 目录）铺到数据目录里。
    /// 属于用户的（记忆、种子、提示词、私人目录的名单）只在没有的时候才放，永远不覆盖；
    /// 属于程序的（清点电脑用的脚本、辅助脚本）每次都换成新的，这样升级之后脚本也是新的。
    pub fn bootstrap() -> Result<Root> {
        let exe_dir = std::env::current_exe()?.parent().map(|p| p.to_path_buf()).unwrap_or_default();
        Root::bootstrap_from(&exe_dir.join("skeleton"), &Root::data_dir())
    }

    pub fn bootstrap_from(skeleton: &Path, data: &Path) -> Result<Root> {
        if !skeleton.is_dir() {
            if is_root(data) {
                return Ok(Root(data.to_path_buf()));
            }
            bail!("没找到初始文件（{}）。程序可能没有装完整。", skeleton.display());
        }
        for (name, overwrite) in [("memory", false), ("seed", false), ("prompts", false), ("sensors", true), ("tools", true)] {
            let src = skeleton.join(name);
            if src.is_dir() {
                sync(&src, &data.join(name), overwrite)?;
            }
        }
        Root::at(data)
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

/// 把一个目录拷到另一处。overwrite 为假时，已经有的文件不动。私人目录的名单属于用户，任何时候都不覆盖。
fn sync(src: &Path, dst: &Path, overwrite: bool) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let (from, to) = (entry.path(), dst.join(entry.file_name()));
        if from.is_dir() {
            sync(&from, &to, overwrite)?;
        } else if !to.exists() || (overwrite && entry.file_name() != "shallow-dirs.txt") {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
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

    #[test]
    fn first_run_lays_out_the_starting_files_and_later_runs_keep_what_is_the_user_s() {
        let d = tempfile::tempdir().unwrap();
        let (sk, data) = (d.path().join("skeleton"), d.path().join("data"));
        for (f, body) in [("memory/index.md", "空的索引"), ("seed/temperament.md", "默认的种子"), ("prompts/system.md", "规矩一"), ("sensors/census/census.ps1", "脚本一"), ("sensors/census/shallow-dirs.txt", "# 空的名单")] {
            std::fs::create_dir_all(sk.join(f).parent().unwrap()).unwrap();
            std::fs::write(sk.join(f), body).unwrap();
        }
        let root = Root::bootstrap_from(&sk, &data).unwrap();
        assert_eq!(std::fs::read_to_string(root.memory().join("index.md")).unwrap(), "空的索引");
        // 用户用了一阵：记了东西、改了规矩、填了私人目录
        std::fs::write(data.join("memory/index.md"), "它记的").unwrap();
        std::fs::write(data.join("prompts/system.md"), "他改过的规矩").unwrap();
        std::fs::write(data.join("sensors/census/shallow-dirs.txt"), "D:/私人").unwrap();
        // 升级：随程序来的脚本和默认文件都变了
        std::fs::write(sk.join("memory/index.md"), "新版的空索引").unwrap();
        std::fs::write(sk.join("prompts/system.md"), "规矩二").unwrap();
        std::fs::write(sk.join("sensors/census/census.ps1"), "脚本二").unwrap();
        Root::bootstrap_from(&sk, &data).unwrap();
        assert_eq!(std::fs::read_to_string(data.join("memory/index.md")).unwrap(), "它记的", "记忆不能被覆盖");
        assert_eq!(std::fs::read_to_string(data.join("prompts/system.md")).unwrap(), "他改过的规矩");
        assert_eq!(std::fs::read_to_string(data.join("sensors/census/shallow-dirs.txt")).unwrap(), "D:/私人", "私人目录的名单不能被覆盖");
        assert_eq!(std::fs::read_to_string(data.join("sensors/census/census.ps1")).unwrap(), "脚本二", "程序的脚本要换成新的");
        // 没有初始文件、数据目录也不成样子：报错，不硬来
        assert!(Root::bootstrap_from(&d.path().join("nothing"), &d.path().join("empty")).is_err());
        assert!(Root::bootstrap_from(&d.path().join("nothing"), &data).is_ok(), "初始文件没了但数据还在：照常用");
    }
}
