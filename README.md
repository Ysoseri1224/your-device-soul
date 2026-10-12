<div align="center">

<img src="assets/hero.png" alt="The black cat in four poses: happy, working, asleep, sitting" width="560">

# Your Device Soul

An AI assistant that lives on your Windows desktop. It knows what you are doing on this computer and remembers what you did each day. It can operate the computer for you, but every change waits for your approval.

**[English](README.md) | [中文](README.zh-CN.md)**

[![Stars](https://img.shields.io/github/stars/Ysoseri1224/your-device-soul?style=flat)](https://github.com/Ysoseri1224/your-device-soul/stargazers)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Build](https://github.com/Ysoseri1224/your-device-soul/actions/workflows/ci.yml/badge.svg)](https://github.com/Ysoseri1224/your-device-soul/actions/workflows/ci.yml)
[![Last commit](https://img.shields.io/github/last-commit/Ysoseri1224/your-device-soul)](https://github.com/Ysoseri1224/your-device-soul/commits/main)

</div>

## Why this exists

AI assistants that live in a chat window are awkward in three ways:

- **They don't know your computer.** What is installed, where your projects are — you explain it again every time.
- **They don't know what you were just doing.** After an afternoon in your editor, you switch to the chat and it knows nothing about that afternoon.
- **Letting them act is uncomfortable either way.** The ones that only chat can't do anything. The ones that control the computer often tell you what they changed after the fact.

Your Device Soul addresses these three points. After installation it takes an inventory of the computer and keeps it as plain-text lists. From then on it notes, in the background, which program and which project you are working in, and keeps a daily log. It can read files, run commands and click buttons in other programs — but you grant permissions place by place, and they are enforced by code rather than by asking the model to behave.

On screen it is a black cat that sits on the window you are using and rarely speaks first.

<div align="center">
<img src="assets/ticket.png" alt="A pending action shown as a ticket above the chat panel" width="48%"> <img src="assets/marks.png" alt="A note clipped to a window, a circled spot and a note beside some text" width="48%">
</div>

<sub>Left: an action it wants to take is listed as a "ticket" and waits for you to click <i>Do</i>. Right: marks it leaves on screen — a note clipped to a window, a circled spot, a comment beside a piece of text. Both images are UI previews, not a real desktop.</sub>

## What using it is like

A few concrete moments. They have one thing in common: you do not brief it first, and you do not carry out the steps yourself.

**You sit down in the morning and can't remember where you stopped.** Press the shortcut and ask "where did I leave off yesterday?". It checks yesterday's record and answers along the lines of: "You were in my-project all afternoon, last in form.ts; two tests were still failing." It noted that in the background yesterday. You never told it.

**An error you don't understand appears on screen.** Ask "what is this error?". It looks at your current window and explains. If the fix is a setting, it writes the change out as a confirmation slip; you approve, it makes the change, and you can undo it if it turns out wrong.

**The computer takes longer and longer to start.** Say "see what runs at startup and turn off what I don't need". It lists the startup items, tells you what each one is, and puts the ones it suggests disabling in front of you one by one. It is looking at what is actually on this computer, not reciting a general tutorial.

**You are writing a document and want a second pair of eyes.** Select a passage and say "have a look at this". It reads the selection and writes its comments next to the sentences they are about, so you are not going back and forth between a chat window and the document.

**There is a repetitive chore.** Say "sort the screenshots in Downloads into D:\pics by month". It lists which files it would move and where; once you agree, it does it.

**Something is better done by another tool.** Say "have Claude Code fix the typos in this project". It passes that sentence on to the tool, to run in the project you named — and shows you first, as with everything else.

In short, it is for two kinds of things. Questions about this computer and about what you have been doing lately, which it already keeps track of. And operations on the computer: you say what result you want, it does the work, and it asks before changing anything.

## Getting started

You need Windows 10 or 11, [PowerShell 7](https://learn.microsoft.com/powershell/scripting/install/installing-powershell-on-windows), and an AI model it can call (see "Which model" below).

Download the zip from [Releases](https://github.com/Ysoseri1224/your-device-soul/releases), unzip it, and in that folder:

```powershell
.\ysoul-app.exe     # start it. The first launch opens a setup window: your name, folders it must not read, which model
.\ysoul.exe birth   # run once, about ten minutes: inventories this computer and lets it write its first notes
```

Then click the crate in the bottom-right corner of the screen and choose *Chat*.

The binaries are not code-signed, so Windows shows a security prompt on first run. To build from source, run `cargo build --release` in both `runtime` and `app/src-tauri`.

## What it does in each situation

Those were examples. This table lists what it actually does, case by case.

| Situation | What happens |
|---|---|
| You stay in one window for 20 seconds | The cat leaves its crate and sits on that window's top edge |
| You are idle for 1 minute | The cat curls up; after 5 minutes it returns to the crate |
| Your typing reaches the spot where the cat sits | The cat moves somewhere without text |
| You press `Ctrl+Alt+Space` | An input box appears at the cursor; the answer shows up in the same place |
| You select some text and ask "what does this mean?" | It can read your selection |
| You right-click a file and choose *Hand to ysoul* | It may read that file, even inside a folder you marked as off-limits |
| You say "open DeepL" | It opens. No model call is involved |
| You say "turn off that updater's autostart" | It creates a ticket stating what, where and how to undo. Nothing runs until you click *Do* |
| You say "search for black cats in Firefox and open the second result" | Opening the page and scrolling need no approval; it asks before clicking. After you approve the same program twice, it stops asking |
| You say "have Claude Code fix the typos in this project" | It creates a ticket that hands the task to that tool |
| You come back after 10 minutes away | It may say one line, but usually doesn't. Right-click the bubble and it stays quiet for two hours |
| You are away for half an hour | It summarises your day's activity in a few sentences and saves it to its notes |

The [guide](docs/GUIDE.md) covers everything: each behaviour, what triggers it, and how to adjust it.

## How permissions work

- **A permission table.** For each location on the computer, reading, creating, editing, deleting and running are set separately. Locations not in the table are read-only.
- **A list of off-limits folders.** It cannot see inside these at all, not even file names.
- **Checks are done by code.** Every request from the model to read or write a file, or run a command, first passes through a checking routine that consults the two items above. The model cannot skip it.
- **Anything uncertain goes to you.** Read-only commands run directly. A command the program cannot fully analyse becomes a ticket.
- **There is a record.** Every executed ticket keeps its output, and most can be undone with one click.

## Which model

It does not ship with a model. On start it looks for what is already on your computer: a logged-in Claude Code, providers configured in [cc-switch](https://github.com/farion1231/cc-switch), environment variables such as `OPENAI_API_KEY` or `ANTHROPIC_API_KEY`, and a local Ollama. You choose the order in the setup window; if one fails, the next is tried.

cc-switch is the recommended way to manage this: enter the endpoint, key and model name there once and Your Device Soul reads them. It never stores the key, only which entry you chose.

Both the Anthropic and the OpenAI API formats are supported; most services speak one of them.

## Your data

- The action log, a copy of every request sent to the model, screenshots and the timestamped activity log stay on your computer, in the `local` folder.
- Before a request is sent, keys, e-mail addresses and account IDs in it are replaced with placeholders.
- It does not record the windows of chat or mail programs, does not use the microphone, and does not take screenshots continuously.

More in [section 11 of the guide](docs/GUIDE.md#11-where-your-data-goes) and in [Privacy setup](docs/PRIVACY.md).

## Current limitations

Windows only, one monitor only; the interface is in Chinese for now (the guide lists the labels); it can operate only the frontmost window; some programs (certain games, older editors) expose nothing it can operate; there is only one look, the black cat. Full list in [section 12 of the guide](docs/GUIDE.md#12-what-it-cannot-do-yet).

## Planned

- Multiple monitors
- More looks. The designs are in [`design/ysoul-skin-lab.html`](design/ysoul-skin-lab.html), which opens directly in a browser
- Screenshot reading with every model
- macOS and Linux

## Contributing

Issues and pull requests are welcome, new looks especially. The cat's drawing and animation are defined in the HTML prototypes under `design`; the code the app uses is generated from them. See [CONTRIBUTING.md](CONTRIBUTING.md).

## Before you use it

This program can read your files, run commands and click inside other programs for you. The permission system exists to keep that within what you allow, but please read [section 8 of the guide](docs/GUIDE.md#8-what-it-may-and-may-not-touch) before deciding how much to grant.

## License

[MIT](LICENSE)
