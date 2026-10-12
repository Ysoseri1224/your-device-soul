//! 它留在屏幕上的记号：夹在窗口上的条、圈、页边批注。
//!
//! 记号是它在对话里用工具留下的，由桌面上那只画出来。都会自己消失，机主点一下也会消失。
//! 圈和批注的位置来自"看一眼"截的那张图：它说的是图上的位置，这里换算回屏幕上的位置。

use crate::paths::Root;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Mark {
    pub id: String,
    /// tag 夹在窗口上的条 / ring 圈 / note 页边批注
    pub kind: String,
    /// 条夹在哪个程序的窗口上（程序名）。圈和批注不用
    pub app: String,
    pub text: String,
    /// 屏幕上的位置：左、上、右、下，真实像素。条不用
    pub rect: (i32, i32, i32, i32),
    /// 留下的时间，Unix 秒
    pub at: i64,
    /// 批注跟着走的那段字。有它，桌面上那只会不断去问这段字现在在屏幕上的哪里
    #[serde(default)]
    pub anchor: String,
}

/// 一个记号留多久（秒）。条是留言，留半小时；圈和批注是"你看这里"，看完就该没了。
pub fn life(kind: &str) -> i64 {
    if kind == "tag" {
        1800
    } else {
        45
    }
}

fn file(root: &Root) -> PathBuf {
    root.local().join("marks.json")
}

fn read(root: &Root) -> Vec<Mark> {
    std::fs::read_to_string(file(root)).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

fn write(root: &Root, marks: &[Mark]) -> Result<()> {
    std::fs::create_dir_all(root.local())?;
    std::fs::write(file(root), serde_json::to_string(marks)?)?;
    Ok(())
}

/// 还没过期的记号。
pub fn list(root: &Root, now: i64) -> Vec<Mark> {
    // 跟着字走的批注留十分钟：它不会因为滚动而指错地方，可以多留一会儿
    read(root).into_iter().filter(|m| now - m.at < if m.anchor.is_empty() { life(&m.kind) } else { 600 }).collect()
}

/// 让一条批注跟着一段字走。
pub fn follow(root: &Root, id: &str, anchor: &str) -> Result<()> {
    let mut all = read(root);
    if let Some(m) = all.iter_mut().find(|m| m.id == id) {
        m.anchor = anchor.to_string();
    }
    write(root, &all)
}

/// 那段字挪了地方（或者滚出去了，那就是全零）。
pub fn moved(root: &Root, id: &str, rect: (i32, i32, i32, i32)) -> Result<()> {
    let mut all = read(root);
    match all.iter_mut().find(|m| m.id == id) {
        Some(m) if m.rect != rect => m.rect = rect,
        _ => return Ok(()),
    }
    write(root, &all)
}

/// 留一个记号。字数有上限：记号是一眼能看完的东西，长话该在说话的地方说。同时最多留六个，多了挤掉最早的。
pub fn add(root: &Root, now: i64, kind: &str, app: &str, text: &str, rect: (i32, i32, i32, i32)) -> Result<Mark> {
    if !["tag", "ring", "note"].contains(&kind) {
        bail!("不认识的记号：{kind}");
    }
    let text = text.trim();
    if text.is_empty() && kind != "ring" {
        bail!("要写上一句话");
    }
    if text.chars().count() > 40 {
        bail!("太长了。记号上最多四十个字，长话在回话里说");
    }
    if kind != "tag" && (rect.2 - rect.0 < 4 || rect.3 - rect.1 < 4) {
        bail!("这块地方太小，圈不出来");
    }
    let mut all = list(root, now);
    let mark = Mark { id: format!("m{now}-{}", all.len()), kind: kind.into(), app: app.into(), text: text.into(), rect, at: now, anchor: String::new() };
    all.push(mark.clone());
    let skip = all.len().saturating_sub(6);
    write(root, &all[skip..])?;
    Ok(mark)
}

/// 机主点掉了一个记号。
pub fn clear(root: &Root, id: &str) -> Result<()> {
    let all: Vec<Mark> = read(root).into_iter().filter(|m| m.id != id).collect();
    write(root, &all)
}

/// "看一眼"截的那张图对应屏幕上的哪一块：图的左上角在屏幕的哪里，图缩小了多少。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Frame {
    pub left: i32,
    pub top: i32,
    pub scale: f64,
}

fn frame_file(root: &Root) -> PathBuf {
    root.local().join("tmp").join("look.json")
}

