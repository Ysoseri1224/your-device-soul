//! ysoul 的 runtime。现阶段是"脑在终端里"：能对话，能读写记忆，记忆有改动就提交。

use anyhow::Result;
use clap::{Parser, Subcommand};
use ysoul::mind::{self, now, today};
use ysoul::paths::Root;
use ysoul::{birth, cheap, context, outbound};
use std::io::{BufRead, Write};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "ysoul", about = "这台机器的灵魂", version)]
struct Cli {
    /// 项目根目录。不给就从当前目录往上找。
    #[arg(long, global = true)]
    root: Option<PathBuf>,
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// 说一句，听它回一句
    Say { text: Vec<String> },
    /// 在终端里聊。空行或 Ctrl+Z 结束。加 --resume 接着上一次的会话
    Chat {
        #[arg(long)]
        resume: bool,
    },
    /// 出生：体检、摘要、认识身体、写下自述。只能做一次
    Birth,
    /// 看每次思考必带的那一份有多大；加 --full 看全文
    Context {
        #[arg(long)]
        full: bool,
    },
    /// 现查此刻：前台窗口、能落脚的台面、身体状况。只读
    Sense,
    /// 给它一个时机，看它想不想说一句。多数时候它不说
    Muse { occasion: Vec<String> },
    /// 把今天攒下的经历理一理（平时由桌面上那只在机主走开时自己做）
    Reflect,
    /// 按名字找这台机器上的程序；加 --open 就把对得最准的那个打开
    App {
        name: Vec<String>,
        #[arg(long)]
        open: bool,
    },
    /// 让它的门看一条命令并照规矩处理：只读的直接跑，别的转成票，不能看的不跑
    Run { command: Vec<String> },
    /// 普查的第二遍：顺着清单里指向的东西去看它到底是什么
    Follow,
    /// 别的 AI 工具干完一轮活时来说一声（挂在它们的钩子上）。从标准输入读钩子送来的 JSON
    Heard {
        #[arg(long, default_value = "Claude Code")]
        tool: String,
    },
    /// 把一个文件或文件夹交给桌面上那只（资源管理器右键菜单用）
    Hand { path: String },
    /// 把三样要改机器的安装各递成一张票：右键菜单里的"交给 ysoul"、终端里的 ys 命令、Claude Code 的钩子
    Setup {
        /// 只递其中几样：menu、ys、claude、cursor。不写就是全部
        only: Vec<String>,
        /// 只把脚本打出来看，不递票
        #[arg(long)]
        dry: bool,
    },
    /// 看程序会替它先翻到哪些笔记（回忆的前两级）
    Recall { text: Vec<String> },
    /// 量屏幕上一块地方"有多满"（左 上 宽 高，真实像素）。调猫躲字的门槛用
    Busy { left: i32, top: i32, width: i32, height: i32 },
    /// 转达：别的工具只有一个通知位、而且已经被占着时用。先照原样运行它原来要通知的那个程序，再给自己记一笔
    Relay {
        #[arg(long)]
        tool: String,
        /// 原来的通知命令和它的参数。工具自己追加的那段 JSON 也在最后
        #[arg(last = true)]
        command: Vec<String>,
    },
    /// 模型通道：不带参数就列出这台机器上发现的；use 加编号是定下强模型按什么次序用（可以写几个）；cheap 加编号是定跑腿用哪条；auto 是恢复默认
    Channel { action: Option<String>, ids: Vec<String> },
    /// 那道门：Claude Code 调用文件工具之前的钩子。从标准输入读这次调用，不放行就以 2 退出
    Gate,
    /// 指向本机的 MCP 服务（走标准输入输出）
    Mcp,
    /// 查家的地图：这个路径能做什么
    Where { path: String },
    /// 票：不带参数列出最近的；do / no / undo 加编号，分别是做、不做、撤回
    Ticket { action: Option<String>, id: Option<String> },
    /// 派便宜的模型跑一件腿，用来试通道
    Errand {
        text: Vec<String>,
        #[arg(long, default_value = cheap::DEFAULT_MODEL)]
        model: String,
    },
}

/// 一轮对话：拼上下文、留底、问强模型、记忆有改动就提交。返回会话编号，下一轮接着用。
fn turn(root: &Root, user: &str, resume: Option<&str>) -> Result<Option<String>> {
    let t = mind::converse(root, user, resume)?;
    println!("{}", t.text);
    if !t.noted.is_empty() {
        eprintln!("（记下了：{}）", t.noted.join("、"));
    }
    if let Some(id) = &t.session {
        mind::remember_session(root, id);
    }
    Ok(t.session)
}

fn chat(root: &Root, resume: bool) -> Result<()> {
    let stdin = std::io::stdin();
    let mut session: Option<String> = if resume { mind::last_session(root) } else { None };
    loop {
        eprint!("> ");
        std::io::stderr().flush()?;
        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 || line.trim().is_empty() {
            return Ok(());
        }
        match turn(root, line.trim(), session.as_deref()) {
            Ok(id) => session = id.or(session),
            Err(e) => eprintln!("（我现在想不了：{e}）"),
        }
    }
}

