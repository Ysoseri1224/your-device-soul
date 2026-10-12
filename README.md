<div align="center">

<img src="assets/hero.png" alt="A black cat sitting on top of a window" width="560">

# Your Device Soul

**Give your computer a soul.** A quiet black cat that lives on your desktop, sees what you're doing on this machine, remembers your days, and acts for you — only after you nod.

**[English](README.md) | [中文](README.zh-CN.md)**

[![Stars](https://img.shields.io/github/stars/Ysoseri1224/your-device-soul?style=flat)](https://github.com/Ysoseri1224/your-device-soul/stargazers)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Build](https://github.com/Ysoseri1224/your-device-soul/actions/workflows/ci.yml/badge.svg)](https://github.com/Ysoseri1224/your-device-soul/actions/workflows/ci.yml)
[![Last commit](https://img.shields.io/github/last-commit/Ysoseri1224/your-device-soul)](https://github.com/Ysoseri1224/your-device-soul/commits/main)

</div>

It is not a floating chat box. It is a cat that treats your windows as a cat tree, a crate it sleeps in, and a few small things scattered on your screen: tickets, paw prints, sticky tags, circles and margin notes.

<div align="center">
<img src="assets/ticket.png" alt="A ticket waiting for approval above the chat panel" width="48%"> <img src="assets/marks.png" alt="A tag clipped on a window, a circle and a margin note" width="48%">
</div>

## Quick Start

Windows 10/11, PowerShell 7. Grab the zip from [Releases](https://github.com/Ysoseri1224/your-device-soul/releases), unzip, then:

```powershell
.\ysoul-app.exe        # first launch opens a setup page: your name, private folders, model
.\ysoul.exe birth      # once: it surveys this machine and writes its first notes (~10 min)
.\ysoul.exe setup      # optional: right-click menu, `ys` command, Start-menu entry, AI-tool hooks
```

Building from source: `cargo build --release` in `runtime/` and in `app/src-tauri/`. The release binaries are not code-signed, so SmartScreen will ask once.

You need one working model channel. It **discovers** what is already on your machine — Claude Code, providers configured in [cc-switch](https://github.com/farion1231/cc-switch), `OPENAI_API_KEY` / `ANTHROPIC_API_KEY` style environment variables, a local Ollama — and you pick the order.

## What it does, and when

| You… | It… |
|---|---|
| stay in a window for 20 seconds | hops out of its crate and sits on that window's top edge |
| stop touching the keyboard for a minute | curls up and sleeps; goes home after five |
| type until your text reaches its feet | steps aside, or perches on a line of text that has blank space above it |
| press `Ctrl+Alt+Space` | gives you a one-line prompt at the cursor; the answer appears right there |
| select some text and ask "what does this mean?" | knows "this" is your selection |
| right-click a file → *Hand to ysoul* | can read that one file, even inside a private folder |
| say "open DeepL" | opens it — no model call needed |
| say "turn off that updater's autostart" | hands you a **ticket**: what, where, how to undo. Nothing runs until you click *Do* |
| say "search for black cats in Firefox and click the second result" | opens the page, scrolls freely, asks before it clicks — and stops asking after you've approved that app twice |
| say "have Claude Code fix the typos in this repo" | files a ticket that hands the job to that tool, in that folder, with your words verbatim |
| say "convert this with ffmpeg" | looks up which programs on this machine expose a CLI, a URL scheme or an automation interface, asks the program for `--help` if it has never used it, then calls it |
| come back after ten minutes | *might* say one line. Mostly it chooses not to. Right-click the bubble to shush it for two hours |
| walk away for half an hour | turns the day's activity into a few plain sentences in its notes |

## Why this one

- **A gate enforced by code, not by a prompt.** Every file read and write is checked against a map of your machine (see / add / edit / delete / run, per place) before it happens. Private folders can't even be listed. We tried five ways around it — `..`, odd spellings, UNC paths, searching from a parent folder, glob patterns that carry paths — and closed them.
- **Tickets instead of trust.** Anything that changes the machine becomes a ticket with an undo script. Read-only commands run directly; a command the program can't fully parse becomes a ticket rather than a guess.
- **It adapts to how you grant.** Walking around (scrolling, paging) never asks. An app you've approved twice is remembered.
- **Silence by default.** Every nudge passes hard rules before a model is even asked, and the model is told that the right answer is usually *nothing*.
- **Your notes, in plain files.** Its memory is a folder of Markdown you can open and edit. Secrets and personal identifiers are masked before anything leaves the machine, and every outgoing prompt is logged locally.
- **Bring your own model.** Three wire formats (Claude Code, Anthropic Messages, OpenAI chat/completions). For the latter two it runs its own tool loop through the same gate.

## Documentation

| | |
|---|---|
| [Guide](docs/GUIDE.md) · [中文全攻略](docs/GUIDE.zh-CN.md) | setup, every feature, every trigger, how to customise |
| [Privacy start-up](docs/PRIVACY.md) | finding your private folders; auditing before you push your own fork |
| [Mastering the machine](docs/design/12-mastery.md) | the map, the gate, tickets, the command line, UI automation *(Chinese)* |
| [Living](docs/design/13-living.md) · [Surface](docs/design/14-surface.md) | when it speaks, what is on screen and why *(Chinese)* |

**Recommended:** manage model providers with [cc-switch](https://github.com/farion1231/cc-switch). Add a provider there once (endpoint, key, model name); Your Device Soul reads it and never stores the key — only which channel you picked. Then `ysoul channel` lists what it found and `ysoul channel use <id>` sets the order.

## Roadmap

- Multiple monitors (it only knows the primary one today)
- Four more skins — the designs are in [`design/ysoul-skin-lab.html`](design/ysoul-skin-lab.html), waiting for hands
- Reading screenshots through non-Claude-Code channels
- macOS / Linux

## Contributing

Issues and PRs are welcome — especially skins. `design/` holds clickable prototypes; the cat's engine is *generated* from `design/ysoul-surface-lab.html`, so change the prototype and run `node app/tools/extract-cat.mjs`. See [CONTRIBUTING.md](CONTRIBUTING.md).

Before pushing a fork that contains your own `memory/`, run `tools/audit.ps1`.

## A fair warning

This program can read files, run commands and click inside other applications on your behalf. The gate and the tickets are there so that it does so only where and when you allow — but read [the guide's section on what it can touch](docs/GUIDE.md#what-it-can-and-cannot-touch) before you let it loose.

## License

[MIT](LICENSE)
