//! 手：它在这台机器上动手的办法。
//!
//! 把东西拿给机主看（打开、定位），没有后果，直接做。
//! 会改变机器的事，它不直接做，而是递一张票：做什么、在哪、怎么撤回，附上要运行的脚本。
//! 机主点了"做"，票才执行；做完，结果和撤回的办法都留在票上，并记进动作记录。
//! 需要管理员权限的票，执行时系统会走一次提权，等于机主亲手运行（见 02）。

use crate::paths::Root;
use anyhow::{bail, Context as _, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Ticket {
    pub id: String,
    /// 递出的时间
    pub at: String,
    /// 做什么，一句话
    pub title: String,
    /// 在哪
    pub place: String,
    /// 要运行的 PowerShell 脚本
    pub script: String,
    /// 撤回用的脚本。空的表示撤不回
    pub undo: String,
    pub elevated: bool,
    /// waiting 等机主点头 / done 做完了 / failed 做了但失败 / refused 机主说不做 / undone 已撤回
    pub status: String,
    /// 运行时的输出
    #[serde(default)]
    pub output: String,
    /// 这是一张改地图的票时，这里是要添的那个地方；这种票不运行脚本，点头后由程序把这一行写进地图
    #[serde(default)]
    pub map: Option<crate::places::Place>,
    /// 这是一张界面操作的票时，这里是要点的那一下
    #[serde(default)]
    pub ui: Option<crate::ui::Act>,
}

fn dir(root: &Root) -> PathBuf {
    root.local().join("tickets")
}

fn save(root: &Root, t: &Ticket) -> Result<()> {
    std::fs::create_dir_all(dir(root))?;
    std::fs::write(dir(root).join(format!("{}.json", t.id)), serde_json::to_string_pretty(t)?)?;
    Ok(())
}

pub fn get(root: &Root, id: &str) -> Result<Ticket> {
    if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        bail!("票的编号不对");
    }
    let text = std::fs::read_to_string(dir(root).join(format!("{id}.json"))).with_context(|| format!("没有编号为 {id} 的票"))?;
    Ok(serde_json::from_str(&text)?)
}

/// 所有的票，新的在前。
pub fn list(root: &Root) -> Vec<Ticket> {
    let mut out: Vec<Ticket> = std::fs::read_dir(dir(root))
        .map(|d| d.filter_map(|e| e.ok()).filter(|e| e.path().extension().is_some_and(|x| x == "json")).filter_map(|e| std::fs::read_to_string(e.path()).ok()).filter_map(|s| serde_json::from_str(&s).ok()).collect())
        .unwrap_or_default();
    out.sort_by(|a, b| b.id.cmp(&a.id));
    out
}

/// 递一张票。只是写下来等机主点头，不执行。
pub fn issue(root: &Root, now: &str, title: &str, place: &str, script: &str, undo: &str, elevated: bool) -> Result<Ticket> {
    if title.trim().is_empty() || script.trim().is_empty() {
        bail!("票上要写清做什么，并附上要运行的脚本");
    }
    let stamp: String = now.chars().filter(|c| c.is_ascii_digit()).collect();
    let mut id = format!("t{stamp}");
    let mut n = 1;
    while dir(root).join(format!("{id}.json")).exists() {
        n += 1;
        id = format!("t{stamp}-{n}");
    }
    let t = Ticket { id, at: now.to_string(), title: title.trim().into(), place: place.trim().into(), script: script.into(), undo: undo.into(), elevated, status: "waiting".into(), output: String::new(), map: None, ui: None };
    save(root, &t)?;
    Ok(t)
}

/// 递一张界面操作的票：在别的程序里点一下、填一格。
pub fn issue_ui(root: &Root, now: &str, title: &str, act: crate::ui::Act) -> Result<Ticket> {
    let mut t = issue(root, now, title, &crate::pulse::friendly(&act.app), &format!("（界面操作）{title}"), "", false)?;
    t.ui = Some(act);
    save(root, &t)?;
    Ok(t)
}

