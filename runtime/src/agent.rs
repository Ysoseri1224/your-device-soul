//! 自己跑的工具循环：给不是 Claude Code 的通道用。
//!
//! Claude Code 自带读写文件的工具和"调用前先问钩子"的机制。别的通道只有一个对话接口，所以这里补上三样：
//!   · 五样文件工具（读、列、搜、写、改），每次调用前在进程内过同一道门（gate::decide），规矩和钩子那边一字不差；
//!   · 它自己的那些工具（now、run、ticket……）直接调 mcp 里的同一个函数；
//!   · 一圈一圈地问：模型要用工具就替它用，把结果交回去，直到它给出一段话。最多二十四圈。
//! 两种说话的格式（Anthropic 的 Messages、OpenAI 的 chat/completions）在这里各写一遍，别的都共用。

use crate::channels::{Channel, Wire};
use crate::paths::Root;
use crate::strong::Reply;
use crate::{gate, mcp, places};
use anyhow::{bail, Context as _, Result};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::Duration;

const MAX_TURNS: usize = 24;

fn file_tools() -> Value {
    let s = |props: Value, req: &[&str]| json!({ "type": "object", "properties": props, "required": req });
    json!([
        { "name": "read_file", "description": "读一个文本文件。路径不写全就是相对你的记忆目录。太长的只给开头，可以用 offset 和 limit 接着读。", "inputSchema": s(json!({ "path": { "type": "string" }, "offset": { "type": "integer", "description": "从第几行开始，从 1 数" }, "limit": { "type": "integer", "description": "读多少行，默认 400" } }), &["path"]) },
        { "name": "list_files", "description": "列出一个目录下面的文件（往下找）。可以给一个名字里要含的字样，比如 .md。最多给 300 个。", "inputSchema": s(json!({ "path": { "type": "string" }, "contains": { "type": "string" } }), &["path"]) },
        { "name": "search_text", "description": "在一个目录下面的文本文件里找一个字样（正则）。给出在哪个文件第几行。最多 120 处。", "inputSchema": s(json!({ "pattern": { "type": "string" }, "path": { "type": "string" } }), &["pattern", "path"]) },
        { "name": "write_file", "description": "写一个文件（没有就建，有就整个换掉）。", "inputSchema": s(json!({ "path": { "type": "string" }, "content": { "type": "string" } }), &["path", "content"]) },
        { "name": "edit_file", "description": "把文件里的一段字换成另一段。old 必须在文件里正好出现一次。", "inputSchema": s(json!({ "path": { "type": "string" }, "old": { "type": "string" }, "new": { "type": "string" } }), &["path", "old", "new"]) }
    ])
}

fn all_tools() -> Vec<Value> {
    let mut all: Vec<Value> = file_tools().as_array().cloned().unwrap_or_default();
    all.extend(mcp::tool_list().as_array().cloned().unwrap_or_default());
    all
}

fn resolve(root: &Root, path: &str) -> PathBuf {
    let p = Path::new(path);
    if p.is_absolute() { p.to_path_buf() } else { root.memory().join(p) }
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>, cap: usize) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        if out.len() >= cap {
            return;
        }
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if p.is_dir() {
            if name != ".git" && name != "node_modules" && name != "target" {
                walk(&p, out, cap);
            }
        } else {
            out.push(p);
        }
    }
}

