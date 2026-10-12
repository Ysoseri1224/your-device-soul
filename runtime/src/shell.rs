//! 命令行：它能跑命令了，那道门就得会看命令。
//!
//! 做法是只放行程序能**完全看懂**的命令，其余的一律转成票给机主过目：
//!
//! 1. 用 PowerShell 自己的语法分析器把命令拆开（`judge.ps1`，不执行）。只有"一条管道、第一段的参数全是写死的值、
//!    后面几段只做筛选和排版、哪里都没有变量和套着的命令"才算看得懂。
//! 2. 第一段必须是一条已知只读的命令；它碰到的每个路径都去对家的地图。
//! 3. 都过了才直接跑，而且是在受限的语言模式下跑；输出先遮密再交给模型。
//!
//! 为什么不去"分析任意脚本安不安全"：路径可以拼出来、可以从文件里读出来、可以顺着管道传下去，静态分析追不完。
//! 所以不追：凡是有这些写法的，程序就说"我看不懂"，交给机主看。看不懂不等于坏，只是不由程序做主。

use crate::paths::Root;
use crate::places::{self, Act, Place};
use crate::{hands, outbound};
use anyhow::{bail, Context as _, Result};
use serde::Deserialize;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const JUDGE: &str = include_str!("judge.ps1");

