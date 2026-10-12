//! 强模型：对话和反思。这个月的试点是经由调用 Claude Code，用机主的订阅。
//!
//! Claude Code 在这里只是被调用的一件工具。它在记忆目录里运行，所以读写都落在记忆里。

use crate::paths::Root;
use anyhow::{bail, Context as _, Result};
use serde::Deserialize;
use std::io::Write;
use std::process::{Command, Stdio};

/// 强模型的一次回答。
#[derive(Debug, PartialEq)]
pub struct Reply {
    pub text: String,
    pub session_id: Option<String>,
    pub model: Option<String>,
}

#[derive(Deserialize)]
struct Raw {
    #[serde(default)]
    result: Option<String>,
    #[serde(default)]
    is_error: bool,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default, rename = "modelUsage")]
    model_usage: serde_json::Map<String, serde_json::Value>,
}

/// 解析 Claude Code 以 JSON 格式给出的结果。
pub fn parse_reply(json: &str) -> Result<Reply> {
    let raw: Raw = serde_json::from_str(json.trim()).context("Claude Code 的输出不是预期的 JSON")?;
    let text = raw.result.unwrap_or_default();
    if raw.is_error {
        bail!("Claude Code 报错：{text}");
    }
    if text.trim().is_empty() {
        bail!("Claude Code 没有给出回答");
    }
    Ok(Reply { text, session_id: raw.session_id, model: raw.model_usage.keys().next().cloned() })
}

/// 命令行程序在哪。自己就是的话用自己；不是（桌面上那只是另一个程序）就去找。
fn cli_exe(root: &Root) -> Option<std::path::PathBuf> {
    if let Ok(me) = std::env::current_exe() {
        if me.file_stem().is_some_and(|s| s == "ysoul") {
            return Some(me);
        }
    }
    root.cli_exe()
}

/// 问强模型一句。`resume` 是上一轮的会话编号，带上它就是接着聊。
/// 走哪条通道由机主定（见 channels.rs）：按他排的次序挨条试，前一条不通就换下一条。没排过就先用 Claude Code。
pub fn ask(root: &Root, system: &str, user: &str, resume: Option<&str>) -> Result<Reply> {
    let all = crate::channels::discover();
    let order = crate::channels::strong_order(&all, &crate::channels::choice(root));
    if order.is_empty() {
        bail!("这台机器上没有找到能用的模型通道。装一个 Claude Code，或者在 cc-switch 里配一条，或者设好 OPENAI_API_KEY / ANTHROPIC_API_KEY 这样的环境变量。");
    }
    // 设了 YSOUL_DIRECT 就不借 claude 命令的道，一律自己直接问（试通道时用）
    let has_cli = all.iter().any(|c| c.wire == crate::channels::Wire::ClaudeCode) && std::env::var_os("YSOUL_DIRECT").is_none();
    let mut errors = Vec::new();
    for ch in &order {
        use crate::channels::Wire;
        let got = match ch.wire {
            Wire::ClaudeCode => ask_claude_code(root, system, user, resume, &[]),
            // 给 Claude Code 配的中转：有的只认 Claude Code 这个客户端。本机有 claude 命令时，就让它带着这条供应商的入口和密钥去问，
            // 工具和把关照旧走它的钩子；不成再退回自己直接问
            Wire::Anthropic if ch.source == "cc-switch" && has_cli => {
                let env: Vec<(String, String)> = [("ANTHROPIC_BASE_URL", &ch.base_url), ("ANTHROPIC_AUTH_TOKEN", &ch.key), ("ANTHROPIC_MODEL", &ch.model), ("ANTHROPIC_DEFAULT_HAIKU_MODEL", &ch.model), ("ANTHROPIC_DEFAULT_SONNET_MODEL", &ch.model), ("ANTHROPIC_DEFAULT_OPUS_MODEL", &ch.model)].iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
                ask_claude_code(root, system, user, resume, &env).or_else(|_| crate::agent::ask(root, ch, system, user, resume))
            }
            _ => crate::agent::ask(root, ch, system, user, resume),
        };
        match got {
            Ok(r) => return Ok(r),
            Err(e) => errors.push(format!("{}：{e:#}", ch.name)),
        }
    }
    bail!("几条通道都没问通。{}", errors.join("；"))
}

