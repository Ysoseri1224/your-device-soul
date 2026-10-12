//! 模型通道：这台机器上已经装好、配好的模型，它自己去发现，用哪条由机主定。
//!
//! 机主的话："我总有不用 Claude Code 的时候（我也心疼钱）。"所以强模型不写死走哪一家。
//! 通道只分三种说话的格式，别的都是这三种的变体：
//!   · ClaudeCode：调本机的 claude 命令。工具和把关靠它的钩子（见 strong.rs）。
//!   · Anthropic：Messages 接口。凡是给 Claude Code 配的中转、DeepSeek 的 anthropic 入口都是这种。
//!   · OpenAI：chat/completions 接口。OpenRouter、Ollama、各家兼容入口、给 Codex 配的多半是这种。
//! 后两种没有现成的工具循环，由 agent.rs 自己跑，工具调用在进程内过同一道门。
//!
//! 从哪发现：claude 命令在不在；cc-switch 里配过的供应商；环境变量里的密钥；本机的 Ollama。
//! 密钥只在内存里，不落盘，不进任何记录；存下来的只有"用哪一条"的编号。

use crate::paths::Root;
use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Wire {
    ClaudeCode,
    Anthropic,
    OpenAI,
}

#[derive(Debug, Clone, Serialize)]
pub struct Channel {
    /// 稳定的编号，存进配置用
    pub id: String,
    /// 给机主看的名字
    pub name: String,
    pub wire: Wire,
    pub base_url: String,
    pub model: String,
    /// 从哪发现的：claude 命令 / cc-switch / 环境变量 / Ollama
    pub source: String,
    #[serde(skip)]
    pub key: String,
}

fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

/// 模型名里方括号的后缀是 Claude Code 自己的记号（比如 [1m]），发给接口前去掉。
pub fn clean_model(m: &str) -> String {
    m.split('[').next().unwrap_or(m).trim().to_string()
}

/// cc-switch 里给 Claude Code 配的一条：环境变量里有入口、密钥、模型。
pub fn from_claude_provider(name: &str, cfg: &str) -> Option<Channel> {
    let v: Value = serde_json::from_str(cfg).ok()?;
    let env = v.get("env")?;
    let s = |k: &str| env.get(k).and_then(Value::as_str).unwrap_or("").trim().to_string();
    let (base, key) = (s("ANTHROPIC_BASE_URL"), if s("ANTHROPIC_AUTH_TOKEN").is_empty() { s("ANTHROPIC_API_KEY") } else { s("ANTHROPIC_AUTH_TOKEN") });
    let model = clean_model(&s("ANTHROPIC_MODEL"));
    if base.is_empty() || key.is_empty() || model.is_empty() {
        return None;
    }
    Some(Channel { id: format!("ccs-{}-{}", slug(name), slug(&model)), name: format!("{name}（{model}）"), wire: Wire::Anthropic, base_url: base.trim_end_matches('/').to_string(), model, source: "cc-switch".into(), key })
}

fn toml_value(text: &str, key: &str) -> Option<String> {
    let re = regex::Regex::new(&format!(r#"(?m)^\s*{key}\s*=\s*"([^"]*)""#)).ok()?;
    re.captures(text).map(|c| c[1].trim().to_string()).filter(|s| !s.is_empty())
}

/// cc-switch 里给 Codex 配的一条：密钥在 auth 里，入口和模型在那段 TOML 里。
pub fn from_codex_provider(name: &str, cfg: &str) -> Option<Channel> {
    let v: Value = serde_json::from_str(cfg).ok()?;
    let key = v.pointer("/auth/OPENAI_API_KEY").and_then(Value::as_str).unwrap_or("").trim().to_string();
    let toml = v.get("config").and_then(Value::as_str).unwrap_or("");
    let base = toml_value(toml, "base_url").or_else(|| v.pointer("/auth/OPENAI_BASE_URL").and_then(Value::as_str).map(|s| s.trim().to_string()).filter(|s| !s.is_empty()))?;
    let model = clean_model(&toml_value(toml, "model")?);
    if key.is_empty() {
        return None;
    }
    Some(Channel { id: format!("ccs-{}-{}", slug(name), slug(&model)), name: format!("{name}（{model}）"), wire: Wire::OpenAI, base_url: base.trim_end_matches('/').to_string(), model, source: "cc-switch".into(), key })
}

fn cc_switch() -> Vec<Channel> {
    let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) else { return Vec::new() };
    let db = std::path::PathBuf::from(home).join(".cc-switch").join("cc-switch.db");
    let Ok(con) = rusqlite::Connection::open_with_flags(&db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY) else { return Vec::new() };
    let Ok(mut st) = con.prepare("select app_type, name, settings_config from providers") else { return Vec::new() };
    let rows = st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2).unwrap_or_default())));
    let Ok(rows) = rows else { return Vec::new() };
    rows.flatten().filter_map(|(app, name, cfg)| match app.as_str() {
        "claude" => from_claude_provider(&name, &cfg),
        "codex" => from_codex_provider(&name, &cfg),
        _ => None,
    }).collect()
}