/// 记下一件已经做了的事（机主放开过的程序里的界面操作）。留成一张做完的票，桌面上有爪印可查。
pub fn record(root: &Root, now: &str, title: &str, place: &str, output: &str) -> Result<Ticket> {
    let mut t = issue(root, now, title, place, &format!("（界面操作）{title}"), "", false)?;
    t.status = "done".into();
    t.output = output.into();
    save(root, &t)?;
    log_action(root, now, &t, "做");
    Ok(t)
}

/// 机主点了"这个程序以后都行"：做这一次，并在地图上给这个程序记一行。之后它在这个程序里动手不再递票。
pub fn decide_always(root: &Root, now: &str, id: &str) -> Result<Ticket> {
    let t = decide(root, now, id, true)?;
    if let Some(act) = &t.ui {
        let row = crate::places::Place { path: crate::ui::app_row(&act.app), what: format!("{}：可以替你点和填", crate::pulse::friendly(&act.app)), see: false, put: false, edit: false, drop: false, run: true, source: format!("你点头的 {}", now.chars().take(10).collect::<String>()) };
        crate::places::upsert(root, &row)?;
    }
    Ok(t)
}

/// 三样要改机器的安装，各递成一张票，机主点了才装，都能撤回。
/// 每样返回：简称、票的标题、在哪、脚本、撤回的脚本。先生成出来检查过，再递。
pub fn setup_scripts(root: &Root) -> Result<Vec<(String, String, String, String, String)>> {
    let exe = root.cli_exe().unwrap_or_else(|| root.path().join("ysoul.exe"));
    let (exe, home) = (exe.display().to_string(), root.path().display().to_string());
    let bin = root.path().join("bin");
    std::fs::create_dir_all(&bin)?;
    // ys：终端里的短命令。文件放在项目里，票只负责把这个目录加进 PATH
    std::fs::write(bin.join("ys.cmd"), format!("@echo off\r\n\"{exe}\" --root \"{home}\" say %*\r\n"))?;
    let bin = bin.display().to_string();
    // 注册表的这个位置名字里有个星号。用 .NET 的接口去写，免得星号被当成通配符
    let menu = |key: &str| format!("$k = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey('Software\\Classes\\{key}\\shell\\ysoul')\n$k.SetValue('', '交给 ysoul')\n$k.SetValue('Position', 'Top')\n$c = $k.CreateSubKey('command')\n$c.SetValue('', '\"{exe}\" --root \"{home}\" hand \"%1\"')\n$c.Close(); $k.Close()\n");
    let unmenu = |key: &str| format!("[Microsoft.Win32.Registry]::CurrentUser.DeleteSubKeyTree('Software\\Classes\\{key}\\shell\\ysoul', $false)\n");
    let hook = format!("\"{}\" --root \"{}\" heard", exe.replace('\\', "/"), home.replace('\\', "/"));
    let cursor = format!("\"{}\" --root \"{}\" heard --tool Cursor", exe.replace('\\', "/"), home.replace('\\', "/"));
    let s = |a: &str, b: &str, c: &str, d: String, e: String| (a.to_string(), b.to_string(), c.to_string(), d, e);
    Ok(vec![
        s("start", "在开始菜单里放一个「问 ysoul」：在系统自带的搜索里打 ysoul 或者「问」就能叫出那一行输入；也可以把它固定到任务栏", "你这个用户的开始菜单（%APPDATA%\\Microsoft\\Windows\\Start Menu\\Programs）", format!("$lnk = Join-Path $env:APPDATA 'Microsoft\\Windows\\Start Menu\\Programs\\问 ysoul.lnk'\n$s = (New-Object -ComObject WScript.Shell).CreateShortcut($lnk)\n$s.TargetPath = '{}'\n$s.Arguments = '--quick'\n$s.WorkingDirectory = '{home}'\n$s.Description = '就地问 ysoul 一句'\n$s.Save()\n'放好了。按一下 Win 键，打 ysoul，回车。'", root.app_exe().unwrap_or_else(|| root.path().join("ysoul-app.exe")).display()), "Remove-Item -LiteralPath (Join-Path $env:APPDATA 'Microsoft\\Windows\\Start Menu\\Programs\\问 ysoul.lnk') -Force -ErrorAction SilentlyContinue".to_string()),
        // Codex 只有一个通知位，已经被它自己的组件占着。不顶掉它：把那一行改成先经过 ysoul 的"转达"，转达会照原样叫回原来那个
        s("codex", "让 Codex 每干完一轮活也告诉我一声（它原来的通知照常，只是先经过我转一道）", "你的 Codex 设置 ~/.codex/config.toml 的 notify 那一行", format!(
            r#"$f = Join-Path $env:USERPROFILE '.codex\config.toml'
$lines = @(Get-Content -LiteralPath $f -Encoding utf8)
$i = -1; for ($k = 0; $k -lt $lines.Count; $k++) {{ if ($lines[$k] -match '^\s*notify\s*=\s*\[.*\]\s*$') {{ $i = $k; break }} }}
if ($i -lt 0) {{ Write-Error '没找到写在一行里的 notify，没有改。'; exit 1 }}
if ($lines[$i] -match '"relay"') {{ '已经挂过了，没有再改。'; exit 0 }}
Copy-Item -LiteralPath $f -Destination "$f.ysoul-bak" -Force
$inner = (($lines[$i] -replace '^\s*notify\s*=\s*\[', '') -replace '\]\s*$', '').Trim()
$lines[$i] = 'notify = [ "{exe2}", "--root", "{home2}", "relay", "--tool", "Codex", "--", ' + $inner + ' ]'
Set-Content -LiteralPath $f -Value $lines -Encoding utf8
'挂好了。原来的设置备份在 config.toml.ysoul-bak。'"#,
            exe2 = exe.replace('\\', "\\\\"),
            home2 = home.replace('\\', "\\\\")
        ), "$f = Join-Path $env:USERPROFILE '.codex\\config.toml'\nif (Test-Path \"$f.ysoul-bak\") { Copy-Item -LiteralPath \"$f.ysoul-bak\" -Destination $f -Force }".to_string()),
        s("cursor","让 Cursor 每干完一轮活告诉我一声（只说在哪个目录，不说干了什么）", "你的 Cursor 设置 ~/.cursor/hooks.json 的 hooks.stop", format!("$f = Join-Path $env:USERPROFILE '.cursor\\hooks.json'\nCopy-Item -LiteralPath $f -Destination \"$f.ysoul-bak\" -Force\n$j = Get-Content -Raw -LiteralPath $f | ConvertFrom-Json -AsHashtable\nif (-not $j.hooks) {{ $j.hooks = @{{}} }}\nif (-not $j.hooks.stop) {{ $j.hooks.stop = @() }}\n$cmd = '{cursor}'\nif (-not ((ConvertTo-Json -InputObject @($j.hooks.stop) -Depth 8) -match 'heard')) {{ $j.hooks.stop = @($j.hooks.stop) + @{{ command = $cmd }} }}\n$j | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $f -Encoding utf8\n'挂好了。原来的设置备份在 hooks.json.ysoul-bak。'"), "$f = Join-Path $env:USERPROFILE '.cursor\\hooks.json'\nif (Test-Path \"$f.ysoul-bak\") { Copy-Item -LiteralPath \"$f.ysoul-bak\" -Destination $f -Force }".to_string()),
        s("menu", "在资源管理器的右键菜单里加一项「交给 ysoul」", "注册表 HKCU\\Software\\Classes（只对你这个用户）", format!("{}{}'装好了。右键一个文件或文件夹就能看到。'", menu("*"), menu("Directory")), format!("{}{}", unmenu("*"), unmenu("Directory"))),
        s("ys", "让终端里能用 ys 这个短命令（ys 你好）", "你这个用户的 PATH 环境变量", format!("$p = [Environment]::GetEnvironmentVariable('Path', 'User')\nif (($p -split ';') -notcontains '{bin}') {{ [Environment]::SetEnvironmentVariable('Path', ($p.TrimEnd(';') + ';{bin}'), 'User') }}\n'加好了。新开的终端里可以用 ys。'"), format!("$p = [Environment]::GetEnvironmentVariable('Path', 'User')\n[Environment]::SetEnvironmentVariable('Path', (($p -split ';' | Where-Object {{ $_ -and $_ -ne '{bin}' }}) -join ';'), 'User')")),
        s("claude", "让 Claude Code 每干完一轮活告诉我一声（只说在哪个目录，不说干了什么）", "你的 Claude Code 设置 ~/.claude/settings.json 的 hooks.Stop", format!("$f = Join-Path $env:USERPROFILE '.claude\\settings.json'\nCopy-Item -LiteralPath $f -Destination \"$f.ysoul-bak\" -Force\n$j = Get-Content -Raw -LiteralPath $f | ConvertFrom-Json -AsHashtable\nif (-not $j.hooks) {{ $j.hooks = @{{}} }}\nif (-not $j.hooks.Stop) {{ $j.hooks.Stop = @() }}\n$cmd = '{hook}'\nif (-not (($j.hooks.Stop | ConvertTo-Json -Depth 8) -match 'heard')) {{ $j.hooks.Stop = @($j.hooks.Stop) + @{{ hooks = @(@{{ type = 'command'; command = $cmd }}) }} }}\n$j | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $f -Encoding utf8\n'挂好了。原来的设置备份在 settings.json.ysoul-bak。'"), "$f = Join-Path $env:USERPROFILE '.claude\\settings.json'\nif (Test-Path \"$f.ysoul-bak\") { Copy-Item -LiteralPath \"$f.ysoul-bak\" -Destination $f -Force }".to_string()),
    ])
}