/// 经由本机的 Claude Code 问一句。
fn ask_claude_code(root: &Root, system: &str, user: &str, resume: Option<&str>, env: &[(String, String)]) -> Result<Reply> {
    let tmp = root.local().join("tmp");
    std::fs::create_dir_all(&tmp)?;
    let system_file = tmp.join(format!("system-{}.md", std::process::id()));
    std::fs::write(&system_file, system)?;
    // 只挂它自己那一个指向本机的服务。不这样写死的话，机主账号上挂的连接器会跟进来，它会把那些当成自己的事来说。
    // 文件工具每次调用前先过那道门（见 gate.rs）。命令行程序还没编出来时两样都不挂，退回到只在记忆里读写。
    let cli = cli_exe(root);
    let mcp_file = tmp.join("mcp.json");
    let settings_file = tmp.join("settings.json");
    let (mcp, settings) = match &cli {
        Some(exe) => (
            serde_json::json!({ "mcpServers": { "ysoul": { "command": exe, "args": ["--root", root.path(), "mcp"] } } }),
            serde_json::json!({ "hooks": { "PreToolUse": [{ "matcher": "Read|Glob|Grep|Edit|Write|MultiEdit|NotebookEdit", "hooks": [{ "type": "command", "command": format!("\"{}\" --root \"{}\" gate", exe.display(), root.path().display()) }] }] } }),
        ),
        None => (serde_json::json!({ "mcpServers": {} }), serde_json::json!({})),
    };
    std::fs::write(&mcp_file, mcp.to_string())?;
    std::fs::write(&settings_file, settings.to_string())?;

    // claude 是一个 .cmd，所以经由 cmd 启动。参数全是固定的；机主的话走标准输入，不进命令行。
    let mut cmd = Command::new("cmd");
    crate::quiet::quiet(&mut cmd);
    cmd.args(["/D", "/C", "claude", "-p", "--output-format", "json", "--permission-mode", "acceptEdits"])
        .args(["--allowedTools", "Read", "Glob", "Grep", "Edit", "Write", "mcp__ysoul__now", "mcp__ysoul__where", "mcp__ysoul__open", "mcp__ysoul__launch", "mcp__ysoul__reveal", "mcp__ysoul__look", "mcp__ysoul__delegate", "mcp__ysoul__ui_read", "mcp__ysoul__ui_see", "mcp__ysoul__ui_do", "mcp__ysoul__run", "mcp__ysoul__tag", "mcp__ysoul__mark", "mcp__ysoul__ticket", "mcp__ysoul__place", "mcp__ysoul__tickets"])
        .arg("--settings")
        .arg(&settings_file)
        .arg("--strict-mcp-config")
        .arg("--mcp-config")
        .arg(&mcp_file)
        .arg("--system-prompt-file")
        .arg(&system_file);
    // 别的通道留下的会话编号（a…、o… 开头）它认不得，那种情况就另起一场
    if let Some(id) = resume.filter(|id| id.contains('-')) {
        cmd.args(["--resume", id]);
    }
    cmd.envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())));
    let mut child = cmd
        .current_dir(root.memory())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("启动不了 Claude Code。确认命令行里能运行 claude。")?;
    child.stdin.take().context("拿不到标准输入")?.write_all(user.as_bytes())?;
    let out = child.wait_with_output()?;
    let _ = std::fs::remove_file(&system_file);

    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.trim().is_empty() {
        bail!("Claude Code 没有输出。{}", String::from_utf8_lossy(&out.stderr).trim());
    }
    parse_reply(&stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_normal_result() {
        let j = r#"{"type":"result","is_error":false,"result":"七天。","session_id":"abc","modelUsage":{"claude-x":{}}}"#;
        let r = parse_reply(j).unwrap();
        assert_eq!(r, Reply { text: "七天。".into(), session_id: Some("abc".into()), model: Some("claude-x".into()) });
    }

    #[test]
    fn an_error_result_becomes_an_error() {
        let j = r#"{"is_error":true,"result":"额度用完了"}"#;
        assert!(parse_reply(j).unwrap_err().to_string().contains("额度用完了"));
    }

    #[test]
    fn an_empty_result_becomes_an_error() {
        assert!(parse_reply(r#"{"is_error":false,"result":"  "}"#).is_err());
    }

    #[test]
    fn garbage_becomes_an_error() {
        assert!(parse_reply("not json").is_err());
    }
}