fn env_channels() -> Vec<Channel> {
    let get = |k: &str| std::env::var(k).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let mut out = Vec::new();
    let mut add = |id: &str, name: &str, wire: Wire, base: String, model: &str, key: Option<String>| {
        if let Some(key) = key {
            out.push(Channel { id: id.into(), name: name.into(), wire, base_url: base.trim_end_matches('/').to_string(), model: get("YSOUL_MODEL").unwrap_or_else(|| model.to_string()), source: "环境变量".into(), key });
        }
    };
    add("env-anthropic", "环境变量里的 Anthropic", Wire::Anthropic, get("ANTHROPIC_BASE_URL").unwrap_or_else(|| "https://api.anthropic.com".into()), "claude-sonnet-5-5", get("ANTHROPIC_API_KEY"));
    add("env-openai", "环境变量里的 OpenAI", Wire::OpenAI, get("OPENAI_BASE_URL").unwrap_or_else(|| "https://api.openai.com/v1".into()), "gpt-5", get("OPENAI_API_KEY"));
    add("env-openrouter", "环境变量里的 OpenRouter", Wire::OpenAI, "https://openrouter.ai/api/v1".into(), "anthropic/claude-sonnet-5.5", get("OPENROUTER_API_KEY"));
    add("env-deepseek", "环境变量里的 DeepSeek", Wire::OpenAI, "https://api.deepseek.com/v1".into(), "deepseek-chat", get("DEEPSEEK_API_KEY"));
    out
}

/// 本机的 Ollama：问它装了哪些模型。没开着就是空的，半秒内放弃。
fn ollama() -> Vec<Channel> {
    let agent = ureq::AgentBuilder::new().timeout(Duration::from_millis(500)).build();
    let Ok(resp) = agent.get("http://127.0.0.1:11434/api/tags").call() else { return Vec::new() };
    let Ok(v) = resp.into_json::<Value>() else { return Vec::new() };
    v["models"].as_array().map(|ms| ms.iter().filter_map(|m| m["name"].as_str()).map(|n| Channel { id: format!("ollama-{}", slug(n)), name: format!("本机 Ollama（{n}）"), wire: Wire::OpenAI, base_url: "http://127.0.0.1:11434/v1".into(), model: n.to_string(), source: "Ollama".into(), key: String::new() }).collect()).unwrap_or_default()
}

fn has_claude_cli() -> bool {
    let mut cmd = std::process::Command::new("cmd");
    crate::quiet::quiet(&mut cmd);
    cmd.args(["/D", "/C", "where", "claude"]).output().map(|o| o.status.success()).unwrap_or(false)
}

/// 这台机器上现在能用的通道。同一个编号只留第一条。
pub fn discover() -> Vec<Channel> {
    let mut all = Vec::new();
    if has_claude_cli() {
        all.push(Channel { id: "claude-code".into(), name: "Claude Code（用它登录的那个账号）".into(), wire: Wire::ClaudeCode, base_url: String::new(), model: String::new(), source: "claude 命令".into(), key: String::new() });
    }
    all.extend(cc_switch());
    all.extend(env_channels());
    all.extend(ollama());
    let mut seen = std::collections::HashSet::new();
    all.retain(|c| seen.insert(c.id.clone()));
    all
}

/// 机主的选择：强模型按这个次序试，前一条不通就用后一条；跑腿的用哪一条。存的只有编号。
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Choice {
    #[serde(default)]
    pub strong: Vec<String>,
    #[serde(default)]
    pub cheap: Option<String>,
}

fn choice_file(root: &Root) -> std::path::PathBuf {
    root.local().join("channels.json")
}