/// 这台机器上能"不开界面、直接交给它一件活"的 AI 工具，和各自的叫法。
pub const AGENTS: &[(&str, &str, &[&str])] = &[
    ("claude", "Claude Code", &["-p", "{task}", "--permission-mode", "acceptEdits"]),
    ("codex", "Codex", &["exec", "{task}"]),
    ("gemini", "Gemini CLI", &["-p", "{task}"]),
    ("cursor-agent", "Cursor", &["-p", "{task}"]),
    ("aider", "aider", &["--message", "{task}", "--yes"]),
];

/// 把一件活交给另一个 AI 工具，在某个目录里做。那个工具会自己读写那个目录里的文件，不经过 ysoul 的门，
/// 所以这一定是一张票：机主看得到交给谁、在哪、交的是什么，点了才开始。撤不回（它改了什么要看那个目录的 git）。
pub fn issue_delegate(root: &Root, now: &str, agent: &str, cwd: &str, task: &str) -> Result<Ticket> {
    let Some((cmd, label, args)) = AGENTS.iter().find(|(c, ..)| *c == agent) else { bail!("不认识这个工具：{agent}。认识的有：{}", AGENTS.iter().map(|a| a.0).collect::<Vec<_>>().join("、")) };
    let dir = std::path::Path::new(cwd);
    if !dir.is_dir() {
        bail!("没有这个目录：{cwd}");
    }
    let (ok, why) = crate::places::allowed(&crate::places::load(root), root, dir, crate::places::Act::See);
    if !ok {
        bail!("{why}");
    }
    let task = task.trim();
    if task.is_empty() {
        bail!("要写清交给它做什么");
    }
    // 交的话放进一个单引号的 here-string，原样传过去；里面要是有会提前结束它的那两个字符，拆开
    let body = task.replace("\n'@", "\n' @");
    let argv: Vec<String> = args.iter().map(|a| if *a == "{task}" { "$task".to_string() } else { format!("'{a}'") }).collect();
    let script = format!("Set-Location -LiteralPath '{}'\n$task = @'\n{body}\n'@\n& {cmd} {}\nif ($LASTEXITCODE) {{ exit $LASTEXITCODE }}", cwd.replace('\'', "''"), argv.join(" "));
    issue(root, now, &format!("把一件活交给 {label}：{}", task.chars().take(48).collect::<String>()), cwd, &script, "", false)
}