/// 替模型用一次工具。先过门，过不去就把原因交回给它。
pub fn use_tool(root: &Root, name: &str, a: &Value) -> String {
    let text = |k: &str| a.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let gate_name = match name {
        "read_file" => "Read",
        "list_files" => "Glob",
        "search_text" => "Grep",
        "write_file" => "Write",
        "edit_file" => "Edit",
        _ => {
            return match mcp::call_tool(root, name, a) {
                Ok(t) => t,
                Err(e) => format!("没做成：{e:#}"),
            }
        }
    };
    let path = resolve(root, &text("path"));
    let key = if gate_name == "Glob" || gate_name == "Grep" { "path" } else { "file_path" };
    let (ok, why) = gate::decide(root, &places::load(root), &root.memory(), gate_name, &json!({ key: path }));
    if !ok {
        return format!("被拦下了：{why}");
    }
    let done: Result<String> = (|| match name {
        "read_file" => {
            let body = std::fs::read_to_string(&path).with_context(|| format!("读不了 {}", path.display()))?;
            let (from, n) = (a.get("offset").and_then(Value::as_u64).unwrap_or(1).max(1) as usize, a.get("limit").and_then(Value::as_u64).unwrap_or(400) as usize);
            let total = body.lines().count();
            let part: Vec<&str> = body.lines().skip(from - 1).take(n).collect();
            let mut out: String = part.join("\n").chars().take(60_000).collect();
            if from - 1 + part.len() < total {
                out.push_str(&format!("\n……（共 {total} 行，这里是第 {from}–{} 行）", from - 1 + part.len()));
            }
            Ok(out)
        }
        "list_files" => {
            let mut files = Vec::new();
            walk(&path, &mut files, 4000);
            let needle = text("contains").to_lowercase();
            let shown: Vec<String> = files.iter().map(|p| p.display().to_string()).filter(|p| needle.is_empty() || p.to_lowercase().contains(&needle)).take(300).collect();
            Ok(if shown.is_empty() { "（没有）".into() } else { shown.join("\n") })
        }
        "search_text" => {
            let re = regex::Regex::new(&text("pattern")).context("这个字样不是合法的正则")?;
            let mut files = Vec::new();
            if path.is_file() { files.push(path.clone()) } else { walk(&path, &mut files, 4000) }
            let mut hits = Vec::new();
            for f in files {
                if std::fs::metadata(&f).map(|m| m.len() > 1_500_000).unwrap_or(true) {
                    continue;
                }
                let Ok(body) = std::fs::read_to_string(&f) else { continue };
                for (i, line) in body.lines().enumerate() {
                    if re.is_match(line) {
                        hits.push(format!("{}:{}: {}", f.display(), i + 1, line.trim().chars().take(200).collect::<String>()));
                        if hits.len() >= 120 {
                            break;
                        }
                    }
                }
                if hits.len() >= 120 {
                    break;
                }
            }
            Ok(if hits.is_empty() { "（没找到）".into() } else { hits.join("\n") })
        }
        "write_file" => {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(&path, text("content"))?;
            Ok(format!("写好了：{}", path.display()))
        }
        _ => {
            let body = std::fs::read_to_string(&path).with_context(|| format!("读不了 {}", path.display()))?;
            let (old, new) = (text("old"), text("new"));
            match body.matches(&old).count() {
                0 => bail!("文件里没有这段字"),
                1 => {
                    std::fs::write(&path, body.replacen(&old, &new, 1))?;
                    Ok(format!("改好了：{}", path.display()))
                }
                n => bail!("这段字在文件里出现了 {n} 次，说不清改哪一处。多带一点上下文"),
            }
        }
    })();
    match done {
        Ok(t) => crate::outbound::redact(&t),
        Err(e) => format!("没做成：{e:#}"),
    }
}

fn post(ch: &Channel, url: &str, body: &Value) -> Result<Value> {
    let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(180)).build();
    let mut req = agent.post(url).set("Content-Type", "application/json").set("X-Title", "ysoul");
    if !ch.key.is_empty() {
        req = req.set("Authorization", &format!("Bearer {}", ch.key));
        if ch.wire == Wire::Anthropic {
            req = req.set("x-api-key", &ch.key);
        }
    }
    if ch.wire == Wire::Anthropic {
        req = req.set("anthropic-version", "2023-06-01");
    }
    match req.send_json(body.clone()) {
        Ok(r) => Ok(r.into_json()?),
        Err(ureq::Error::Status(code, r)) => {
            let detail: String = r.into_string().unwrap_or_default().chars().take(300).collect();
            bail!("{} 返回 {code}：{detail}", ch.name)
        }
        Err(e) => bail!("连不上 {}：{e}", ch.name),
    }
}

/// Anthropic 格式里的一步：从回答里取出文字和要用的工具。
pub fn parse_anthropic(resp: &Value) -> (String, Vec<(String, String, Value)>) {
    let mut text = String::new();
    let mut calls = Vec::new();
    for b in resp["content"].as_array().map(|a| a.as_slice()).unwrap_or(&[]) {
        match b["type"].as_str() {
            Some("text") => text.push_str(b["text"].as_str().unwrap_or("")),
            Some("tool_use") => calls.push((b["id"].as_str().unwrap_or("").to_string(), b["name"].as_str().unwrap_or("").to_string(), b["input"].clone())),
            _ => {}
        }
    }
    (text, calls)
}

