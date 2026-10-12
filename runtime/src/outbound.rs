//! 出站记录：每次发给模型的内容都在本机留底，机主随时可查。留底前遮掉形似密钥的部分。

use crate::paths::Root;
use anyhow::Result;
use regex::Regex;
use serde_json::json;
use std::io::Write;
use std::sync::OnceLock;

fn patterns() -> &'static Vec<(Regex, &'static str)> {
    static P: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();
    P.get_or_init(|| {
        let rules: [(&str, &str); 8] = [
            (r"\bsk-[A-Za-z0-9_\-]{16,}", "<hidden>"),
            (r"\bgh[pousr]_[A-Za-z0-9]{20,}", "<hidden>"),
            (r"\bAKIA[0-9A-Z]{16}", "<hidden>"),
            (r"eyJ[A-Za-z0-9_\-]{20,}\.[A-Za-z0-9_\-]{10,}\.[A-Za-z0-9_\-]*", "<hidden>"),
            (r"(://)[^/\s:@]+:[^/\s@]+@", "${1}<hidden>@"),
            // 名字后面必须跟着等号或冒号才算。"Password Recovery" 这种只是普通的词。
            (r#"(?i)((?:api[_\-]?key|token|secret|password|passwd|pwd)["']?\s*[=:]\s*["']?)[^\s"'&;,]{6,}"#, "${1}<hidden>"),
            // 个人信息和密钥同样对待：邮箱地址、微信的账号编号
            (r"[A-Za-z0-9._%+\-]+@[A-Za-z0-9\-]+(?:\.[A-Za-z0-9\-]+)+", "<hidden>"),
            (r"\bwxid_[A-Za-z0-9_]+", "<hidden>"),
        ];
        rules.iter().map(|(p, r)| (Regex::new(p).expect("内置的规则应当能编译"), *r)).collect()
    })
}

/// 把一段文字里形似密钥的部分换成 `<hidden>`。
pub fn redact(text: &str) -> String {
    let mut out = text.to_string();
    for (re, rep) in patterns() {
        out = re.replace_all(&out, *rep).into_owned();
    }
    out
}

/// 往当天的出站记录里追加一条。
pub fn log(root: &Root, now: &str, channel: &str, model: &str, text: &str) -> Result<()> {
    let dir = root.local().join("outbound");
    std::fs::create_dir_all(&dir)?;
    let day = now.get(..10).unwrap_or(now);
    let line = json!({
        "at": now,
        "channel": channel,
        "model": model,
        "chars": text.chars().count(),
        "text": redact(text),
    });
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(dir.join(format!("{day}.jsonl")))?;
    writeln!(f, "{line}")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hides_sk_keys() {
        assert_eq!(redact("key sk-abcdefghijklmnopqrstuv end"), "key <hidden> end");
    }

    #[test]
    fn does_not_mistake_task_names_for_keys() {
        let s = r"\OneDrive Reporting Task-S-1-5-21-247337767-276251085";
        assert_eq!(redact(s), s);
    }

    #[test]
    fn hides_url_credentials_but_keeps_the_host() {
        assert_eq!(redact("https://bob:hunter2@example.com/x"), "https://<hidden>@example.com/x");
    }

    #[test]
    fn hides_values_after_token_names() {
        assert_eq!(redact("run --token=abcdef123456 --fast"), "run --token=<hidden> --fast");
    }

    #[test]
    fn a_bare_word_like_password_is_not_a_secret() {
        assert_eq!(redact("Advanced Archive Password Recovery.lnk"), "Advanced Archive Password Recovery.lnk");
    }

    #[test]
    fn hides_email_addresses_and_chat_account_ids() {
        assert_eq!(redact("email = someone@example.com"), "email = <hidden>");
        assert_eq!(redact(r"D:\xwechat_files\wxid_abc123def_1a2b"), r"D:\xwechat_files\<hidden>");
    }

    #[test]
    fn leaves_ordinary_text_alone() {
        assert_eq!(redact("昨晚停在表单校验"), "昨晚停在表单校验");
    }

    #[test]
    fn log_appends_one_json_line_per_call() {
        let t = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(t.path().join("memory")).unwrap();
        std::fs::create_dir_all(t.path().join("seed")).unwrap();
        let root = Root::at(t.path()).unwrap();
        log(&root, "2026-10-11T09:00:00", "strong", "claude", "你好 sk-abcdefghijklmnopqrstuv").unwrap();
        log(&root, "2026-10-11T09:01:00", "cheap", "haiku", "第二条").unwrap();
        let body = std::fs::read_to_string(t.path().join("local/outbound/2026-10-11.jsonl")).unwrap();
        let lines: Vec<&str> = body.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("<hidden>") && !lines[0].contains("sk-abc"));
        let v: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(v["channel"], "cheap");
        assert_eq!(v["chars"], 3);
    }
}