pub fn save_frame(root: &Root, frame: Frame) -> Result<()> {
    std::fs::create_dir_all(root.local().join("tmp"))?;
    std::fs::write(frame_file(root), serde_json::to_string(&frame)?)?;
    Ok(())
}

pub fn load_frame(root: &Root) -> Option<Frame> {
    std::fs::read_to_string(frame_file(root)).ok().and_then(|s| serde_json::from_str(&s).ok())
}

pub fn forget_frame(root: &Root) {
    let _ = std::fs::remove_file(frame_file(root));
}

/// 图上的一块（左、上、宽、高，图的像素）换算成屏幕上的一块。
pub fn to_screen(frame: Frame, x: f64, y: f64, w: f64, h: f64) -> (i32, i32, i32, i32) {
    let s = if frame.scale > 0.0 { frame.scale } else { 1.0 };
    let px = |v: f64| (v / s).round() as i32;
    (frame.left + px(x), frame.top + px(y), frame.left + px(x + w), frame.top + px(y + h))
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
    fn a_tag_outlives_a_ring() {
        let (_d, r) = root();
        add(&r, 1000, "tag", "code", "测试还红着两个", (0, 0, 0, 0)).unwrap();
        add(&r, 1000, "ring", "", "", (10, 10, 200, 60)).unwrap();
        assert_eq!(list(&r, 1030).len(), 2);
        let later = list(&r, 1100);
        assert_eq!(later.len(), 1);
        assert_eq!(later[0].kind, "tag");
        assert!(list(&r, 1000 + 1800).is_empty());
    }

    #[test]
    fn marks_are_short_and_few() {
        let (_d, r) = root();
        assert!(add(&r, 1, "tag", "code", &"长".repeat(41), (0, 0, 0, 0)).is_err());
        assert!(add(&r, 1, "tag", "code", "  ", (0, 0, 0, 0)).is_err());
        assert!(add(&r, 1, "note", "", "这里", (5, 5, 6, 6)).is_err(), "太小的一块圈不出来");
        assert!(add(&r, 1, "arrow", "", "x", (0, 0, 50, 50)).is_err());
        for i in 0..8 {
            add(&r, 10 + i, "tag", "code", &format!("第 {i} 条"), (0, 0, 0, 0)).unwrap();
        }
        let all = list(&r, 20);
        assert_eq!(all.len(), 6);
        assert_eq!(all[0].text, "第 2 条");
    }

    #[test]
    fn a_note_that_follows_text_lives_longer_and_can_move() {
        let (_d, r) = root();
        let m = add(&r, 1000, "note", "", "这句重复了", (10, 10, 300, 40)).unwrap();
        follow(&r, &m.id, "重复的那句话").unwrap();
        assert_eq!(list(&r, 1100).len(), 1, "不跟着字走的批注四十五秒就没了，跟着走的还在");
        moved(&r, &m.id, (10, 200, 300, 230)).unwrap();
        assert_eq!(list(&r, 1100)[0].rect, (10, 200, 300, 230));
        assert!(list(&r, 1000 + 600).is_empty());
    }

    #[test]
    fn clearing_removes_only_that_mark() {
        let (_d, r) = root();
        let a = add(&r, 1, "tag", "code", "一", (0, 0, 0, 0)).unwrap();
        add(&r, 2, "tag", "code", "二", (0, 0, 0, 0)).unwrap();
        clear(&r, &a.id).unwrap();
        let all = list(&r, 3);
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].text, "二");
    }

    #[test]
    fn a_spot_on_the_picture_maps_back_to_the_screen() {
        // 窗口左上角在屏幕 (300, 120)，图缩到了原来的一半
        let f = Frame { left: 300, top: 120, scale: 0.5 };
        assert_eq!(to_screen(f, 100.0, 50.0, 200.0, 40.0), (500, 220, 900, 300));
        let same = Frame { left: 0, top: 0, scale: 1.0 };
        assert_eq!(to_screen(same, 10.0, 20.0, 30.0, 40.0), (10, 20, 40, 60));
    }

    #[test]
    fn the_frame_is_remembered_until_forgotten() {
        let (_d, r) = root();
        assert!(load_frame(&r).is_none());
        save_frame(&r, Frame { left: 5, top: 6, scale: 0.8 }).unwrap();
        assert_eq!(load_frame(&r).unwrap().left, 5);
        forget_frame(&r);
        assert!(load_frame(&r).is_none());
    }
}