/// 递一张改地图的票：这个地方以后能做什么。机主点了头，程序才把这一行写进地图。
pub fn issue_map(root: &Root, now: &str, place: crate::places::Place) -> Result<Ticket> {
    if place.path.trim().is_empty() {
        bail!("要写清是哪个地方");
    }
    let can: Vec<&str> = [(place.see, "看"), (place.put, "放"), (place.edit, "改"), (place.drop, "扔"), (place.run, "运行")].iter().filter(|(b, _)| *b).map(|(_, w)| *w).collect();
    let title = format!("往地图上记：这里我以后可以{}", if can.is_empty() { "什么都不做（连看也不看）".to_string() } else { can.join("、") });
    let mut t = issue(root, now, &title, &place.path, &place.row(), "把这一行从地图上撤掉", false)?;
    t.map = Some(place);
    save(root, &t)?;
    Ok(t)
}

/// 运行一段脚本。返回成没成，和它的输出。
fn run(root: &Root, id: &str, kind: &str, script: &str, elevated: bool) -> Result<(bool, String)> {
    std::fs::create_dir_all(dir(root))?;
    let ps1 = dir(root).join(format!("{id}.{kind}.ps1"));
    // 带上 BOM，并让输出走 UTF-8，中文路径和中文输出才不会乱
    std::fs::write(&ps1, format!("\u{feff}[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)\n{script}"))?;
    if !elevated {
        let mut cmd = Command::new("pwsh");
        crate::quiet::quiet(&mut cmd);
        let out = cmd.args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File"]).arg(&ps1).output().context("启动不了 PowerShell 7")?;
        let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
        return Ok((out.status.success(), text.trim().to_string()));
    }
    // 提权运行：另起一个提了权的 PowerShell，把输出写进文件再读回来
    let log = dir(root).join(format!("{id}.{kind}.out.txt"));
    let _ = std::fs::remove_file(&log);
    let inner = format!("& '{}' *> '{}'; exit $LASTEXITCODE", ps1.display(), log.display());
    let outer = format!("$p = Start-Process pwsh -Verb RunAs -Wait -PassThru -WindowStyle Hidden -ArgumentList '-NoProfile','-NonInteractive','-ExecutionPolicy','Bypass','-Command',\"{}\"; exit $p.ExitCode", inner.replace('"', "`\""));
    let mut cmd = Command::new("pwsh");
    crate::quiet::quiet(&mut cmd);
    let out = cmd.args(["-NoProfile", "-NonInteractive", "-Command", &outer]).output().context("启动不了 PowerShell 7")?;
    let text = std::fs::read_to_string(&log).unwrap_or_else(|_| String::from_utf8_lossy(&out.stderr).to_string());
    Ok((out.status.success(), text.trim_start_matches('\u{feff}').trim().to_string()))
}

