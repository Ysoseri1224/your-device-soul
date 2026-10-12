//! 便宜的模型：只跑腿，不下结论。经由 OpenRouter，密钥是机主在 cc-switch 里配好的那一条。
//!
//! 密钥每次用的时候现取，只在内存里，不落盘，不进任何记录。

use anyhow::{bail, Context as _, Result};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::time::Duration;

pub const DEFAULT_MODEL: &str = "anthropic/claude-haiku-4.5";
const ENDPOINT: &str = "https://openrouter.ai/api/v1/chat/completions";

fn cc_switch_db() -> Result<PathBuf> {
    let home = std::env::var_os("USERPROFILE").context("找不到用户目录")?;
    Ok(PathBuf::from(home).join(".cc-switch").join("cc-switch.db"))
}

/// 从 cc-switch 的一条供应商配置里取出密钥。
pub fn extract_key(settings_config: &str) -> Result<String> {
    let v: Value = serde_json::from_str(settings_config).context("cc-switch 里这条配置不是 JSON")?;
    let key = v.pointer("/auth/OPENAI_API_KEY").and_then(Value::as_str).unwrap_or("").trim();
    if key.is_empty() {
        bail!("cc-switch 里 OpenRouter 这一条没有填密钥");
    }
    Ok(key.to_string())
}

fn openrouter_key() -> Result<String> {
    let db = cc_switch_db()?;
    let con = rusqlite::Connection::open_with_flags(&db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("打不开 {}", db.display()))?;
    let cfg: String = con
        .query_row("select settings_config from providers where lower(name) = 'openrouter' limit 1", [], |r| r.get(0))
        .context("cc-switch 里没有名叫 OpenRouter 的供应商")?;
    extract_key(&cfg)
}

/// 从 OpenRouter 的回答里取出文字。
pub fn parse_completion(body: &str) -> Result<String> {
    let v: Value = serde_json::from_str(body).context("OpenRouter 的回答不是 JSON")?;
    if let Some(msg) = v.pointer("/error/message").and_then(Value::as_str) {
        bail!("OpenRouter 报错：{msg}");
    }
    let text = v.pointer("/choices/0/message/content").and_then(Value::as_str).unwrap_or("").trim();
    if text.is_empty() {
        bail!("OpenRouter 没有给出回答");
    }
    Ok(text.to_string())
}

/// 派一件跑腿的活。机主在通道里指定了跑腿用哪条，就用那条（不带工具，问一句答一句）；没指定就照旧走 cc-switch 里的 OpenRouter。
pub fn errand(system: &str, user: &str, model: &str, max_tokens: u32) -> Result<String> {
    if let Some(root) = crate::paths::Root::discover(None).ok() {
        let choice = crate::channels::choice(&root);
        if choice.cheap.is_some() {
            if let Some(ch) = crate::channels::cheap_one(&crate::channels::discover(), &choice) {
                return errand_via(&ch, system, user, max_tokens);
            }
        }
    }
    let key = openrouter_key()?;
    let body = json!({
        "model": model,
        "max_tokens": max_tokens,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user },
        ],
    });
    let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(90)).build();
    let resp = agent
        .post(ENDPOINT)
        .set("Authorization", &format!("Bearer {key}"))
        .set("X-Title", "ysoul")
        .send_json(body);
    let text = match resp {
        Ok(r) => r.into_string()?,
        Err(ureq::Error::Status(code, r)) => {
            let detail = r.into_string().unwrap_or_default();
            return parse_completion(&detail).and_then(|_| bail!("OpenRouter 返回 {code}"));
        }
        Err(e) => bail!("连不上 OpenRouter：{e}"),
    };
    parse_completion(&text)
}

/// 经由一条指定的通道问一句，不带工具。
fn errand_via(ch: &crate::channels::Channel, system: &str, user: &str, max_tokens: u32) -> Result<String> {
    use crate::channels::Wire;
    let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(90)).build();
    let (url, body) = match ch.wire {
        Wire::Anthropic => (format!("{}/v1/messages", ch.base_url), json!({ "model": ch.model, "max_tokens": max_tokens, "system": system, "messages": [{ "role": "user", "content": user }] })),
        _ => (format!("{}/chat/completions", ch.base_url), json!({ "model": ch.model, "max_tokens": max_tokens, "messages": [{ "role": "system", "content": system }, { "role": "user", "content": user }] })),
    };
    let mut req = agent.post(&url).set("Authorization", &format!("Bearer {}", ch.key));
    if ch.wire == Wire::Anthropic {
        req = req.set("x-api-key", &ch.key).set("anthropic-version", "2023-06-01");
    }
    let v: Value = req.send_json(body).map_err(|e| anyhow::anyhow!("{} 没问通：{e}", ch.name))?.into_json()?;
    let text = if ch.wire == Wire::Anthropic { v["content"][0]["text"].as_str() } else { v["choices"][0]["message"]["content"].as_str() }.unwrap_or("").trim().to_string();
    if text.is_empty() {
        bail!("{} 没有给出回答", ch.name);
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_the_key() {
        assert_eq!(extract_key(r#"{"auth":{"OPENAI_API_KEY":" k1 "},"config":""}"#).unwrap(), "k1");
    }

    #[test]
    fn a_missing_key_is_an_error() {
        assert!(extract_key(r#"{"auth":{}}"#).is_err());
        assert!(extract_key("not json").is_err());
    }

    #[test]
    fn parses_a_completion() {
        let b = r#"{"choices":[{"message":{"role":"assistant","content":" 一周有7天。 "}}]}"#;
        assert_eq!(parse_completion(b).unwrap(), "一周有7天。");
    }

    #[test]
    fn an_api_error_is_reported_with_its_message() {
        let b = r#"{"error":{"message":"No auth credentials found","code":401}}"#;
        assert!(parse_completion(b).unwrap_err().to_string().contains("No auth credentials"));
    }

    #[test]
    fn an_empty_completion_is_an_error() {
        assert!(parse_completion(r#"{"choices":[]}"#).is_err());
    }
}
