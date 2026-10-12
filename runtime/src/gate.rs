//! 那道门：强模型每次要读、要写一个文件之前，先过这里。
//!
//! Claude Code 在调用它的文件工具之前，会把"要用哪个工具、对哪个路径"交给一个钩子程序。
//! 这个钩子就是 `ysoul gate`：它去对家的地图，行就放行，不行就拦下并说明原因。
//! 判断只看"对哪个路径做什么"，不看模型自己怎么解释（见 10 第二节 auto 模式的做法）。

use crate::paths::Root;
use crate::places::{self, Act, Place};
use serde_json::Value;
use std::path::{Path, PathBuf};

fn target(cwd: &Path, input: &Value, key: &str) -> Option<PathBuf> {
    let s = input.get(key)?.as_str()?;
    let p = Path::new(s);
    Some(if p.is_absolute() { p.to_path_buf() } else { cwd.join(p) })
}

/// 对一次工具调用下判断。返回放不放行，和一句说明。
pub fn decide(root: &Root, map: &[Place], cwd: &Path, tool: &str, input: &Value) -> (bool, String) {
    let (path, act) = match tool {
        "Read" => (target(cwd, input, "file_path"), Act::See),
        "Glob" | "Grep" => {
            let base = target(cwd, input, "path").unwrap_or_else(|| cwd.to_path_buf());
            // 匹配式自己可以带路径（"D:/x/**"、"../y/*"）。取通配符之前的那一截目录，按它来判断
            let pattern = if tool == "Glob" { input.get("pattern").and_then(|p| p.as_str()).unwrap_or("") } else { "" };
            let literal: String = pattern.chars().take_while(|c| !"*?[{".contains(*c)).collect();
            let dir = match literal.rfind(['/', '\\']) {
                Some(i) => {
                    let lead = Path::new(&literal[..=i]);
                    if lead.is_absolute() { lead.to_path_buf() } else { base.join(lead) }
                }
                None => base,
            };
            if pattern[literal.len()..].contains("..") {
                return (false, "匹配式里通配符后面还有 ..，认不准它要搜哪里，不放行。".into());
            }
            if places::hides_something(map, &dir) {
                return (false, format!("{} 下面有不能看的地方，不能整个搜。缩小到具体的子目录再搜。", dir.display()));
            }
            (Some(dir), Act::See)
        }
        "Edit" | "MultiEdit" | "NotebookEdit" => (target(cwd, input, "file_path").or_else(|| target(cwd, input, "notebook_path")), Act::Edit),
        "Write" => {
            let p = target(cwd, input, "file_path");
            let act = if p.as_ref().is_some_and(|p| p.exists()) { Act::Edit } else { Act::Put };
            (p, act)
        }
        // 它自己的那几样工具各自把关，这里不拦
        t if t.starts_with("mcp__ysoul__") => return (true, "它自己的工具".into()),
        other => return (false, format!("不认识的工具 {other}，不放行")),
    };
    match path {
        Some(p) if act != Act::See && places::norm(&p.to_string_lossy()) == places::norm(&places::file(root).to_string_lossy()) => (false, "地图不能直接改。用 place 工具递一张票，机主点了头，程序会把那一行写进去。".into()),
        Some(p) => places::allowed(map, root, &p, act),
        None => (false, "这次调用没有写明路径，不放行".into()),
    }
}