fn log_action(root: &Root, now: &str, t: &Ticket, what: &str) {
    use std::io::Write;
    let line = serde_json::json!({ "at": now, "ticket": t.id, "what": what, "title": t.title, "place": t.place, "status": t.status, "can_undo": !t.undo.trim().is_empty() });
    let _ = std::fs::create_dir_all(root.local());
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(root.local().join("actions.jsonl")) {
        let _ = writeln!(f, "{line}");
    }
}

/// 机主在这个程序里点过几次"做"了（算上这一次）。
fn approvals(root: &Root, app: &str) -> u32 {
    let file = root.local().join("approvals.json");
    let mut all: std::collections::HashMap<String, u32> = std::fs::read_to_string(&file).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    let n = all.entry(app.to_string()).or_insert(0);
    *n += 1;
    let n = *n;
    let _ = std::fs::write(&file, serde_json::to_string(&all).unwrap_or_default());
    n
}

/// 机主对一张票的决定。点头就执行，摇头就作废。
pub fn decide(root: &Root, now: &str, id: &str, yes: bool) -> Result<Ticket> {
    let mut t = get(root, id)?;
    if t.status != "waiting" {
        bail!("这张票已经处理过了（{}）", t.status);
    }
    if !yes {
        t.status = "refused".into();
    } else {
        match &t.map {
            Some(place) => {
                // 被换掉的那一行存进 undo，撤回时换回去
                t.undo = crate::places::upsert(root, place)?.unwrap_or_else(|| "把这一行从地图上撤掉".into());
                t.status = "done".into();
                t.output = "记进地图了".into();
            }
            None if t.ui.is_some() => match crate::ui::perform(root, t.ui.as_ref().unwrap()) {
                Ok(how) => {
                    t.status = "done".into();
                    t.output = how;
                    // 适应他的授权：同一个程序里他点过两次"做"，就当他放开了这个程序，记到地图上，以后直接动
                    let app = t.ui.as_ref().unwrap().app.clone();
                    if approvals(root, &app) >= 2 && !crate::ui::app_allowed(&crate::places::load(root), &app) {
                        let row = crate::places::Place { path: crate::ui::app_row(&app), what: format!("{}：可以替你点和填", crate::pulse::friendly(&app)), see: false, put: false, edit: false, drop: false, run: true, source: format!("你连着点了两次头 {}", now.chars().take(10).collect::<String>()) };
                        if crate::places::upsert(root, &row).is_ok() {
                            t.output.push_str(&format!("\n（这是你第二次同意它在{}里动手。已经记到地图上，以后在这个程序里它直接动，不再递票。要收回，在家的地图里把那一格点掉。）", crate::pulse::friendly(&app)));
                        }
                    }
                }
                Err(e) => {
                    t.status = "failed".into();
                    t.output = format!("{e:#}");
                }
            },
            None => {
                let (ok, output) = run(root, &t.id, "do", &t.script, t.elevated)?;
                t.status = if ok { "done" } else { "failed" }.into();
                t.output = output;
            }
        }
    }
    save(root, &t)?;
    log_action(root, now, &t, if yes { "做" } else { "不做" });
    Ok(t)
}

