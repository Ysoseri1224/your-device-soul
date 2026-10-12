<div align="center">

<img src="assets/hero.png" alt="黑猫的几种姿势：高兴、干活、睡觉、坐着" width="560">

# Your Device Soul

一个常驻在 Windows 桌面上的 AI 助手。它知道你正在这台电脑上做什么，记得你每天做过的事；它能替你操作电脑，但每一次改动都要你先点同意。

**[English](README.md) | [中文](README.zh-CN.md)**

[![Stars](https://img.shields.io/github/stars/Ysoseri1224/your-device-soul?style=flat)](https://github.com/Ysoseri1224/your-device-soul/stargazers)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Build](https://github.com/Ysoseri1224/your-device-soul/actions/workflows/ci.yml/badge.svg)](https://github.com/Ysoseri1224/your-device-soul/actions/workflows/ci.yml)
[![Last commit](https://img.shields.io/github/last-commit/Ysoseri1224/your-device-soul)](https://github.com/Ysoseri1224/your-device-soul/commits/main)

</div>

## 为什么做这个

聊天窗口里的 AI 助手有三个地方用着不顺手：

- **它不认识你的电脑。** 装了什么软件、项目放在哪，每次都要你重新讲一遍。
- **它不知道你刚才在干什么。** 你改了一下午代码，切过去提问时，它对这一下午一无所知。
- **让它动手，两头都不放心。** 只会聊天的什么都做不了；能操作电脑的，又往往是做完了你才知道它动了什么。

Your Device Soul 针对这三点：安装后它先把电脑清点一遍并存成清单；之后一直在后台留意你用的是哪个程序、哪个项目，记成每天的日记；它可以读文件、跑命令、在别的程序里点按钮，但权限由你逐处授予，并且由程序强制执行，而不是靠嘱咐模型"别乱动"。

它在屏幕上的样子是一只黑猫，平时坐在你正在用的窗口上，很少主动说话。

<div align="center">
<img src="assets/ticket.png" alt="一张等待确认的操作单，显示在对话面板上方" width="48%"> <img src="assets/marks.png" alt="贴在窗口上的纸条、圈出的位置和文字旁的批注" width="48%">
</div>

<sub>左：它要执行的操作会先以一张"票"的形式列出来，等你点"做"。右：它在屏幕上留的标记，包括贴在窗口上的纸条、圈出的位置、写在文字旁的批注。两张图都是界面预览，背景不是真实桌面。</sub>

## 开始使用

需要 Windows 10 或 11、[PowerShell 7](https://learn.microsoft.com/powershell/scripting/install/installing-powershell-on-windows)，以及一个可用的 AI 模型（见下方"用哪个模型"）。

从 [Releases](https://github.com/Ysoseri1224/your-device-soul/releases) 下载压缩包并解压，然后在解压目录里：

```powershell
.\ysoul-app.exe     # 启动。第一次会弹出设置窗口：称呼、哪些文件夹不许它看、用哪个模型
.\ysoul.exe birth   # 只需执行一次，约十分钟：清点这台电脑，让它写下第一批笔记
```

之后点一下屏幕右下角的箱子，选"聊聊"。

程序没有数字签名，第一次运行时 Windows 会弹出安全提示。想自己编译的话，在 `runtime` 和 `app/src-tauri` 两个目录里各执行一次 `cargo build --release`。

## 它能做什么

| 场景 | 它的行为 |
|---|---|
| 你在一个窗口里停留 20 秒 | 猫从箱子里出来，坐到那个窗口的上边缘 |
| 你 1 分钟没有操作 | 猫蜷起来睡觉；5 分钟后回箱子 |
| 你输入的文字延伸到猫所在的位置 | 猫移到没有文字的地方 |
| 按 `Ctrl+Alt+空格` | 光标旁出现输入框；回答显示在原处 |
| 选中一段文字，问"这是什么意思" | 它能读到你选中的内容 |
| 右键一个文件，选"交给 ysoul" | 它可以读取这个文件，即使它在你设为不许看的文件夹里 |
| 说"打开 DeepL" | 直接打开，这一步不调用模型 |
| 说"把那个更新程序的开机启动关掉" | 生成一张票，写明要做什么、在哪、如何撤销。你点"做"才执行 |
| 说"在 Firefox 里搜索小黑猫，点开第二个结果" | 打开搜索页、滚动页面不需要确认；点击链接前会问你。同一个程序里你同意过两次后，不再询问 |
| 说"让 Claude Code 把这个项目里的错别字改掉" | 生成一张票，把这项任务转交给那个工具 |
| 你离开 10 分钟后回来 | 它可能说一句话，但大多数时候不说。在气泡上点右键，它两小时内不再开口 |
| 你离开半小时 | 它把你今天的活动整理成几句话，存进笔记 |

完整的说明、每个行为的触发条件、如何调整，见[使用指南](docs/GUIDE.zh-CN.md)。

## 权限是怎么管的

- **有一张权限表。** 电脑上每个位置，它能不能读、新建、修改、删除、运行，分开设置。表里没有的位置默认只读。
- **有一份"不许看"的文件夹名单。** 名单里的文件夹，它连里面有哪些文件都看不到。
- **检查由程序完成。** 模型每次要读写文件或运行命令，请求都先经过一段检查代码，对照上面两样决定放行还是拒绝。模型无法绕过。
- **无法确认安全的操作交给你决定。** 只读的命令直接运行；程序无法确定会访问哪里的命令，一律生成票。
- **做过的事有记录。** 每张执行过的票都留有结果，多数可以一键撤销。

## 用哪个模型

它自己不带模型。启动时会在电脑上寻找已有的：登录好的 Claude Code、在 [cc-switch](https://github.com/farion1231/cc-switch) 里配置过的供应商、`OPENAI_API_KEY` 或 `ANTHROPIC_API_KEY` 这类环境变量、本机运行的 Ollama。你在设置窗口里选择使用顺序，前一个调用失败会自动换下一个。

推荐用 cc-switch 管理：在它里面填一次接口地址、密钥和模型名称，Your Device Soul 就能读到。它不保存密钥，只保存你选了哪一个。

支持 Anthropic 和 OpenAI 两种接口格式，大多数服务兼容其中一种。

## 你的数据

- 操作记录、发给模型的每次请求的副本、截图、按时间记录的活动日志，都只存在本机的 `local` 文件夹。
- 发给模型的内容在发送前会把密钥、邮箱、账号 ID 替换成占位符。
- 不记录聊天软件和邮件客户端的窗口内容，不使用麦克风，不持续截屏。

详见[使用指南的第 11 节](docs/GUIDE.zh-CN.md#11-你的数据去了哪里)和[隐私设置](docs/PRIVACY.zh-CN.md)。

## 目前的限制

只支持 Windows 和单个显示器；只能操作最前面的窗口；有些程序（部分游戏、老式编辑器）不提供可操作的接口；只有黑猫一种外观。完整列表见[指南第 12 节](docs/GUIDE.zh-CN.md#12-目前做不到的)。

## 计划中

- 多显示器支持
- 更多外观。设计稿在 [`design/ysoul-skin-lab.html`](design/ysoul-skin-lab.html)，可以直接在浏览器里打开
- 让截图功能支持所有模型
- macOS 和 Linux

## 参与

欢迎提 issue 和 PR，尤其是新的外观。猫的图形和动画定义在 `design` 里的网页设计稿中，程序使用的代码由设计稿生成，具体做法见 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 使用前请知道

这个程序可以读取你的文件、运行命令、在其他程序里替你点击。上面的权限机制是为了把这些限制在你允许的范围内，但请先读一遍[指南第 8 节](docs/GUIDE.zh-CN.md#8-它能碰什么不能碰什么)，再决定给它多大的权限。

## 许可证

[MIT](LICENSE)