/// 读钩子送来的那份 JSON，下判断。
pub fn from_hook(root: &Root, json: &str) -> (bool, String) {
    let v: Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(_) => return (false, "钩子送来的不是预期的格式，不放行".into()),
    };
    let tool = v.get("tool_name").and_then(|t| t.as_str()).unwrap_or("");
    let cwd = v.get("cwd").and_then(|c| c.as_str()).map(PathBuf::from).unwrap_or_else(|| root.memory());
    let empty = Value::Null;
    decide(root, &places::load(root), &cwd, tool, v.get("tool_input").unwrap_or(&empty))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn root() -> (tempfile::TempDir, Root) {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("memory")).unwrap();
        std::fs::create_dir_all(d.path().join("seed")).unwrap();
        let r = Root::at(d.path()).unwrap();
        (d, r)
    }
    fn map() -> Vec<Place> {
        places::parse("| 地方 | 是什么 | 看 | 放 | 改 | 扔 | 运行 | 依据 |\n|---|---|---|---|---|---|---|---|\n| `D:\\personal` | 私人的 | – | – | – | – | – | 你说的 |\n| `D:\\proj` | 项目 | ✓ | ✓ | ✓ | – | – | 你说的 |\n")
    }

    #[test]
    fn reading_a_private_place_is_blocked() {
        let (_d, r) = root();
        assert!(!decide(&r, &map(), &r.memory(), "Read", &json!({ "file_path": "D:\\personal\\x.md" })).0);
    }

    #[test]
    fn writing_in_memory_with_a_relative_path_is_allowed() {
        let (_d, r) = root();
        assert!(decide(&r, &map(), &r.memory(), "Write", &json!({ "file_path": "you/said.md" })).0);
    }

    #[test]
    fn editing_an_unmapped_file_is_blocked_but_reading_it_is_not() {
        let (_d, r) = root();
        assert!(decide(&r, &map(), &r.memory(), "Read", &json!({ "file_path": "C:\\Windows\\win.ini" })).0);
        assert!(!decide(&r, &map(), &r.memory(), "Edit", &json!({ "file_path": "C:\\Windows\\win.ini" })).0);
    }

    #[test]
    fn a_mapped_project_can_be_edited() {
        let (_d, r) = root();
        assert!(decide(&r, &map(), &r.memory(), "Edit", &json!({ "file_path": "D:\\proj\\a.rs" })).0);
    }

    #[test]
    fn a_search_without_a_path_is_judged_by_where_it_runs() {
        let (_d, r) = root();
        assert!(decide(&r, &map(), &r.memory(), "Grep", &json!({ "pattern": "x" })).0);
        assert!(!decide(&r, &map(), Path::new("D:\\personal"), "Glob", &json!({ "pattern": "*" })).0);
    }

    #[test]
    fn the_map_itself_can_be_read_but_not_edited_directly() {
        let (_d, r) = root();
        assert!(decide(&r, &map(), &r.memory(), "Read", &json!({ "file_path": "us/places.md" })).0);
        assert!(!decide(&r, &map(), &r.memory(), "Edit", &json!({ "file_path": "us/places.md" })).0);
        assert!(!decide(&r, &map(), &r.memory(), "Write", &json!({ "file_path": places::file(&r) })).0);
    }

    #[test]
    fn searching_a_folder_above_a_hidden_place_is_blocked() {
        let (_d, r) = root();
        assert!(!decide(&r, &map(), &r.memory(), "Grep", &json!({ "pattern": "password", "path": "D:\\" })).0);
        assert!(!decide(&r, &map(), &r.memory(), "Glob", &json!({ "pattern": "**/*.md", "path": "D:/" })).0);
        assert!(decide(&r, &map(), &r.memory(), "Grep", &json!({ "pattern": "x", "path": "D:\\proj" })).0);
    }

    #[test]
    fn a_glob_pattern_carrying_its_own_path_is_judged_by_that_path() {
        let (_d, r) = root();
        assert!(!decide(&r, &map(), &r.memory(), "Glob", &json!({ "pattern": "D:/personal/**/*" })).0);
        assert!(!decide(&r, &map(), Path::new("D:\\proj"), "Glob", &json!({ "pattern": "../personal/*" })).0);
        assert!(!decide(&r, &map(), Path::new("D:\\proj"), "Glob", &json!({ "pattern": "*/../../personal/*" })).0);
        assert!(decide(&r, &map(), &r.memory(), "Glob", &json!({ "pattern": "D:/proj/src/**/*.rs" })).0);
    }

    #[test]
    fn dot_dot_does_not_get_out_of_memory() {
        let (_d, r) = root();
        assert!(!decide(&r, &map(), &r.memory(), "Write", &json!({ "file_path": "../prompts/system.md" })).0);
        assert!(!decide(&r, &map(), Path::new("D:\\proj"), "Read", &json!({ "file_path": "src/../../personal/x" })).0);
    }

    #[test]
    fn an_unknown_tool_is_blocked() {
        let (_d, r) = root();
        assert!(!decide(&r, &map(), &r.memory(), "Bash", &json!({ "command": "dir" })).0);
    }

    #[test]
    fn the_hook_payload_is_understood() {
        let (_d, r) = root();
        let j = json!({ "tool_name": "Read", "tool_input": { "file_path": "C:\\x.txt" }, "cwd": r.memory() }).to_string();
        assert!(from_hook(&r, &j).0);
        assert!(!from_hook(&r, "不是 JSON").0);
    }
}
