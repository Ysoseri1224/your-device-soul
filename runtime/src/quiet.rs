//! 启动子进程时不弹出命令行窗口。
//!
//! 桌面上的猫是一个没有控制台的程序，它每调用一次 git、pwsh 或 Claude Code，
//! Windows 默认都会闪出一个黑窗口。加上这个标志就不会了。

use std::process::Command;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// 让这条命令在后台安静地跑。
pub fn quiet(cmd: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}
