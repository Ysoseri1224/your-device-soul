//! 家的地图：这台机器上每个地方，它能做什么。
//!
//! 地图是记忆里的一份笔记（`us/places.md`），里面一张表，机主可以直接改。它每问清一处，就往表里添一行。
//! 这里负责读懂那张表，并回答一个问题：对这个路径做这件事，行不行。
//! 这是程序在把关，不是提示词里的一句"不要碰"（见 10 第二节关于"模型指导"和"程序强制"的区分）。

use crate::paths::Root;
use serde::Serialize;
use std::path::{Path, PathBuf};

/// 对一个地方能做的五件事。
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum Act {
    /// 看里面有什么、读内容
    See,
    /// 往里放新东西
    Put,
    /// 改已有的东西
    Edit,
    /// 扔掉
    Drop,
    /// 运行里面的程序
    Run,
}

impl Act {
    pub fn word(self) -> &'static str {
        match self {
            Act::See => "看",
            Act::Put => "放",
            Act::Edit => "改",
            Act::Drop => "扔",
            Act::Run => "运行",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, serde::Deserialize)]
pub struct Place {
    pub path: String,
    pub what: String,
    pub see: bool,
    pub put: bool,
    pub edit: bool,
    pub drop: bool,
    pub run: bool,
    /// 依据：机主哪天说的，或者"它猜的"
    pub source: String,
}

impl Place {
    pub fn allows(&self, act: Act) -> bool {
        match act {
            Act::See => self.see,
            Act::Put => self.put,
            Act::Edit => self.edit,
            Act::Drop => self.drop,
            Act::Run => self.run,
        }
    }
}

/// 读懂笔记里的那张表。表头和分隔行跳过；勾是"✓"，其余都算不行。
pub fn parse(md: &str) -> Vec<Place> {
    let mut out = Vec::new();
    for line in md.lines() {
        let line = line.trim();
        if !line.starts_with('|') {
            continue;
        }
        let cells: Vec<&str> = line.trim_matches('|').split('|').map(|c| c.trim()).collect();
        if cells.len() < 8 || cells[0] == "地方" || cells[0].starts_with("---") || cells[0].starts_with(":-") {
            continue;
        }
        let yes = |i: usize| cells[i].contains('✓');
        out.push(Place { path: cells[0].trim_matches('`').to_string(), what: cells[1].to_string(), see: yes(2), put: yes(3), edit: yes(4), drop: yes(5), run: yes(6), source: cells[7].to_string() });
    }
    out
}

pub fn file(root: &Root) -> PathBuf {
    root.memory().join("us").join("places.md")
}

pub fn private_file(root: &Root) -> PathBuf {
    root.path().join("sensors").join("census").join("shallow-dirs.txt")
}

/// 这个目录下面有没有不能看的地方。整个搜一个目录之前要问：搜上层目录会把下层不能看的东西一并搜出来。
pub fn hides_something(places: &[Place], dir: &Path) -> bool {
    let d = real(dir);
    places.iter().any(|pl| !pl.see && { let p = norm(&pl.path); p != d && inside(&p, &d) })
}

/// 机主亲手交来的东西：右键"交给 ysoul"、拖到箱子上、在自己的话里写了路径。
/// 私人名单管的是"它不主动去翻"；机主点名给它看的那一件，以机主的话为准。只放开"看"，只放开那一件，半小时后失效。
#[derive(Serialize, serde::Deserialize)]
struct Grant {
    path: String,
    until: i64,
}

fn grants_file(root: &Root) -> PathBuf {
    root.local().join("grants.json")
}

fn grants(root: &Root) -> Vec<Grant> {
    let now = chrono::Local::now().timestamp();
    std::fs::read_to_string(grants_file(root)).ok().and_then(|s| serde_json::from_str::<Vec<Grant>>(&s).ok()).unwrap_or_default().into_iter().filter(|g| g.until > now).collect()
}

/// 放开一件。是文件就只放开这个文件；是文件夹就放开这个文件夹里面的。
pub fn grant(root: &Root, path: &Path) {
    let mut all = grants(root);
    let p = real(path);
    all.retain(|g| g.path != p);
    all.push(Grant { path: p, until: chrono::Local::now().timestamp() + 1800 });
    let _ = std::fs::create_dir_all(root.local());
    let _ = std::fs::write(grants_file(root), serde_json::to_string(&all).unwrap_or_default());
}

fn granted(root: &Root, p: &str) -> bool {
    grants(root).iter().any(|g| inside(p, &g.path))
}