/// 出生的四步。每一步做完都提交，所以断了可以重跑：体检和摘要会重做或用缓存，写过的笔记不会丢。
fn born(root: &Root) -> Result<()> {
    if birth::is_born(root) {
        anyhow::bail!("它已经出生了。出生只有一次。");
    }
    eprintln!("一、体检");
    birth::census(root)?;
    eprintln!("二、请便宜模型做摘要");
    let digests = birth::digests(root, &now())?;
    eprintln!("三、认识自己的身体（{} 份摘要）", digests.len());
    let first = birth::body(root, &now(), &today(), &digests)?;
    eprintln!("{}
", first.text.trim());
    eprintln!("四、写下自述");
    let opening = birth::selfhood(root, &now(), &today(), first.session_id.as_deref())?;
    println!("{}", opening.text.trim());
    Ok(())
}

fn show_context(root: &Root, full: bool) -> Result<()> {
    let ctx = context::assemble(root, &today())?;
    if full {
        println!("{}", ctx.system);
        return Ok(());
    }
    for (name, chars) in &ctx.parts {
        println!("{chars:>6} 字  {name}");
    }
    println!("{:>6} 字  合计", ctx.system.chars().count());
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    ysoul::sense::dpi_aware();
    let root = Root::discover(cli.root)?;
    match cli.command {
        Cmd::Say { text } => {
            // 他在自己的话里写了路径，就是让它看那一件
            ysoul::places::grant_from_words(&root, &text.join(" "));
            turn(&root, &text.join(" "), None).map(|_| ())
        }
        Cmd::Chat { resume } => chat(&root, resume),
        Cmd::Birth => born(&root),
        Cmd::Context { full } => show_context(&root, full),
        Cmd::Sense => {
            let (screen, wins) = (ysoul::sense::screen(), ysoul::sense::windows());
            let fore = ysoul::sense::foreground();
            let place = fore.as_ref().map(|f| ysoul::sense::guess_place(&f.title, &ysoul::sense::known_places(&root))).unwrap_or_default();
            let out = serde_json::json!({ "fore": fore, "place": place, "idle": ysoul::sense::idle_seconds(), "vitals": ysoul::sense::vitals(), "screen": screen, "ledges": ysoul::sense::ledges(&wins, &screen, 160), "windows": wins.len() });
            println!("{}", serde_json::to_string_pretty(&out)?);
            Ok(())
        }
        Cmd::Muse { occasion } => {
            match mind::muse(&root, &occasion.join(" "))? {
                Some(line) => println!("{line}"),
                None => eprintln!("（它选择不说）"),
            }
            Ok(())
        }
        Cmd::Reflect => {
            let noted = mind::reflect(&root)?;
            eprintln!("{}", if noted.is_empty() { "（没有改动）".to_string() } else { format!("（改了：{}）", noted.join("、")) });
            Ok(())
        }
        Cmd::App { name, open } => {
            let all = ysoul::apps::list(&root);
            let name = name.join(" ");
            let hits = ysoul::apps::find(&all, &name);
            for (app, score) in hits.iter().take(6) {
                println!("{score}　{}", app.name);
            }
            eprintln!("（开始菜单里共 {} 个程序；不问模型就直接打开的会是：{}）", all.len(), ysoul::apps::pick(&all, &name).map(|a| a.name.as_str()).unwrap_or("无，交给模型"));
            if open {
                if let Some((app, _)) = hits.first() {
                    ysoul::apps::launch(app)?;
                }
            }
            Ok(())
        }
        Cmd::Run { command } => {
            let cwd = std::env::current_dir()?;
            match ysoul::shell::run(&root, &now(), &command.join(" "), cwd.to_str(), "", "")? {
                ysoul::shell::Outcome::Ran(_, out) => println!("{out}"),
                ysoul::shell::Outcome::Ticket(t, why) => eprintln!("（没有直接跑：{why}。转成了票 {}）", t.id),
                ysoul::shell::Outcome::Refused(why) => eprintln!("（不跑：{why}）"),
            }
            Ok(())
        }
        Cmd::Follow => {
            let noted = mind::follow(&root)?;
            eprintln!("（改了：{}）", noted.join("、"));
            Ok(())
        }
        Cmd::Heard { tool } => {
            let mut json = String::new();
            std::io::Read::read_to_string(&mut std::io::stdin(), &mut json)?;
            // 各家工具送来的写法不一样：有的叫 cwd，有的给一串工作区目录
            let v = serde_json::from_str::<serde_json::Value>(&json).unwrap_or_default();
            let cwd = v["cwd"].as_str().or_else(|| v["workspace_roots"][0].as_str()).unwrap_or_default().to_string();
            ysoul::pulse::heard(&root, &tool, &cwd);
            Ok(())
        }
        Cmd::Hand { path } => {
            // 右键交来的：机主亲手点的，这一件放开给它看
            ysoul::places::grant(&root, std::path::Path::new(&path));
            std::fs::create_dir_all(root.local())?;
            std::fs::write(root.local().join("handoff.txt"), path)?;
            Ok(())
        }
        Cmd::Setup { only, dry } => {
            for (title, place, script, undo) in ysoul::hands::setup_scripts(&root)?.into_iter().filter(|(key, ..)| only.is_empty() || only.iter().any(|o| o == key)).map(|(_, a, b, c, d)| (a, b, c, d)) {
                if dry {
                    println!("== {title}\n{script}\n-- 撤回\n{undo}\n");
                } else {
                    let t = ysoul::hands::issue(&root, &now(), &title, &place, &script, &undo, false)?;
                    println!("{}　{}", t.id, t.title);
                }
            }
            Ok(())
        }
        Cmd::Recall { text } => {
            let found = ysoul::recall::attach(&root, &text.join(" "), true);
            println!("{}", if found.is_empty() { "（没翻到，交给它自己去翻）" } else { &found });
            Ok(())
        }
        Cmd::Busy { left, top, width, height } => {
            let p = &ysoul::sense::profiles(0, &[(left, top, left + width, top + height)], 9)[0];
            println!("整块 {:.3}　最满的一条 {:.3}　各条：{}", ysoul::sense::busyness(0, &[(left, top, left + width, top + height)])[0], p.iter().cloned().fold(0.0f32, f32::max), p.iter().map(|v| format!("{:.0}", v * 100.0)).collect::<Vec<_>>().join(" "));
            Ok(())
        }
        Cmd::Relay { tool, command } => {
            // 原来的那个照常运行，参数一个不动
            if let Some((exe, args)) = command.split_first() {
                let _ = std::process::Command::new(exe).args(args).status();
            }
            // 最后一个参数是工具给的 JSON：里面有在哪个目录就记一行，没有就不记
            let v = command.last().and_then(|j| serde_json::from_str::<serde_json::Value>(j).ok()).unwrap_or_default();
            if let Some(cwd) = v["cwd"].as_str().or_else(|| v["workspace_roots"][0].as_str()) {
                ysoul::pulse::heard(&root, &tool, cwd);
            }
            Ok(())
        }
        Cmd::Channel { action, ids } => {
            use ysoul::channels;
            let all = channels::discover();
            let mut choice = channels::choice(&root);
            match action.as_deref() {
                None => {}
                Some("use") if !ids.is_empty() && ids.iter().all(|i| all.iter().any(|c| &c.id == i)) => choice.strong = ids,
                Some("cheap") if ids.len() == 1 && all.iter().any(|c| c.id == ids[0]) => choice.cheap = Some(ids[0].clone()),
                Some("auto") => choice = channels::Choice::default(),
                _ => anyhow::bail!("用法：ysoul channel [use 编号…|cheap 编号|auto]。编号照下面列的写。"),
            }
            if action.is_some() {
                channels::save_choice(&root, &choice)?;
            }
            let order: Vec<String> = channels::strong_order(&all, &choice).into_iter().map(|c| c.id).collect();
            let cheap = channels::cheap_one(&all, &choice).map(|c| c.id);
            for c in &all {
                let mark = match order.iter().position(|i| i == &c.id) { Some(n) => format!("强{}", n + 1), None => "　".into() };
                println!("{mark}{}　{}　{}　[{}]", if cheap.as_deref() == Some(&c.id) && choice.cheap.is_some() { "跑腿" } else { "　　" }, c.id, c.name, c.source);
            }
            if all.is_empty() {
                println!("（没有发现任何通道）");
            }
            Ok(())
        }
        Cmd::Gate => {
            let mut json = String::new();
            std::io::Read::read_to_string(&mut std::io::stdin(), &mut json)?;
            let (ok, why) = ysoul::gate::from_hook(&root, &json);
            if !ok {
                eprintln!("{why}");
                std::process::exit(2);
            }
            Ok(())
        }
        Cmd::Mcp => ysoul::mcp::serve(&root),
        Cmd::Where { path } => {
            use ysoul::places::{self, Act};
            let map = places::load(&root);
            for act in [Act::See, Act::Put, Act::Edit, Act::Drop, Act::Run] {
                let (ok, why) = places::allowed(&map, &root, std::path::Path::new(&path), act);
                println!("{}　{}　{}", act.word(), if ok { "可以" } else { "不行" }, why);
            }
            Ok(())
        }
        Cmd::Ticket { action, id } => {
            use ysoul::hands;
            let t = match (action.as_deref(), id) {
                (None, _) => {
                    println!("{}", hands::recent_summary(&root, 20));
                    return Ok(());
                }
                (Some("do"), Some(id)) => hands::decide(&root, &now(), &id, true)?,
                (Some("no"), Some(id)) => hands::decide(&root, &now(), &id, false)?,
                (Some("undo"), Some(id)) => hands::undo(&root, &now(), &id)?,
                _ => anyhow::bail!("用法：ysoul ticket [do|no|undo 编号]"),
            };
            println!("{}　{}
{}", t.id, t.status, t.output);
            Ok(())
        }
        Cmd::Errand { text, model } => {
            let user = text.join(" ");
            outbound::log(&root, &now(), "cheap", &model, &user)?;
            println!("{}", cheap::errand("简明地用中文回答。", &user, &model, 300)?);
            Ok(())
        }
    }
}