/// OpenAI 格式里的一步。参数是一段 JSON 文字，要再解一次。
pub fn parse_openai(resp: &Value) -> (String, Vec<(String, String, Value)>) {
    let msg = &resp["choices"][0]["message"];
    let text = msg["content"].as_str().unwrap_or("").to_string();
    let calls = msg["tool_calls"].as_array().map(|a| a.iter().map(|c| (c["id"].as_str().unwrap_or("").to_string(), c["function"]["name"].as_str().unwrap_or("").to_string(), serde_json::from_str(c["function"]["arguments"].as_str().unwrap_or("{}")).unwrap_or(json!({})))).collect()).unwrap_or_default();
    (text, calls)
}

fn session_file(root: &Root, id: &str) -> PathBuf {
    root.local().join("sessions").join(format!("{id}.json"))
}

/// 一场对话太长了就丢掉最早的几轮。从一句机主的话开始截，免得把"用工具"和"工具的结果"拆散。
pub fn trim(messages: Vec<Value>, keep: usize) -> Vec<Value> {
    if messages.len() <= keep {
        return messages;
    }
    let start = messages.len() - keep;
    let cut = (start..messages.len()).find(|&i| messages[i]["role"] == "user" && messages[i]["content"].is_string()).unwrap_or(start);
    messages[cut..].to_vec()
}