#[derive(Deserialize, Debug, Clone, Default)]
pub struct Arg {
    pub param: Option<String>,
    pub value: Option<String>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct Source {
    pub name: String,
    #[serde(default)]
    pub args: Vec<Arg>,
    #[serde(default)]
    pub words: Vec<String>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct Filter {
    pub name: String,
    #[serde(default)]
    pub params: Vec<String>,
    #[serde(default)]
    pub positionals: usize,
}

/// 语法分析器看完之后说的话。
#[derive(Deserialize, Debug, Clone, Default)]
pub struct Parsed {
    pub simple: bool,
    #[serde(default)]
    pub why: String,
    #[serde(default)]
    pub source: Option<Source>,
    #[serde(default)]
    pub filters: Vec<Filter>,
}

/// 对一条命令的判断。
#[derive(Debug, PartialEq)]
pub enum Verdict {
    /// 只读，碰到的地方都能看：直接跑
    Run,
    /// 程序看不懂，或者不是只读的：交给机主过目。附原因
    Ask(String),
    /// 明确要碰不能看的地方：不跑，也不递票。附原因
    Refuse(String),
}

fn pwsh() -> Command {
    let mut cmd = Command::new("pwsh");
    crate::quiet::quiet(&mut cmd);
    cmd
}

/// 让语法分析器看一条命令。
pub fn parse(root: &Root, command: &str) -> Result<Parsed> {
    let tmp = root.local().join("tmp");
    std::fs::create_dir_all(&tmp)?;
    let script = tmp.join("judge.ps1");
    std::fs::write(&script, format!("\u{feff}{JUDGE}"))?;
    let mut child = pwsh().args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File"]).arg(&script).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().context("启动不了 PowerShell 7")?;
    child.stdin.take().context("拿不到标准输入")?.write_all(command.as_bytes())?;
    let out = child.wait_with_output()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().rev().find(|l| l.trim_start().starts_with('{')).unwrap_or("");
    serde_json::from_str(line).with_context(|| format!("看不懂语法分析器的回答：{}", String::from_utf8_lossy(&out.stderr).trim()))
}

/// 管道后面几段只许是这些：筛选、排序、计数、排版。它们不碰文件。
const FILTERS: &[&str] = &["where-object", "select-object", "sort-object", "measure-object", "group-object", "format-table", "format-list", "format-wide", "out-string", "convertto-json", "convertto-csv", "select-string"];
/// 第一段：读文件系统（或注册表）的。它们的参数要对地图。
const FS: &[&str] = &["get-childitem", "get-content", "get-item", "get-itemproperty", "test-path", "get-filehash", "resolve-path", "select-string", "get-authenticodesignature"];
/// 第一段：读系统状态的。不带路径。
const PLAIN: &[&str] = &["get-process", "get-service", "get-scheduledtask", "get-volume", "get-psdrive", "get-date", "get-command", "get-ciminstance", "get-winevent", "get-nettcpconnection", "get-netadapter", "get-netipaddress", "get-hotfix", "get-startapps", "get-appxpackage", "get-computerinfo", "get-disk", "get-partition", "get-uptime"];
/// 这些参数后面跟的不是路径。别的参数后面的值一律当成路径去对地图：多查不要紧，漏查不行。
const NOT_PATHS: &[&str] = &["filter", "include", "exclude", "pattern", "encoding", "totalcount", "tail", "first", "last", "head", "depth", "algorithm", "name", "stream", "delimiter", "readcount", "context", "erroraction", "ea", "attributes", "property"];
/// 这些参数一出现就不算只读，或者会让命令去别的机器、别的地方读。
const BAD_PARAMS: &[&str] = &["computername", "cimsession", "credential", "filepath", "outfile", "outvariable", "ov", "pipelinevariable", "pv", "inputobject", "followsymlink", "usetransaction", "asjob", "session"];

/// 这个参数是不是不放行的。PowerShell 允许参数名只写开头几个字母（-Comp 就是 -ComputerName），所以按开头来认。
/// 认错了只是多转一张票，不要紧。
fn bad_param(p: &str, extra: &[&str]) -> bool {
    !p.is_empty() && BAD_PARAMS.iter().chain(extra.iter()).any(|b| b.starts_with(p))
}

fn is_dir_listing(name: &str) -> bool {
    matches!(name, "get-childitem" | "select-string" | "get-content" | "get-item" | "get-filehash" | "resolve-path" | "test-path" | "get-itemproperty")
}

/// 一个参数值指向哪里。带通配符的取通配符之前那一截目录。
fn resolve(cwd: &Path, value: &str) -> PathBuf {
    let literal: String = value.chars().take_while(|c| !"*?[".contains(*c)).collect();
    let literal = if literal.len() < value.len() { literal.rfind(['/', '\\']).map(|i| literal[..=i].to_string()).unwrap_or_default() } else { literal };
    let p = Path::new(&literal);
    if literal.is_empty() {
        cwd.to_path_buf()
    } else if p.is_absolute() {
        p.to_path_buf()
    } else {
        cwd.join(p)
    }
}

fn see(root: &Root, map: &[Place], path: &Path) -> Option<String> {
    let (ok, why) = places::allowed(map, root, path, Act::See);
    if !ok {
        return Some(why);
    }
    if path.is_dir() && places::hides_something(map, path) {
        return Some(format!("{} 下面有不能看的地方，不能整个列或整个搜。缩小到具体的子目录。", path.display()));
    }
    None
}

fn check_fs(root: &Root, map: &[Place], cwd: &Path, src: &Source) -> Verdict {
    let mut positional = 0;
    let has_pattern = src.args.iter().any(|a| a.param.as_deref() == Some("pattern"));
    let mut paths = 0;
    for a in &src.args {
        let p = a.param.as_deref();
        if let Some(p) = p {
            if bad_param(p, &[]) {
                return Verdict::Ask(format!("用了 -{p}，程序不放行这个参数"));
            }
        }
        let Some(v) = &a.value else { continue };
        if p.is_none() {
            positional += 1;
            // Select-String 的第一个位置参数是要找的字样，不是路径
            if src.name == "select-string" && positional == 1 && !has_pattern {
                continue;
            }
        }
        if p.is_some_and(|p| NOT_PATHS.contains(&p)) {
            continue;
        }
        if v.starts_with('~') {
            return Verdict::Refuse("用户目录在私人名单里，不能看。".into());
        }
        // 注册表可以读；别的"盘"（环境变量、证书、变量、函数）不行：里面可能有密钥，也不归地图管
        let lower = v.to_lowercase();
        if let Some((drive, _)) = lower.split_once(':') {
            if drive.len() > 1 {
                if matches!(drive, "hklm" | "hkcu") && src.name != "get-content" && src.name != "select-string" {
                    paths += 1;
                    continue;
                }
                return Verdict::Refuse(format!("{drive}: 这种地方不给看。能看的只有盘上的路径，和注册表的 HKLM:、HKCU:。"));
            }
        }
        paths += 1;
        if let Some(why) = see(root, map, &resolve(cwd, v)) {
            return Verdict::Refuse(why);
        }
    }
    // 没写路径：看的就是它所在的目录
    if paths == 0 && is_dir_listing(&src.name) {
        if let Some(why) = see(root, map, cwd) {
            return Verdict::Refuse(why);
        }
    }
    Verdict::Run
}

fn check_plain(src: &Source) -> Verdict {
    for a in &src.args {
        if let Some(p) = a.param.as_deref() {
            if bad_param(p, &["path", "literalpath"]) {
                return Verdict::Ask(format!("用了 -{p}，程序不放行这个参数"));
            }
        }
    }
    Verdict::Run
}

/// git 只放行几样只读的。历史只给看提交说明：不放行会把旧内容打出来的写法（见 00，旧提交里有机主的个人信息）。
fn check_git(root: &Root, map: &[Place], cwd: &Path, src: &Source) -> Verdict {
    let mut words = src.words.iter().map(|s| s.as_str()).peekable();
    let mut repo = cwd.to_path_buf();
    while words.peek() == Some(&"-C") {
        words.next();
        match words.next() {
            Some(dir) => repo = resolve(cwd, dir),
            None => return Verdict::Ask("git -C 后面没有目录".into()),
        }
    }
    if let Some(why) = see(root, map, &repo) {
        return Verdict::Refuse(why);
    }
    let Some(sub) = words.next() else { return Verdict::Ask("git 后面没有写要做什么".into()) };
    let rest: Vec<&str> = words.collect();
    let flags_ok = |allowed: &[&str], prefixes: &[&str]| rest.iter().take_while(|w| **w != "--").filter(|w| w.starts_with('-')).all(|w| allowed.contains(w) || prefixes.iter().any(|p| w.starts_with(p)) || (w.len() > 1 && w[1..].chars().all(|c| c.is_ascii_digit())));
    let no_positionals = rest.iter().take_while(|w| **w != "--").all(|w| w.starts_with('-'));
    let ok = match sub {
        "status" => flags_ok(&["-s", "--short", "-b", "--branch", "--porcelain", "-uno", "--untracked-files=no"], &[]) && no_positionals,
        "branch" => flags_ok(&["-a", "-r", "-v", "-vv", "--list", "--show-current", "--all"], &[]) && no_positionals,
        "remote" => rest.is_empty() || rest == ["-v"],
        "rev-parse" => rest.iter().all(|w| !w.starts_with("--git-path") && !w.contains(':')),
        "ls-files" => flags_ok(&["-m", "-o", "-d", "-s", "--modified", "--others", "--deleted", "--exclude-standard", "--cached"], &[]),
        // 提交说明可以看，改了什么内容不行：不放行 -p / --patch / -S / -G
        "log" => flags_ok(&["--oneline", "--graph", "--decorate", "--stat", "--shortstat", "--name-only", "--name-status", "--no-merges", "--all", "-n", "--reverse"], &["--since=", "--until=", "--after=", "--before=", "--max-count=", "--format=", "--pretty=", "--date=", "--author=", "--grep="]),
        // 只看工作区里还没提交的改动，不拿旧版本来比
        "diff" => flags_ok(&["--stat", "--shortstat", "--cached", "--staged", "--name-only", "--name-status"], &[]) && no_positionals,
        _ => false,
    };
    if !ok {
        return Verdict::Ask(format!("git {sub} 的这种用法不在放行的只读用法里"));
    }
    // "--" 后面的是路径
    if let Some(i) = rest.iter().position(|w| *w == "--") {
        for p in &rest[i + 1..] {
            if let Some(why) = see(root, map, &resolve(&repo, p)) {
                return Verdict::Refuse(why);
            }
        }
    }
    Verdict::Run
}

/// 是不是在问一个程序"你怎么用""你是什么版本"。只认普查记下的那些命令行程序（body/handles.tsv 里的 cli 和 agent），
/// 而且参数只能是求助或问版本的那几种写法，最多带一个子命令（git log --help 这种）。
fn asks_for_help(root: &Root, src: &Source) -> bool {
    let known = std::fs::read_to_string(root.memory().join("body").join("handles.tsv")).unwrap_or_default();
    let name = src.name.trim_end_matches(".exe");
    let listed = known.lines().any(|l| {
        let mut f = l.split('\t');
        matches!(f.next(), Some("cli") | Some("agent")) && f.next() == Some(name)
    });
    let helps = ["--help", "-h", "-?", "/?", "help", "--version", "-V", "version"];
    let w: Vec<&str> = src.words.iter().map(|s| s.as_str()).collect();
    listed && match w.as_slice() {
        [one] => helps.contains(one),
        [sub, one] | [one, sub] if helps.contains(one) => sub.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') && !sub.starts_with('-'),
        _ => false,
    }
}

/// 对拆开后的命令下判断。
pub fn decide(root: &Root, map: &[Place], cwd: &Path, parsed: &Parsed, raw: &str) -> Verdict {
    if !parsed.simple {
        return Verdict::Ask(parsed.why.clone());
    }
    let Some(src) = &parsed.source else { return Verdict::Ask("没看出要运行什么".into()) };
    if let Some(why) = see(root, map, cwd).filter(|_| !places::allowed(map, root, cwd, Act::See).0) {
        return Verdict::Refuse(why);
    }
    for f in &parsed.filters {
        if !FILTERS.contains(&f.name.as_str()) {
            return Verdict::Ask(format!("管道里的 {} 不在只做筛选和排版的那几样里", f.name));
        }
        if f.params.iter().any(|p| p == "lp" || bad_param(p, &["path", "literalpath"])) || (f.name == "select-string" && f.positionals > 1) {
            return Verdict::Ask(format!("管道里的 {} 带了会去读别处的参数", f.name));
        }
    }
    let name = src.name.as_str();
    // 进程的窗口标题里有他在和谁聊天。默认的表格不带标题；会把所有属性都打出来的写法不放行
    if name == "get-process" {
        let low = raw.to_lowercase();
        if low.contains("windowtitle") || low.contains('*') || parsed.filters.iter().any(|f| matches!(f.name.as_str(), "format-list" | "convertto-json" | "convertto-csv")) {
            return Verdict::Ask("这样会把各个窗口的标题打出来，里面有聊天对象的名字".into());
        }
    }
    if FS.contains(&name) {
        check_fs(root, map, cwd, src)
    } else if PLAIN.contains(&name) {
        check_plain(src)
    } else if name == "git" {
        check_git(root, map, cwd, src)
    } else if asks_for_help(root, src) {
        // 问一个已知的命令行程序"你怎么用"：这是它自己学用法的办法，不改任何东西
        Verdict::Run
    } else {
        Verdict::Ask(format!("{name} 不在已知只读的命令里"))
    }
}

/// 一条命令的下场。
#[derive(Debug)]
pub enum Outcome {
    /// 跑了。成没成，和遮过密的输出
    Ran(bool, String),
    /// 转成了票，等机主点头
    Ticket(hands::Ticket, String),
    /// 没跑，也没递票
    Refused(String),
}

fn log(root: &Root, now: &str, command: &str, cwd: &Path, verdict: &str) {
    let line = serde_json::json!({ "at": now, "command": command, "cwd": cwd, "verdict": verdict });
    let _ = std::fs::create_dir_all(root.local());
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(root.local().join("runs.jsonl")) {
        let _ = writeln!(f, "{line}");
    }
}

/// 直接跑一条已经判为只读的命令。受限的语言模式，最多一分钟，输出遮密后截到一万字。
fn execute(root: &Root, command: &str, cwd: &Path) -> Result<(bool, String)> {
    let tmp = root.local().join("tmp");
    std::fs::create_dir_all(&tmp)?;
    let script = tmp.join(format!("run-{}.ps1", std::process::id()));
    std::fs::write(&script, format!("\u{feff}[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)\n$ProgressPreference = 'SilentlyContinue'\n$ExecutionContext.SessionState.LanguageMode = 'ConstrainedLanguage'\n{command} | Out-String -Width 220\n"))?;
    let mut child = pwsh().args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File"]).arg(&script).current_dir(cwd).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().context("启动不了 PowerShell 7")?;
    let started = std::time::Instant::now();
    // 另起两个线程把输出读走，免得输出太多把管子堵住
    let (o, e) = (child.stdout.take().context("拿不到输出")?, child.stderr.take().context("拿不到输出")?);
    let ho = std::thread::spawn(move || { let mut s = Vec::new(); let _ = std::io::Read::read_to_end(&mut { o }, &mut s); s });
    let he = std::thread::spawn(move || { let mut s = Vec::new(); let _ = std::io::Read::read_to_end(&mut { e }, &mut s); s });
    let status = loop {
        if let Some(s) = child.try_wait()? {
            break Some(s);
        }
        if started.elapsed().as_secs() >= 60 {
            let _ = child.kill();
            break None;
        }
        std::thread::sleep(std::time::Duration::from_millis(40));
    };
    let _ = std::fs::remove_file(&script);
    let mut text = format!("{}{}", String::from_utf8_lossy(&ho.join().unwrap_or_default()), String::from_utf8_lossy(&he.join().unwrap_or_default()));
    let text_trim = text.trim().to_string();
    text = outbound::redact(&text_trim);
    if text.chars().count() > 10_000 {
        text = format!("{}\n……（输出太长，只给了前一万字。加上筛选再跑一次。）", text.chars().take(10_000).collect::<String>());
    }
    match status {
        Some(s) => Ok((s.success(), text)),
        None => Ok((false, format!("{text}\n（跑了一分钟还没完，停掉了。）"))),
    }
}

/// 它要跑一条命令。只读又看得懂的直接跑；其余的转成票；明确要碰不能看的地方的，不跑。
pub fn run(root: &Root, now: &str, command: &str, cwd: Option<&str>, why: &str, undo: &str) -> Result<Outcome> {
    let command = command.trim();
    if command.is_empty() {
        bail!("没有写命令");
    }
    let cwd = cwd.filter(|c| !c.trim().is_empty()).map(PathBuf::from).unwrap_or_else(|| root.memory());
    if !cwd.is_dir() {
        bail!("没有这个目录：{}", cwd.display());
    }
    let parsed = parse(root, command)?;
    match decide(root, &places::load(root), &cwd, &parsed, command) {
        Verdict::Run => {
            log(root, now, command, &cwd, "run");
            let (ok, out) = execute(root, command, &cwd)?;
            Ok(Outcome::Ran(ok, out))
        }
        Verdict::Refuse(reason) => {
            log(root, now, command, &cwd, "refuse");
            Ok(Outcome::Refused(reason))
        }
        Verdict::Ask(reason) => {
            log(root, now, command, &cwd, "ticket");
            let title = if why.trim().is_empty() { format!("运行一条命令：{}", command.chars().take(40).collect::<String>()) } else { why.trim().to_string() };
            let script = format!("Set-Location -LiteralPath '{}'\n{command}", cwd.display().to_string().replace('\'', "''"));
            let t = hands::issue(root, now, &title, &cwd.display().to_string(), &script, undo, false)?;
            Ok(Outcome::Ticket(t, reason))
        }
    }
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
    fn map() -> Vec<Place> {
        places::parse("| 地方 | 是什么 | 看 | 放 | 改 | 扔 | 运行 | 依据 |\n|---|---|---|---|---|---|---|---|\n| `D:\\personal` | 私人的 | – | – | – | – | – | 你说的 |\n| `D:\\proj` | 项目 | ✓ | ✓ | ✓ | – | – | 你说的 |\n")
    }
    fn verdict(r: &Root, cmd: &str) -> Verdict {
        decide(r, &map(), &r.memory(), &parse(r, cmd).unwrap(), cmd)
    }

    #[test]
    fn plain_reads_run_directly() {
        let (_d, r) = root();
        for cmd in [
            "Get-ChildItem D:\\proj -Recurse | Where-Object { $_.Length -gt 1000 } | Select-Object -First 5 Name, Length",
            "gc D:\\proj\\a.txt -Tail 20",
            "Get-Process | Sort-Object WS -Descending | Select-Object -First 8 ProcessName, WS",
            "Get-Content D:\\proj\\a.log | Select-String 'error'",
            "Select-String -Path D:\\proj\\*.rs -Pattern 'todo'",
            "git -C D:\\proj status --short",
            "git -C D:\\proj log --oneline -5",
            "Get-ItemProperty 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Run'",
            "Get-Service | Where-Object { $_.Status -eq 'Running' -and $_.Name -like 'W*' }",
            "ls",
        ] {
            assert_eq!(verdict(&r, cmd), Verdict::Run, "{cmd}");
        }
    }

    #[test]
    fn anything_that_writes_or_that_the_program_cannot_follow_goes_to_the_owner() {
        let (_d, r) = root();
        for cmd in [
            "Remove-Item D:\\proj\\a.txt",
            "Set-Content D:\\proj\\a.txt hi",
            "gci D:\\proj > out.txt",
            "gci D:\\proj; Remove-Item D:\\proj\\a.txt",
            "$p = 'D:\\per' + 'sonal'; gci $p",
            "gc ('D:\\per' + 'sonal\\x.txt')",
            "gc (gc D:\\proj\\where.txt)",
            "gc D:\\proj\\list.txt | ForEach-Object { gc $_ }",
            "gc D:\\proj\\list.txt | Get-Content",
            "gci D:\\proj | % { $_.Delete() }",
            "[IO.File]::ReadAllText('D:\\personal\\x')",
            "gc \"$env:USERPROFILE\\x\"",
            "& 'gci' D:\\proj",
            "iex 'gci'",
            "gci D:\\proj | Out-File x.txt",
            "git -C D:\\proj log -p",
            "git -C D:\\proj show HEAD~3",
            "git -C D:\\proj diff HEAD~3",
            "git -C D:\\proj branch -D main",
            "git -C D:\\proj push",
            "Get-WinEvent -Path D:\\proj\\x.evtx",
            "Get-Process -ComputerName other",
            "gc D:\\proj\\a.txt | Select-String x D:\\personal\\y.txt",
            "Start-Process notepad",
            "Get-Process -Comp other",
            "Get-Process | Select-Object ProcessName, MainWindowTitle",
            "Get-Process | Format-List",
            "Get-Process | Select-Object *",
            "gci D:\\proj -OutV x",
        ] {
            assert!(matches!(verdict(&r, cmd), Verdict::Ask(_)), "{cmd} → {:?}", verdict(&r, cmd));
        }
    }

    #[test]
    fn reads_that_touch_what_it_may_not_see_are_refused_outright() {
        let (_d, r) = root();
        for cmd in [
            "gc D:\\personal\\x.txt",
            "gci D:\\ -Recurse",
            "gci D:\\proj\\..\\personal",
            "Select-String -Path D:\\personal\\*.md -Pattern x",
            "gc ~\\x.txt",
            "gci Env:",
            "gc Env:\\PATH",
            "git -C D:\\personal status",
            "git -C D:\\proj log --oneline -- ..\\personal\\x",
            "gc \\\\localhost\\D$\\personal\\x.txt",
            "Get-FileHash -LiteralPath D:\\personal\\x.bin",
        ] {
            assert!(matches!(verdict(&r, cmd), Verdict::Refuse(_)), "{cmd} → {:?}", verdict(&r, cmd));
        }
    }

    #[test]
    fn asking_a_known_program_how_it_works_runs_directly_but_only_that() {
        let (_d, r) = root();
        std::fs::create_dir_all(r.memory().join("body")).unwrap();
        std::fs::write(r.memory().join("body").join("handles.tsv"), "kind	name	how	owner
cli	ffmpeg	ffmpeg …	ffmpeg.exe
agent	codex	codex exec	codex.cmd
scheme	vscode:	x	Code.exe
").unwrap();
        for cmd in ["ffmpeg --help", "ffmpeg -h", "codex --version", "codex exec --help", "ffmpeg help"] {
            assert_eq!(verdict(&r, cmd), Verdict::Run, "{cmd}");
        }
        for cmd in ["ffmpeg -i a.mp4 b.mp3", "ffmpeg --help; del x", "unknowntool --help", "codex exec 'rm -rf'", "ffmpeg --help extra more", "vscode --help"] {
            assert!(matches!(verdict(&r, cmd), Verdict::Ask(_)), "{cmd} → {:?}", verdict(&r, cmd));
        }
    }

    #[test]
    fn a_read_only_command_really_runs_and_its_output_is_masked() {
        let (d, r) = root();
        let dir = d.path().join("memory");
        std::fs::write(dir.join("a.txt"), "第一行\napi_key = sk-abcdefghijklmnopqrstuvwxyz123456\n").unwrap();
        match run(&r, "now", "Get-Content a.txt", None, "", "").unwrap() {
            Outcome::Ran(ok, out) => {
                assert!(ok, "{out}");
                assert!(out.contains("第一行"), "{out}");
                assert!(!out.contains("sk-abcdefghijklmnopqrstuvwxyz123456"), "密钥应该被遮掉：{out}");
            }
            other => panic!("应该直接跑：{other:?}"),
        }
    }

    #[test]
    fn a_writing_command_becomes_a_ticket_and_does_not_run() {
        let (d, r) = root();
        let target = d.path().join("memory").join("made.txt");
        match run(&r, "now", &format!("Set-Content -LiteralPath '{}' -Value x", target.display()), None, "建一个文件", "").unwrap() {
            Outcome::Ticket(t, _) => {
                assert_eq!(t.status, "waiting");
                assert_eq!(t.title, "建一个文件");
            }
            other => panic!("应该转成票：{other:?}"),
        }
        assert!(!target.exists());
    }
}
