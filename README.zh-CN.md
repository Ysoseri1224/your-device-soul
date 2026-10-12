<div align="center">

<img src="assets/hero.png" alt="一只黑猫坐在窗口的上沿" width="560">

# Your Device Soul

**让你的机器有自己的灵魂。** 一只住在桌面上的黑猫：看得见你在这台机器上做什么，记得住每一天，你点头之后才替你动手，多数时候很安静。

**[English](README.md) | [中文](README.zh-CN.md)**

[![Stars](https://img.shields.io/github/stars/Ysoseri1224/your-device-soul?style=flat)](https://github.com/Ysoseri1224/your-device-soul/stargazers)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Build](https://github.com/Ysoseri1224/your-device-soul/actions/workflows/ci.yml/badge.svg)](https://github.com/Ysoseri1224/your-device-soul/actions/workflows/ci.yml)
[![Last commit](https://img.shields.io/github/last-commit/Ysoseri1224/your-device-soul)](https://github.com/Ysoseri1224/your-device-soul/commits/main)

</div>

它不是一个悬浮的聊天框。它是一只把你的窗口当猫爬架的猫、一个它睡觉的箱子，和散在屏幕上的几样小东西：票、爪印、夹在窗口上的条、圈和批注。

<div align="center">
<img src="assets/ticket.png" alt="一张等你点头的票" width="48%"> <img src="assets/marks.png" alt="夹在窗口上的条、圈和批注" width="48%">
</div>

## 三步跑起来

Windows 10/11，PowerShell 7。从 [Releases](https://github.com/Ysoseri1224/your-device-soul/releases) 下载压缩包，解开，然后：

```powershell
.\ysoul-app.exe        # 第一次启动会弹出设置页：怎么称呼你、哪些目录是私人的、用哪个模型
.\ysoul.exe birth      # 只做一次：它把这台机器摸一遍，写下第一份笔记（十来分钟）
.\ysoul.exe setup      # 可选：右键菜单、ys 短命令、开始菜单入口、AI 工具的钩子
```

自己编：在 `runtime/` 和 `app/src-tauri/` 里各跑一次 `cargo build --release`。发布的程序没有签名，SmartScreen 会问一次。

你需要一条能用的模型通道。它会**自己发现**这台机器上已有的：Claude Code、在 [cc-switch](https://github.com/farion1231/cc-switch) 里配过的供应商、`OPENAI_API_KEY` / `ANTHROPIC_API_KEY` 这类环境变量、本机的 Ollama。用哪条、按什么次序，你定。

## 它做什么，什么时候做

| 你 | 它 |
|---|---|
| 在一个窗口里待满二十秒 | 跳出箱子，蹲到那个窗口的上沿 |
| 一分钟没碰键盘 | 蜷起来睡；五分钟后回箱子 |
| 打的字到了它脚下 | 让开，或者踩到上面有空白的一行字上 |
| 按 `Ctrl+Alt+空格` | 光标旁出现一行输入，回话出现在原处 |
| 选中一段字问"这是什么意思" | 知道"这"指的是你选中的那段 |
| 右键一个文件 →"交给 ysoul" | 这一件它能看，哪怕在私人目录里 |
| 说"打开 DeepL" | 直接打开，不用过模型 |
| 说"把那个更新助手的自启动关了" | 递一张**票**：做什么、在哪、怎么撤回。你点"做"之前什么都不会发生 |
| 说"在 Firefox 里搜小黑猫，点第二个结果" | 打开网页、自己滚动，要点的时候问你；同一个程序你点过两次头，以后不再问 |
| 说"让 Claude Code 把这个仓库里的错别字改了" | 递一张票，把这件活原话交给那个工具，在那个目录里做 |
| 说"用 ffmpeg 把这个转一下" | 它查得到这台机器上哪些程序留了命令行、网址前缀、自动化接口；没用过的先问那个程序的 `--help`，再去调 |
| 离开十分钟回来 | **可能**说一句。多数时候它选择不说。右键气泡，两小时内它不再开口 |
| 走开半小时 | 把这一天理成几句话，记进笔记 |

## 为什么是它

- **把关的是程序，不是提示词。** 每一次读写文件之前，程序先对一张"家的地图"（每个地方能看、放、改、扔、运行里的哪几样）。私人目录连列都列不出来。我们自己试过五种绕法，都堵上了。
- **用票，不靠信任。** 会改变机器的事都变成一张带撤回脚本的票。只读的命令直接跑；程序看不懂的命令转成票，而不是去猜。
- **它会适应你的授权。** 走动（滚动、翻页）从不问你。你点过两次头的程序它会记住。
- **默认沉默。** 每一次想开口都先过硬条件，过了才去问模型，而且告诉模型"多数时候应该不说"。
- **记忆是一堆你打得开的文件。** 一个 Markdown 文件夹，你随时能看、能改。密钥和个人信息在出这台机器之前先遮掉；每一次发给模型的原文都在本机留底。
- **用你自己的模型。** 三种说话的格式：Claude Code、Anthropic 的 Messages、OpenAI 的 chat/completions。后两种由它自己跑工具循环，过的是同一道门。

## 文档

| | |
|---|---|
| [全攻略](docs/GUIDE.zh-CN.md) · [English Guide](docs/GUIDE.md) | 上手、每一项功能、每一个触发的时机、怎么定制 |
| [隐私上手](docs/PRIVACY.zh-CN.md) | 把私人目录找全；推自己的分支之前先审查 |
| [掌握本机](docs/design/12-mastery.md) | 地图、门、票、命令行、界面操作、模型通道 |
| [过日子](docs/design/13-living.md) · [操作面](docs/design/14-surface.md) | 它什么时候开口；屏幕上有什么、为什么 |

**推荐用 [cc-switch](https://github.com/farion1231/cc-switch) 管模型供应商。** 在它里面配一次（入口地址、密钥、模型名），Your Device Soul 自己读得到，而且不存密钥，只存"用哪一条"。之后 `ysoul channel` 列出发现的通道，`ysoul channel use <编号>` 定次序。

## 以后要做的

- 多显示器（现在只认主显示器）
- 另外四件皮肤：设计稿在 [`design/ysoul-skin-lab.html`](design/ysoul-skin-lab.html)，等人接手
- 经由非 Claude Code 的通道看截图
- macOS / Linux

## 参与

欢迎提 issue 和 PR，尤其是皮肤。`design/` 里是能点的样稿；猫的引擎是从 `design/ysoul-surface-lab.html` **生成**的，所以改样稿，再运行 `node app/tools/extract-cat.mjs`。见 [CONTRIBUTING.md](CONTRIBUTING.md)。

推一个带着你自己 `memory/` 的分支之前，先跑一遍 `tools/audit.ps1`。

## 先说清楚

这个程序能替你读文件、跑命令、在别的程序里点东西。那道门和那些票，就是为了让它只在你允许的地方、你允许的时候这么做。放它出去之前，先读一下[全攻略里"它能动什么、不能动什么"那一节](docs/GUIDE.zh-CN.md#六它能动什么不能动什么)。

## 许可证

[MIT](LICENSE)