/// 撤回一张做完的票。
pub fn undo(root: &Root, now: &str, id: &str) -> Result<Ticket> {
    let mut t = get(root, id)?;
    if t.status != "done" && t.status != "failed" {
        bail!("这张票还没做过，谈不上撤回");
    }
    if let Some(place) = &t.map {
        crate::places::remove(root, &place.path, Some(t.undo.as_str()).filter(|s| s.starts_with('|')))?;
        t.status = "undone".into();
        t.output = "从地图上撤掉了".into();
        save(root, &t)?;
        log_action(root, now, &t, "撤回");
        return Ok(t);
    }
    if t.undo.trim().is_empty() {
        bail!("这张票没写撤回的办法");
    }
    let (ok, output) = run(root, &t.id, "undo", &t.undo, t.elevated)?;
    if !ok {
        bail!("撤回没成：{output}");
    }
    t.status = "undone".into();
    t.output = output;
    save(root, &t)?;
    log_action(root, now, &t, "撤回");
    Ok(t)
}

/// 把屏幕上的一块截成图片存到本机的临时目录，返回图片的路径。只截一张，不连续截。
pub fn snapshot(root: &Root, left: i32, top: i32, right: i32, bottom: i32) -> Result<PathBuf> {
    let (w, h) = (right - left, bottom - top);
    if w < 50 || h < 50 {
        bail!("这个窗口太小，没什么可看的");
    }
    let tmp = root.local().join("tmp");
    std::fs::create_dir_all(&tmp)?;
    let out = tmp.join("look.png");
    // 太宽的缩到 1600 像素以内，够看清字就行
    let script = format!(
        "Add-Type -AssemblyName System.Drawing\nAdd-Type -Namespace Y -Name D -MemberDefinition '[DllImport(\"user32.dll\")] public static extern bool SetProcessDPIAware();'\n[void][Y.D]::SetProcessDPIAware()\n$b = New-Object Drawing.Bitmap {w}, {h}\n$g = [Drawing.Graphics]::FromImage($b)\n$g.CopyFromScreen({left}, {top}, 0, 0, $b.Size)\n$s = [Math]::Min(1.0, 1600 / {w})\n$o = New-Object Drawing.Bitmap ([int]({w} * $s)), ([int]({h} * $s))\n$g2 = [Drawing.Graphics]::FromImage($o)\n$g2.InterpolationMode = 'HighQualityBicubic'\n$g2.DrawImage($b, 0, 0, $o.Width, $o.Height)\n$o.Save('{}')\n",
        out.display()
    );
    let (ok, output) = run(root, "look", "shot", &script, false)?;
    if !ok || !out.exists() {
        bail!("没截成：{output}");
    }
    crate::marks::save_frame(root, crate::marks::Frame { left, top, scale: (1600.0 / w as f64).min(1.0) })?;
    Ok(out)
}