/// 从机主自己说的话里找出他写的路径（「」里的，或者盘符开头的），确实存在的就放开。返回放开了哪些。
/// 只能用在机主亲口说的话上。程序附上的、别处读来的文字不算。
pub fn grant_from_words(root: &Root, text: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find('「') {
        let Some(j) = rest[i..].find('」') else { break };
        found.push(rest[i + '「'.len_utf8()..i + j].to_string());
        rest = &rest[i + j + '」'.len_utf8()..];
    }
    let re = regex::Regex::new(r#"[A-Za-z]:[\\/][^\s「」"<>|*?，。；！？]+"#).unwrap();
    found.extend(re.find_iter(text).map(|m| m.as_str().trim_end_matches(['.', ',', ')', '）', '、']).to_string()));
    let mut out = Vec::new();
    for f in found {
        let p = Path::new(f.trim());
        if on_a_local_drive(&norm(&p.to_string_lossy())) && p.exists() && !out.contains(&f) {
            grant(root, p);
            out.push(f);
        }
    }
    out
}

/// 机主列为私人的目录。普查不往里看，它也不能看。名单放在程序这边，不写进地图，免得每次思考都把这些名字带出去。
pub fn private_list(root: &Root) -> Vec<Place> {
    let text = std::fs::read_to_string(private_file(root)).unwrap_or_default();
    text.lines().map(|l| l.trim().trim_start_matches('\u{feff}')).filter(|l| !l.is_empty() && !l.starts_with('#')).map(|l| Place { path: l.to_string(), what: "私人的".into(), see: false, put: false, edit: false, drop: false, run: false, source: "机主列为私人".into() }).collect()
}

/// 地图上的每一行，加上私人目录的名单。
pub fn load(root: &Root) -> Vec<Place> {
    let mut all = parse(&std::fs::read_to_string(file(root)).unwrap_or_default());
    all.extend(private_list(root));
    all
}

impl Place {
    /// 这一处在表里的那一行。
    pub fn row(&self) -> String {
        let m = |b: bool| if b { "✓" } else { "–" };
        format!("| `{}` | {} | {} | {} | {} | {} | {} | {} |", self.path, self.what.replace('|', "/"), m(self.see), m(self.put), m(self.edit), m(self.drop), m(self.run), self.source.replace('|', "/"))
    }
}

fn row_path(line: &str) -> Option<String> {
    parse(line).first().map(|p| norm(&p.path))
}

/// 往地图里添一行；这个地方已经有一行就换掉。返回被换掉的那一行，撤回时用。
pub fn upsert(root: &Root, place: &Place) -> std::io::Result<Option<String>> {
    let text = std::fs::read_to_string(file(root)).unwrap_or_default();
    let key = norm(&place.path);
    let mut old = None;
    let mut lines: Vec<String> = Vec::new();
    let mut last_row = None;
    for l in text.lines() {
        if row_path(l).as_deref() == Some(key.as_str()) {
            // 这个地方已经有一行：在原处换掉，别的行的次序不动
            old = Some(l.to_string());
            lines.push(place.row());
            continue;
        }
        if l.trim_start().starts_with('|') {
            last_row = Some(lines.len());
        }
        lines.push(l.to_string());
    }
    match last_row {
        _ if old.is_some() => {}
        Some(i) => lines.insert(i + 1, place.row()),
        None => {
            lines.push("| 地方 | 是什么 | 看 | 放 | 改 | 扔 | 运行 | 依据 |".into());
            lines.push("|---|---|---|---|---|---|---|---|".into());
            lines.push(place.row());
        }
    }
    std::fs::create_dir_all(file(root).parent().unwrap())?;
    std::fs::write(file(root), lines.join("\n") + "\n")?;
    Ok(old)
}

/// 把一个地方的那一行撤掉；给了原来的那一行就换回去。
pub fn remove(root: &Root, path: &str, restore: Option<&str>) -> std::io::Result<()> {
    let text = std::fs::read_to_string(file(root)).unwrap_or_default();
    let key = norm(path);
    let lines: Vec<String> = text.lines().filter_map(|l| if row_path(l).as_deref() == Some(key.as_str()) { restore.map(|r| r.to_string()) } else { Some(l.to_string()) }).collect();
    std::fs::write(file(root), lines.join("\n") + "\n")
}

/// 把路径写法统一，让同一个地方只有一种写法。不这样做，换个写法就能绕过地图：
/// 斜杠统一成反斜杠；去掉 `\\?\` 前缀；算掉 `.` 和 `..`；每一段去掉结尾的点和空格（Windows 会忽略它们）；
/// 去掉文件名后面的 `:流名`；全部小写。网络路径保留开头的两个反斜杠，后面会因为"不是本机盘上的路径"被拒。
pub fn norm(path: &str) -> String {
    let s = path.replace('/', "\\");
    let s = s.strip_prefix("\\\\?\\").unwrap_or(&s);
    let unc = s.starts_with("\\\\");
    let mut parts: Vec<String> = Vec::new();
    for raw in s.split('\\') {
        match raw {
            "" | "." => continue,
            ".." => {
                if parts.len() > 1 {
                    parts.pop();
                }
            }
            _ => {
                let mut seg = raw.trim_end_matches(['.', ' ']);
                if !parts.is_empty() {
                    seg = seg.split(':').next().unwrap_or(seg);
                }
                if !seg.is_empty() {
                    parts.push(seg.to_lowercase());
                }
            }
        }
    }
    format!("{}{}", if unc { "\\\\" } else { "" }, parts.join("\\"))
}

/// 是不是本机盘上的一个完整路径：盘符、冒号，然后结束或者接反斜杠。
fn on_a_local_drive(p: &str) -> bool {
    let b = p.as_bytes();
    b.len() >= 2 && b[0].is_ascii_lowercase() && b[1] == b':' && (b.len() == 2 || b[2] == b'\\')
}

/// 这个路径真正指向哪里。路径上要是有链接（目录联接、符号链接）或短文件名，换成它们指向的真名再比。
/// 做法：找到路径上最深的、确实存在的那一层，问系统它的真名，再把后面还不存在的部分接上。
pub fn real(path: &Path) -> String {
    let lexical = norm(&path.to_string_lossy());
    // 不是本机盘上的完整路径就不去问系统：只写了盘符的相对路径问出来的是"当前目录"，会认错
    if !on_a_local_drive(&lexical) {
        return lexical;
    }
    // 盘的根要带上反斜杠，否则 "d:" 的意思是"D 盘上的当前目录"
    let mut existing = PathBuf::from(if lexical.len() == 2 { format!("{lexical}\\") } else { lexical.clone() });
    let mut rest: Vec<String> = Vec::new();
    loop {
        if let Ok(c) = std::fs::canonicalize(&existing) {
            let mut out = norm(&c.to_string_lossy());
            for seg in rest.iter().rev() {
                out.push('\\');
                out.push_str(seg);
            }
            return out;
        }
        match (existing.file_name().map(|n| n.to_string_lossy().to_string()), existing.parent().map(|p| p.to_path_buf())) {
            (Some(name), Some(parent)) => {
                rest.push(name);
                existing = parent;
            }
            _ => return lexical,
        }
    }
}

fn inside(path: &str, dir: &str) -> bool {
    path == dir || (path.starts_with(dir) && path[dir.len()..].starts_with('\\'))
}

/// 这个路径属于地图上的哪个地方：取写得最具体（最长）的那一行。
pub fn find<'a>(places: &'a [Place], path: &str) -> Option<&'a Place> {
    let p = norm(path);
    places.iter().filter(|pl| inside(&p, &norm(&pl.path))).max_by_key(|pl| norm(&pl.path).len())
}