pub fn choice(root: &Root) -> Choice {
    std::fs::read_to_string(choice_file(root)).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

pub fn save_choice(root: &Root, c: &Choice) -> Result<()> {
    std::fs::create_dir_all(root.local())?;
    std::fs::write(choice_file(root), serde_json::to_string_pretty(c)?).context("存不了通道的选择")
}

/// 强模型该按什么次序试。机主选过的排前面（还在的才算）；没选过就是：有 Claude Code 先用它，否则用发现的第一条。
pub fn strong_order(all: &[Channel], choice: &Choice) -> Vec<Channel> {
    let mut out: Vec<Channel> = choice.strong.iter().filter_map(|id| all.iter().find(|c| &c.id == id).cloned()).collect();
    if out.is_empty() {
        out.extend(all.iter().find(|c| c.wire == Wire::ClaudeCode).or_else(|| all.first()).cloned());
    }
    out
}

/// 跑腿用哪一条：机主选的；没选就找一条不是 Claude Code 的。
pub fn cheap_one(all: &[Channel], choice: &Choice) -> Option<Channel> {
    choice.cheap.as_ref().and_then(|id| all.iter().find(|c| &c.id == id)).or_else(|| all.iter().find(|c| c.wire != Wire::ClaudeCode)).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(id: &str, wire: Wire) -> Channel {
        Channel { id: id.into(), name: id.into(), wire, base_url: "http://x".into(), model: "m".into(), source: "测试".into(), key: "k".into() }
    }

    #[test]
    fn a_claude_code_provider_becomes_an_anthropic_channel() {
        let c = from_claude_provider("DeepSeek", r#"{"env":{"ANTHROPIC_BASE_URL":"https://api.deepseek.com/anthropic/","ANTHROPIC_AUTH_TOKEN":"sk-1","ANTHROPIC_MODEL":"deepseek-v4-pro[1m]"}}"#).unwrap();
        assert_eq!((c.wire, c.base_url.as_str(), c.model.as_str(), c.key.as_str()), (Wire::Anthropic, "https://api.deepseek.com/anthropic", "deepseek-v4-pro", "sk-1"));
        assert_eq!(c.id, "ccs-deepseek-deepseek-v4-pro");
        assert!(from_claude_provider("官方", r#"{"skipDangerousModePermissionPrompt":true}"#).is_none(), "没有入口和密钥的不算一条通道");
    }

    #[test]
    fn a_codex_provider_becomes_an_openai_channel() {
        let cfg = r#"{"auth":{"OPENAI_API_KEY":"sk-2"},"config":"model_provider = \"or\"\nmodel = \"openai/gpt-5\"\n[model_providers.or]\nname = \"OpenRouter\"\nbase_url = \"https://openrouter.ai/api/v1\"\n"}"#;
        let c = from_codex_provider("OpenRouter", cfg).unwrap();
        assert_eq!((c.wire, c.base_url.as_str(), c.model.as_str()), (Wire::OpenAI, "https://openrouter.ai/api/v1", "openai/gpt-5"));
        assert!(from_codex_provider("空的", r#"{"config":""}"#).is_none());
    }

    #[test]
    fn the_key_is_never_serialised() {
        let j = serde_json::to_string(&ch("a", Wire::OpenAI)).unwrap();
        assert!(!j.contains("\"k\"") && !j.contains("key"), "{j}");
    }

    #[test]
    fn the_owner_s_order_wins_and_missing_ones_are_skipped() {
        let all = vec![ch("claude-code", Wire::ClaudeCode), ch("a", Wire::Anthropic), ch("b", Wire::OpenAI)];
        let picked = strong_order(&all, &Choice { strong: vec!["gone".into(), "b".into(), "a".into()], cheap: None });
        assert_eq!(picked.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(), ["b", "a"]);
        assert_eq!(strong_order(&all, &Choice::default())[0].id, "claude-code", "没选过就先用 Claude Code");
        assert_eq!(strong_order(&all[1..], &Choice::default())[0].id, "a", "没有 Claude Code 就用发现的第一条");
        assert_eq!(cheap_one(&all, &Choice::default()).unwrap().id, "a", "跑腿的不用 Claude Code");
        assert_eq!(cheap_one(&all, &Choice { strong: vec![], cheap: Some("b".into()) }).unwrap().id, "b");
    }
}