/// 用系统默认的程序打开一个文件、文件夹或网址。
/// 不经过命令解释器：经过它的话，网址里一个 & 就能夹带别的命令。
pub fn open(target: &str) -> Result<()> {
    if !openable(target) {
        bail!("这个地址里有不该有的字符，不打开");
    }
    Command::new("rundll32").args(["url.dll,FileProtocolHandler", target]).spawn().context("打不开")?;
    Ok(())
}

/// 能不能交给系统去打开：不带引号和控制字符；是网址的话只认 http 和 https，且中间没有空白。
pub fn openable(target: &str) -> bool {
    if target.trim().is_empty() || target.chars().any(|c| c.is_control() || c == '"') {
        return false;
    }
    match target.split_once("://") {
        Some((scheme, rest)) => (scheme == "http" || scheme == "https") && !rest.is_empty() && !target.chars().any(char::is_whitespace),
        None => true,
    }
}

/// 在资源管理器里定位到一个文件。
pub fn reveal(path: &str) -> Result<()> {
    Command::new("explorer").arg(format!("/select,{}", path.replace('/', "\\"))).spawn().context("打不开资源管理器")?;
    Ok(())
}

/// 最近几张票的情况，写给它自己看的：上一轮递出去的票后来怎么样了。
pub fn recent_summary(root: &Root, n: usize) -> String {
    let word = |s: &str| match s {
        "waiting" => "等机主点头",
        "done" => "做完了",
        "failed" => "做了但失败",
        "refused" => "机主说不做",
        "undone" => "已撤回",
        _ => "不明",
    };
    list(root).iter().take(n).map(|t| {
        let mut line = format!("- {}　{}　【{}】　{}", t.id, t.at, word(&t.status), t.title);
        if t.status == "failed" && !t.output.is_empty() {
            line.push_str(&format!("　输出：{}", t.output.chars().take(200).collect::<String>().replace('\n', " ")));
        }
        line
    }).collect::<Vec<_>>().join("\n")
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
    fn issuing_writes_a_waiting_ticket_and_does_not_run_it() {
        let (d, r) = root();
        let marker = d.path().join("ran.txt");
        let t = issue(&r, "2026-10-12 01:00:00", "建一个标记文件", "临时目录", &format!("Set-Content -LiteralPath '{}' -Value x", marker.display()), "", false).unwrap();
        assert_eq!(t.status, "waiting");
        assert!(!marker.exists(), "递票不等于执行");
        assert_eq!(list(&r).len(), 1);
    }

    #[test]
    fn a_ticket_needs_a_title_and_a_script() {
        let (_d, r) = root();
        assert!(issue(&r, "now", "", "x", "echo 1", "", false).is_err());
        assert!(issue(&r, "now", "x", "x", "  ", "", false).is_err());
    }

    #[test]
    fn two_tickets_in_the_same_second_get_different_ids() {
        let (_d, r) = root();
        let a = issue(&r, "2026-10-12 01:00:00", "a", "", "echo 1", "", false).unwrap();
        let b = issue(&r, "2026-10-12 01:00:00", "b", "", "echo 1", "", false).unwrap();
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn refusing_marks_the_ticket_and_runs_nothing() {
        let (d, r) = root();
        let marker = d.path().join("ran.txt");
        let t = issue(&r, "now1", "x", "", &format!("Set-Content -LiteralPath '{}' -Value x", marker.display()), "", false).unwrap();
        let t = decide(&r, "now2", &t.id, false).unwrap();
        assert_eq!(t.status, "refused");
        assert!(!marker.exists());
        assert!(decide(&r, "now3", &t.id, true).is_err(), "处理过的票不能再决定一次");
    }

    #[test]
    fn approving_runs_the_script_and_undo_reverses_it() {
        let (d, r) = root();
        let marker = d.path().join("做了.txt");
        let t = issue(&r, "now1", "建一个标记文件", "临时目录", &format!("Set-Content -LiteralPath '{}' -Value 好; '建好了'", marker.display()), &format!("Remove-Item -LiteralPath '{}'", marker.display()), false).unwrap();
        let t = decide(&r, "now2", &t.id, true).unwrap();
        assert_eq!(t.status, "done", "{}", t.output);
        assert!(marker.exists());
        assert!(t.output.contains("建好了"));
        let t = undo(&r, "now3", &t.id).unwrap();
        assert_eq!(t.status, "undone");
        assert!(!marker.exists());
        let log = std::fs::read_to_string(r.local().join("actions.jsonl")).unwrap();
        assert_eq!(log.lines().count(), 2);
    }

    #[test]
    fn a_map_ticket_changes_the_map_only_after_approval_and_can_be_undone() {
        let (_d, r) = root();
        let place = crate::places::Place { path: "D:/work".into(), what: "干活的地方".into(), see: true, put: true, edit: true, drop: false, run: false, source: "你说的".into() };
        let t = issue_map(&r, "now1", place).unwrap();
        assert!(crate::places::load(&r).is_empty(), "递票不改地图");
        let t = decide(&r, "now2", &t.id, true).unwrap();
        assert_eq!(t.status, "done");
        assert!(crate::places::find(&crate::places::load(&r), "D:/work/a").unwrap().edit);
        undo(&r, "now3", &t.id).unwrap();
        assert!(crate::places::load(&r).is_empty());
    }

    #[test]
    fn only_plain_paths_and_web_addresses_can_be_opened() {
        assert!(openable("D:\\SOUL\\docs\\00-requirements.md"));
        assert!(openable("https://example.com/a?b=1&c=2"));
        assert!(!openable("https://example.com/\" & calc"));
        assert!(!openable("https://example.com/ x"));
        assert!(!openable("file:///C:/Windows/System32/calc.exe"));
        assert!(!openable("ms-settings://x"));
        assert!(!openable("javascript://alert(1)"));
        assert!(!openable("  "));
    }

    #[test]
    fn delegating_is_always_a_ticket_and_keeps_the_task_verbatim() {
        let (d, r) = root();
        let work = d.path().join("work");
        std::fs::create_dir_all(&work).unwrap();
        let t = issue_delegate(&r, "now", "claude", &work.to_string_lossy(), "把 README 里的错别字改掉\n不要动代码").unwrap();
        assert_eq!(t.status, "waiting");
        assert!(t.title.contains("Claude Code") && t.script.contains("不要动代码") && t.script.contains("& claude '-p' $task"), "{}", t.script);
        assert!(issue_delegate(&r, "now", "nobody", &work.to_string_lossy(), "x").is_err());
        assert!(issue_delegate(&r, "now", "claude", "Z:\\没有这个目录", "x").is_err());
        assert!(issue_delegate(&r, "now", "claude", &work.to_string_lossy(), "  ").is_err());
    }

    #[test]
    fn a_failing_script_is_recorded_as_failed_with_its_output() {
        let (_d, r) = root();
        let t = issue(&r, "now1", "注定失败", "", "Write-Error '不行'; exit 3", "", false).unwrap();
        let t = decide(&r, "now2", &t.id, true).unwrap();
        assert_eq!(t.status, "failed");
        assert!(t.output.contains("不行"));
        assert!(recent_summary(&r, 5).contains("做了但失败"));
    }
}