/// 经由一条不是 Claude Code 的通道问一句。
pub fn ask(root: &Root, ch: &Channel, system: &str, user: &str, resume: Option<&str>) -> Result<Reply> {
    let tools = all_tools();
    let prefix = if ch.wire == Wire::Anthropic { "a" } else { "o" };
    // 只接着同一种格式的会话聊；换了通道就另起一场
    let prior: Vec<Value> = resume.filter(|id| id.starts_with(prefix)).and_then(|id| std::fs::read_to_string(session_file(root, id)).ok()).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    let id = resume.filter(|id| id.starts_with(prefix) && !prior.is_empty()).map(|s| s.to_string()).unwrap_or_else(|| format!("{prefix}{}", chrono::Local::now().format("%Y%m%d%H%M%S%3f")));
    let mut messages = trim(prior, 40);
    messages.push(json!({ "role": "user", "content": user }));
    let mut answer = String::new();
    for turn in 0..MAX_TURNS {
        let (text, calls) = if ch.wire == Wire::Anthropic {
            let defs: Vec<Value> = tools.iter().map(|t| json!({ "name": t["name"], "description": t["description"], "input_schema": t["inputSchema"] })).collect();
            let resp = post(ch, &format!("{}/v1/messages", ch.base_url), &json!({ "model": ch.model, "max_tokens": 4096, "system": system, "messages": messages, "tools": defs }))?;
            if let Some(e) = resp["error"]["message"].as_str() {
                bail!("{} 报错：{e}", ch.name);
            }
            messages.push(json!({ "role": "assistant", "content": resp["content"] }));
            parse_anthropic(&resp)
        } else {
            let defs: Vec<Value> = tools.iter().map(|t| json!({ "type": "function", "function": { "name": t["name"], "description": t["description"], "parameters": t["inputSchema"] } })).collect();
            let mut all = vec![json!({ "role": "system", "content": system })];
            all.extend(messages.iter().cloned());
            let resp = post(ch, &format!("{}/chat/completions", ch.base_url), &json!({ "model": ch.model, "messages": all, "tools": defs }))?;
            if let Some(e) = resp["error"]["message"].as_str() {
                bail!("{} 报错：{e}", ch.name);
            }
            messages.push(resp["choices"][0]["message"].clone());
            parse_openai(&resp)
        };
        if calls.is_empty() {
            answer = text;
            break;
        }
        let results: Vec<(String, String)> = calls.iter().map(|(cid, name, input)| (cid.clone(), use_tool(root, name, input))).collect();
        if ch.wire == Wire::Anthropic {
            messages.push(json!({ "role": "user", "content": results.iter().map(|(cid, out)| json!({ "type": "tool_result", "tool_use_id": cid, "content": out })).collect::<Vec<_>>() }));
        } else {
            for (cid, out) in results {
                messages.push(json!({ "role": "tool", "tool_call_id": cid, "content": out }));
            }
        }
        if turn + 1 == MAX_TURNS {
            answer = format!("{text}\n（这件事我用了 {MAX_TURNS} 轮工具还没做完，先停在这里。）");
        }
    }
    if answer.trim().is_empty() {
        bail!("{} 没有给出回答", ch.name);
    }
    std::fs::create_dir_all(root.local().join("sessions"))?;
    std::fs::write(session_file(root, &id), serde_json::to_string(&messages)?)?;
    Ok(Reply { text: answer, session_id: Some(id), model: Some(format!("{}（经由 {}）", ch.model, ch.name)) })
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
    fn both_wire_formats_yield_the_same_shape() {
        let a = json!({ "content": [{ "type": "text", "text": "我看一下。" }, { "type": "tool_use", "id": "t1", "name": "read_file", "input": { "path": "you/said.md" } }] });
        let (text, calls) = parse_anthropic(&a);
        assert_eq!(text, "我看一下。");
        assert_eq!(calls, vec![("t1".to_string(), "read_file".to_string(), json!({ "path": "you/said.md" }))]);
        let o = json!({ "choices": [{ "message": { "content": null, "tool_calls": [{ "id": "c1", "type": "function", "function": { "name": "read_file", "arguments": "{\"path\":\"you/said.md\"}" } }] } }] });
        let (text, calls) = parse_openai(&o);
        assert_eq!(text, "");
        assert_eq!(calls, vec![("c1".to_string(), "read_file".to_string(), json!({ "path": "you/said.md" }))]);
    }

    #[test]
    fn file_tools_work_inside_memory_and_are_gated_outside() {
        let (d, r) = root();
        assert!(use_tool(&r, "write_file", &json!({ "path": "you/a.md", "content": "第一行\n第二行\n" })).contains("写好了"));
        assert!(use_tool(&r, "read_file", &json!({ "path": "you/a.md" })).contains("第二行"));
        assert!(use_tool(&r, "edit_file", &json!({ "path": "you/a.md", "old": "第二行", "new": "改过的" })).contains("改好了"));
        assert!(use_tool(&r, "search_text", &json!({ "pattern": "改过", "path": "." })).contains("a.md:2"));
        assert!(use_tool(&r, "list_files", &json!({ "path": ".", "contains": ".md" })).contains("a.md"));
        // 记忆之外：没在地图上放开的地方，写不了；这和 Claude Code 那边的钩子是同一道门
        let outside = d.path().join("outside.txt");
        let out = use_tool(&r, "write_file", &json!({ "path": outside, "content": "x" }));
        assert!(out.starts_with("被拦下了"), "{out}");
        assert!(!outside.exists());
        assert!(use_tool(&r, "edit_file", &json!({ "path": "you/a.md", "old": "不存在的", "new": "x" })).contains("没做成"));
    }

    #[test]
    fn every_tool_has_a_name_a_description_and_a_schema() {
        let all = all_tools();
        assert!(all.len() > 10);
        assert!(all.iter().all(|t| t["name"].is_string() && t["description"].is_string() && t["inputSchema"]["type"] == "object"));
        assert!(all.iter().any(|t| t["name"] == "ticket") && all.iter().any(|t| t["name"] == "read_file"));
    }

    #[test]
    fn trimming_starts_at_an_owner_message() {
        let msgs = vec![
            json!({ "role": "user", "content": "一" }),
            json!({ "role": "assistant", "content": [{ "type": "tool_use" }] }),
            json!({ "role": "user", "content": [{ "type": "tool_result" }] }),
            json!({ "role": "assistant", "content": "好" }),
            json!({ "role": "user", "content": "二" }),
            json!({ "role": "assistant", "content": "嗯" }),
        ];
        let t = trim(msgs.clone(), 4);
        assert_eq!(t[0]["content"], "二", "不能从工具结果那一条开始");
        assert_eq!(trim(msgs.clone(), 10).len(), 6);
    }
}