/// 对这个路径做这件事，行不行。返回行不行，以及一句说明。
pub fn allowed(places: &[Place], root: &Root, path: &Path, act: Act) -> (bool, String) {
    let p = real(path);
    if !on_a_local_drive(&p) {
        return (false, "这不是本机盘上的一个完整路径（网络路径、设备路径、只写了盘符的相对路径都不行），不放行。".into());
    }
    // 两份决定它权限的文件：地图和私人名单。它能看地图，但两份都不能由它来改
    if act != Act::See && (p == real(&file(root)) || p == real(&private_file(root))) {
        return (false, "这是决定你权限的文件，不能由你来改。".into());
    }
    if act == Act::See && p == real(&private_file(root)) {
        return (false, "这份名单不给你看。".into());
    }
    if act == Act::See && granted(root, &p) {
        return (true, "这是他亲手交来给你看的，这一件可以看".into());
    }
    // 它自己的记忆：读写随意，但不在里面运行东西
    if inside(&p, &real(&root.memory())) {
        return (act != Act::Run, "这是它自己的记忆".into());
    }
    match find(places, &p) {
        Some(pl) if pl.allows(act) => (true, format!("地图上写着 {} 可以{}（{}）", pl.path, act.word(), pl.source)),
        Some(pl) => (false, format!("地图上写着 {} 不能{}（{}）。要做，先问机主；机主同意后把地图改了再做。", pl.path, act.word(), pl.source)),
        // 地图上没写的地方：只知道有，能看不能动
        None if act == Act::See => (true, "地图上没写这个地方，默认能看".into()),
        None => (false, format!("地图上没写这个地方，默认只能看，不能{}。先问机主这里能不能动，问清楚后往地图里添一行。", act.word())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAP: &str = "# 家的地图\n\n| 地方 | 是什么 | 看 | 放 | 改 | 扔 | 运行 | 依据 |\n|---|---|---|---|---|---|---|---|\n| `D:\\SOUL` | 它自己 | ✓ | ✓ | ✓ | ✓ | ✓ | 你说的 2026-10-11 |\n| `D:\\personal` | 私人的 | – | – | – | – | – | 你说的 2026-10-11 |\n| `D:\\tools` | 小工具 | ✓ | – | – | – | – | 它猜的 |\n| `D:\\tools\\dns` | 抓包脚本 | ✓ | ✓ | ✓ | – | ✓ | 你说的 2026-10-12 |\n";

    fn root() -> (tempfile::TempDir, Root) {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("memory")).unwrap();
        std::fs::create_dir_all(d.path().join("seed")).unwrap();
        let r = Root::at(d.path()).unwrap();
        (d, r)
    }

    #[test]
    fn parse_reads_rows_and_skips_the_header() {
        let ps = parse(MAP);
        assert_eq!(ps.len(), 4);
        assert_eq!(ps[0].path, "D:\\SOUL");
        assert!(ps[0].run && !ps[1].see && ps[2].see && !ps[2].edit);
    }

    #[test]
    fn find_takes_the_most_specific_row() {
        let ps = parse(MAP);
        assert_eq!(find(&ps, "D:/tools/dns/run.ps1").unwrap().what, "抓包脚本");
        assert_eq!(find(&ps, "d:\\TOOLS\\notepad++\\x.exe").unwrap().what, "小工具");
        assert!(find(&ps, "D:\\toolshed\\x").is_none(), "名字只是开头相同的目录不算在里面");
    }

    #[test]
    fn a_private_place_cannot_even_be_seen() {
        let (_d, r) = root();
        let (ok, why) = allowed(&parse(MAP), &r, Path::new("D:\\personal\\a.md"), Act::See);
        assert!(!ok && why.contains("不能看"), "{why}");
    }

    #[test]
    fn an_unmapped_place_can_be_seen_but_not_touched() {
        let (_d, r) = root();
        let ps = parse(MAP);
        assert!(allowed(&ps, &r, Path::new("C:\\Windows\\win.ini"), Act::See).0);
        let (ok, why) = allowed(&ps, &r, Path::new("C:\\Windows\\win.ini"), Act::Edit);
        assert!(!ok && why.contains("先问机主"), "{why}");
    }

    #[test]
    fn its_own_memory_is_always_writable() {
        let (_d, r) = root();
        let note = r.memory().join("us").join("x.md");
        assert!(allowed(&[], &r, &note, Act::Edit).0);
        assert!(!allowed(&[], &r, &note, Act::Run).0);
    }

    #[test]
    fn the_private_list_is_enforced_without_being_on_the_map() {
        let (d, r) = root();
        std::fs::create_dir_all(d.path().join("sensors").join("census")).unwrap();
        std::fs::write(d.path().join("sensors").join("census").join("shallow-dirs.txt"), "# 说明\nD:/secret\n\n").unwrap();
        let all = load(&r);
        assert_eq!(all.len(), 1);
        assert!(!allowed(&all, &r, Path::new("D:/secret/a.txt"), Act::See).0);
    }

    #[test]
    fn upsert_adds_a_row_replaces_an_old_one_and_remove_restores_it() {
        let (_d, r) = root();
        std::fs::create_dir_all(r.memory().join("us")).unwrap();
        std::fs::write(file(&r), MAP).unwrap();
        let mut p = Place { path: "D:/tools".into(), what: "小工具".into(), see: true, put: true, edit: false, drop: false, run: false, source: "你说的 2026-10-12".into() };
        let old = upsert(&r, &p).unwrap();
        assert!(old.as_deref().unwrap().contains("它猜的"));
        let order: Vec<String> = parse(&std::fs::read_to_string(file(&r)).unwrap()).into_iter().map(|p| p.path).collect();
        assert_eq!(order[2], "D:/tools", "改过的那一行留在原处");
        let now = parse(&std::fs::read_to_string(file(&r)).unwrap());
        assert_eq!(now.len(), 4);
        assert!(find(&now, "D:/tools/x").unwrap().put);
        remove(&r, "D:/tools", old.as_deref()).unwrap();
        assert!(!find(&parse(&std::fs::read_to_string(file(&r)).unwrap()), "D:/tools/x").unwrap().put);
        p.path = "E:/new".into();
        assert!(upsert(&r, &p).unwrap().is_none());
        remove(&r, "E:/new", None).unwrap();
        assert!(find(&parse(&std::fs::read_to_string(file(&r)).unwrap()), "E:/new/x").is_none());
    }

    #[test]
    fn other_spellings_of_a_path_do_not_get_around_the_map() {
        let (_d, r) = root();
        let ps = parse(MAP);
        for sneaky in [
            "D:\\tools\\..\\personal\\a.md",
            "D:/tools/dns/../../personal/a.md",
            "D:\\personal.\\a.md",
            "D:\\personal \\a.md",
            "\\\\?\\D:\\personal\\a.md",
            "D:\\PERSONAL\\a.md",
            "D:\\.\\personal\\a.md:stream",
        ] {
            assert!(!allowed(&ps, &r, Path::new(sneaky), Act::See).0, "{sneaky} 应该被拦下");
        }
        let escape = r.memory().join("..").join("..").join("x.txt");
        assert!(!allowed(&ps, &r, &escape, Act::Edit).0, "从记忆里用 .. 走出去不算在记忆里");
    }

    #[test]
    fn network_device_and_drive_relative_paths_are_refused() {
        let (_d, r) = root();
        for odd in ["\\\\localhost\\D$\\personal\\a.md", "\\\\server\\share\\x", "D:personal\\a.md", "personal\\a.md", "\\\\.\\PhysicalDrive0"] {
            assert!(!allowed(&[], &r, Path::new(odd), Act::See).0, "{odd} 应该被拒");
        }
    }

    #[test]
    fn the_two_files_that_decide_its_rights_cannot_be_changed_by_it() {
        let (d, r) = root();
        std::fs::create_dir_all(d.path().join("sensors").join("census")).unwrap();
        std::fs::write(private_file(&r), "D:/secret\n").unwrap();
        // 哪怕地图把整个项目都放开了
        let open = vec![Place { path: d.path().to_string_lossy().to_string(), what: "项目".into(), see: true, put: true, edit: true, drop: true, run: true, source: "测试".into() }];
        assert!(!allowed(&open, &r, &private_file(&r), Act::Edit).0);
        assert!(!allowed(&open, &r, &private_file(&r), Act::See).0);
        assert!(!allowed(&open, &r, &file(&r), Act::Edit).0);
        assert!(allowed(&open, &r, &file(&r), Act::See).0);
    }

    #[test]
    fn a_folder_that_contains_a_hidden_place_is_noticed() {
        let ps = parse(MAP);
        assert!(hides_something(&ps, Path::new("D:\\")));
        assert!(!hides_something(&ps, Path::new("D:\\tools")));
        assert!(!hides_something(&ps, Path::new("D:\\personal")), "它自己就是那个地方时，由看不看得了来管");
    }

    #[test]
    fn what_the_owner_hands_over_can_be_seen_but_only_that_and_only_seen() {
        let (d, r) = root();
        let private = d.path().join("private");
        std::fs::create_dir_all(private.join("sub")).unwrap();
        std::fs::write(private.join("a.png"), "x").unwrap();
        std::fs::write(private.join("b.png"), "x").unwrap();
        let map = vec![Place { path: private.to_string_lossy().to_string(), what: "私人的".into(), see: false, put: false, edit: false, drop: false, run: false, source: "测试".into() }];
        assert!(!allowed(&map, &r, &private.join("a.png"), Act::See).0);
        let said = format!("看看「{}」这张图", private.join("a.png").display());
        assert_eq!(grant_from_words(&r, &said).len(), 1);
        assert!(allowed(&map, &r, &private.join("a.png"), Act::See).0, "他点名的那一张可以看");
        assert!(!allowed(&map, &r, &private.join("b.png"), Act::See).0, "旁边的不行");
        assert!(!allowed(&map, &r, &private.join("a.png"), Act::Edit).0, "只放开看");
        assert!(grant_from_words(&r, "看看 Z:\\没有这个\\文件.txt").is_empty(), "不存在的路径不算");
        assert_eq!(grant_from_words(&r, &format!("{} 里有什么", private.join("sub").display())).len(), 1, "不带括号、盘符开头的也认");
    }

    #[test]
    fn a_mapped_place_follows_its_row() {
        let (_d, r) = root();
        let ps = parse(MAP);
        assert!(allowed(&ps, &r, Path::new("D:\\SOUL\\app\\ui\\app.js"), Act::Edit).0);
        assert!(!allowed(&ps, &r, Path::new("D:\\tools\\x.exe"), Act::Run).0);
        assert!(allowed(&ps, &r, Path::new("D:\\tools\\dns\\run.ps1"), Act::Run).0);
    }
}
