# Your Device Soul — User Guide

Your Device Soul (the program is called `ysoul`) runs in the background on a Windows desktop. It appears on screen as a black cat. It knows which program and which project you are working in, and writes down what you did on this computer each day. When you ask it to do something it can read files, run commands and click buttons in other programs for you — but anything that changes the computer waits for you to approve it.

This guide assumes you have never heard of it. Sections 1–3 explain why it exists, what you will see, and what a few terms mean. The rest covers installation, daily use, permissions, models and customisation. The Chinese version ([GUIDE.zh-CN.md](GUIDE.zh-CN.md)) is the original and is updated first.

## Contents

1. [Why this exists](#1-why-this-exists)
2. [What you will see on screen](#2-what-you-will-see-on-screen)
3. [A few terms](#3-a-few-terms)
4. [Installing and first launch](#4-installing-and-first-launch)
5. [What it does on its own](#5-what-it-does-on-its-own)
6. [Talking to it](#6-talking-to-it)
7. [Getting it to do things](#7-getting-it-to-do-things)
8. [What it may and may not touch](#8-what-it-may-and-may-not-touch)
9. [Choosing a model](#9-choosing-a-model)
10. [Adjusting it](#10-adjusting-it)
11. [Where your data goes](#11-where-your-data-goes)
12. [What it cannot do yet](#12-what-it-cannot-do-yet)
13. [What is in the repository](#13-what-is-in-the-repository)
14. [References](#14-references)

---

## 1. Why this exists

Most AI assistants live in a chat window. Three things about that are awkward.

First, they don't know your computer. You explain the situation every time: what is installed, where the project is, what error just appeared. Next time you explain it again.

Second, they don't know what you are doing right now. After an afternoon of editing code, you switch to the chat window and the assistant knows nothing about that afternoon.

Third, when it comes to acting, neither option is comfortable. An assistant that only chats cannot do anything; to change a system setting you follow its steps by hand. The kind that controls the computer is often simply let loose, and you learn what it deleted or changed afterwards.

ysoul is an attempt at these three problems:

- After installation it inventories the computer — installed software, projects, what starts at boot — and saves the result as lists it can consult later.
- It keeps track, in the background, of which program and project are in front of you, and turns the day's activity into a log. When you ask "where did I leave off yesterday?", it has something to look up.
- It can act, but you grant permission place by place and the program enforces it. Before any change to the computer it writes down what it will do, where, and how to undo it, and runs it only after you approve.

One more requirement: it must not be annoying. Most of the time it is a cat in the corner of the screen. It opens no pop-ups, does not take the keyboard, and rarely speaks first.

### What it is for

Think of it as a partner that lives in this computer.

It knows the computer. What is installed, where each project is, what starts at boot, which drive is nearly full — it has taken stock of all this and keeps it up to date. When you ask about the computer, it checks the actual state before answering.

It knows what you are doing, and is gradually getting to know you. Which projects you have been spending time on, where you stopped yesterday, how you like things done, what you have corrected it on — it keeps all of that. So you can talk to it without preamble.

It can operate the computer for you. Whatever you could do at this computer with the keyboard, the mouse and the command line can, in principle, be handed to it: looking things up, changing settings, organising files, operating a program, passing a job to another tool. You say what result you want and it decides how; any step that changes the computer is shown to you first.

For that reason it has no fixed list of features. What it can do depends on what is on this computer and how much permission you give it. The sections below describe how it behaves and how permissions are managed; the things it can do are not limited to those mentioned here.


## 2. What you will see on screen

While running, ysoul covers the whole screen with a transparent window. The items below are drawn on it. Everywhere else, mouse clicks pass straight through to the program behind, so it does not get in the way.

**The crate.** A wooden box in the bottom-right corner; the cat's home. Click it to open a small panel showing what it thinks you are doing, memory usage, today's activity, and a few buttons. You can drag the crate elsewhere.

**The cat.** When you stay in a window for more than twenty seconds, the cat leaves the crate and sits on that window's title bar; if the window fills the screen, it sits on the taskbar. It treats the top edges of windows as ledges and walks and jumps between them. While the cat is out, the crate slides to the edge of the screen. You can pick the cat up with the mouse and put it somewhere else, or drag it back into the crate.

**The cat's pose shows its state:**

| Pose | Meaning |
|---|---|
| Sitting | Nothing going on |
| Curled up | You have been idle for over a minute, or you turned on "Stay quiet for a while" |
| Holding a sign | A ticket is waiting for you ("ticket" is explained in the next section) |
| Playing with a ball of yarn | An action you approved is running |
| Stars in its eyes and above its head | The action succeeded |
| Three exclamation marks | The action failed |
| Fur standing on end | Memory usage has stayed above 93% |
| Knocking a cup over | It disagrees with what you asked (it says why; if you insist, it does it) |

**Other things that appear:**

- **Now strip** — a line above the crate, e.g. "You're in: VS Code · my-project". This is its guess at what you are doing; if it is wrong you can tell at a glance.
- **Bubble** — one line it says unprompted; disappears after a few seconds.
- **Ticket** — a card describing one action it intends to take, waiting for *Do* or *Don't*.
- **Paw prints** — a row beside the crate, one per ticket executed today. Click one to see the result or undo it.
- **Note clipped to a window** — a line about a particular window, attached to its top edge; gone after half an hour.
- **Circles and comments** — when you ask "which button?" or "where is the mistake?", it circles the spot on screen or writes a line beside the text.

> **The interface is in Chinese only for now.** This guide uses English names; the labels you will actually see are: 聊聊 (*Chat*), 做 (*Do*), 不做 (*Don't*), 撤回 (*Undo*), 这个程序以后都行 (*Always for this program*), 家的地图 (*Map*), 设置 (*Settings*), 叫它回箱子 (*Send it home*), 这阵子别出声 (*Stay quiet for a while*), 就这样 (*That's it*), 重新找 (*Search again*), 跑腿 (*Errands*). It also answers in Chinese unless you edit `prompts/system.md`.

## 3. A few terms

**Ticket.** A confirmation slip ysoul creates before any action that changes the computer. It has four parts: what will be done (one sentence), where (which folder, registry key or service), whether it can be undone, and the actual script (expandable). It runs only when you click *Do*; afterwards the ticket keeps the output and an *Undo* button. *Don't* discards it.

**The map.** A permission table recording what ysoul may do at each location on this computer. Each location has five permissions: see (read contents), add (create files), edit (change existing files), delete, and run (execute programs there). Locations not listed are read-only by default. The table is an ordinary text file you can read and edit.

**Private folders.** Folders you designate. ysoul cannot read anything inside them — it cannot even see which files exist. The map decides whether it may *change* things; the private list decides whether it may *look*. One exception: a file you hand to it yourself (section 7).

**Memory.** The `memory` folder: Markdown notes ysoul writes and maintains — the inventory of this computer, things you told it, daily summaries, questions it wants to ask you. It carries the most important few into every thought and looks up the rest when needed. They are plain text; open and edit them with any editor.

**Channel.** ysoul contains no AI model; it calls one to think. A channel is one available way to do that — a logged-in Claude Code, an API you configured, a local Ollama. It finds the channels already on your computer and you decide which to use.

**Birth.** A one-time initialisation you start from the setup window after installing. It inventories the computer, then has ysoul read that inventory and write its first description of the computer and of itself. Until you run it, ysoul knows nothing about this machine.

## 4. Installing and first launch

**Requirements:** Windows 10 or 11; PowerShell 7 (Windows ships with 5.1; install 7 separately); at least one working channel (section 9).

**Installing:** download `YourDeviceSoul-…-setup.exe` from the [Releases page](https://github.com/Ysoseri1224/your-device-soul/releases/latest) and run it. It installs for the current user and does not need administrator rights. The installer is not code-signed, so Windows shows "Windows protected your PC"; click *More info*, then *Run anyway*.

After installing there are two things on disk:

- The program, in `%LOCALAPPDATA%\Your Device Soul`. `ysoul-app.exe` is the cat on your desktop; `ysoul.exe` is the command-line tool that does the thinking, the permission checks and the actions. The cat calls it behind the scenes.
- Its data, in a `YourDeviceSoul` folder in your user folder (for example `C:\Users\you\YourDeviceSoul`). Its notes, your settings and the record of every ticket are here. The `memory`, `local`, `prompts` and other folders mentioned later in this guide are inside it. Uninstalling or upgrading the program leaves this folder alone.

**Step 1: fill in the settings.** The program starts when the installer finishes and opens a setup window with three things to fill in:

1. **Name** — what it should call you. Used only for addressing you.
2. **Private folders** — the window lists the top-level folders on each drive; click one to add it, or type paths yourself, one per line.
3. **Model** — the window lists the channels it found. Click the circle on the left to set the order (the first is used; if a call fails it moves to the next). The *Errands* button picks the channel for small jobs; choose a cheap one.

Click *That's it* (就这样) to save. To reopen it later, right-click the cat icon in the system tray and choose *Settings*.

**Step 2: initialise.** After saving, a last section appears at the bottom of the window ("最后一步：让它认识这台电脑"); click *Start* (开始). A terminal window opens and shows progress. It takes about ten minutes and you can keep using the computer. Fill in the private folders before this step, because the inventory uses them.

Then click the crate in the bottom-right corner of the screen, choose *Chat* (聊聊), and start talking.

**Building it yourself:** you need Rust. Run `cargo build --release` in both `runtime` and `app/src-tauri`, then start `app/src-tauri/target/release/ysoul-app.exe`. Run this way, the data lives in the repository folder, not in the data folder described above.

**Step 3 (optional): add entry points to the system.**

```powershell
.\ysoul.exe setup
```

This installs nothing by itself. It creates one ticket per item below; you decide on each, and each can be undone from its ticket:

| Name | What it adds |
|---|---|
| `menu` | A "Hand to ysoul" entry in Explorer's right-click menu |
| `ys` | A `ys your question` command usable in any terminal |
| `start` | An "Ask ysoul" entry in the Start menu, so pressing Win and typing ysoul brings up the input box |
| `claude`, `cursor`, `codex` | Makes these AI coding tools tell ysoul when they finish a turn (only which folder, not what they did), so its log includes them |

To install only some, name them: `ysoul setup menu ys`.

## 5. What it does on its own

**In response to what you do:**

| You | It |
|---|---|
| Stay in one window for over 20 seconds | Leaves the crate and sits on that window's top edge |
| Switch to another window | Follows after a few seconds |
| Don't touch keyboard or mouse for over 1 minute | Curls up where it is; wakes when you move |
| Are idle for over 5 minutes | Returns to the crate |
| Pick it up and put it somewhere | Stays there until you switch windows |
| Drag it into the crate, or choose "Send it home" | Stays in the crate until you call it out |
| Watch a full-screen video or play a game | The whole layer hides. If a ticket is waiting, half its head shows at the right edge |
| Type until your text reaches it | Moves to a spot without text, or returns to the crate if there is none |

**Things it starts itself:**

| When | What it does |
|---|---|
| You return after more than 10 minutes away, or leave something you worked on for 40 minutes straight | May say one line in a bubble. It is instructed to stay silent most of the time, so usually it doesn't. At least 20 minutes between two |
| Memory usage stays above 93% for half a minute | Bristles and tells you once |
| A ticket has waited 15 minutes | Reminds you once, then no more |
| You enter a project or open a program it has not seen before | Looks up what it is and writes a few lines in its notes. Silent. At most three times a day |
| You have been away over 30 minutes and the day's log has a dozen or so entries | Summarises the day in a few sentences and saves it. At most twice a day |

Left-click a bubble to say "noted, that kind of remark is fine". Right-click to say "stop"; it will not speak unprompted for two hours. It remembers your reaction to each remark and uses that to adjust.

To silence it completely: right-click the crate and tick "Stay quiet for a while".

## 6. Talking to it

**Quick question.** Press `Ctrl+Alt+Space`. An input box appears next to the mouse cursor. Type and press Enter; the box disappears and keyboard focus returns to your program. The answer appears in the same spot; click it to dismiss. Each quick question is a separate conversation and is not mixed with the previous one.

If another program holds that shortcut, ysoul uses `Ctrl+Alt+Y`. You can also click the cat icon in the tray, or (after installing `start`) press Win and type ysoul.

**Ongoing conversation.** Click the crate, then *Chat*. This conversation is continuous. After 45 minutes without a message, the next one starts a new conversation.

**Terminal.** After installing `ys`: `ys your question`.

In both input boxes Enter sends and Shift+Enter inserts a new line.

## 7. Getting it to do things

Tell it in ordinary words. Requests fall into three groups.

### Done immediately, no confirmation

- **Opening a program.** Say "open DeepL", or just type "DeepL". This does not go through the AI model; the program matches the name and launches it.
- **Opening files, folders and web pages.**
- **Looking things up.** "Which process uses the most memory?", "What is in this folder?", "Which branch is this project on?" It runs read-only commands on the spot.
- **Scrolling and paging inside other programs.**

### Needs you to click *Do* on a ticket

- Changing system settings or the registry, deleting files, stopping services, installing or uninstalling software.
- Creating or editing files in a location the map does not allow.
- Clicking buttons, filling in fields or typing inside other programs.
- Handing a task to another AI tool ("have Claude Code fix the typos in this project").

Tickets for operating another program have a third button, **Always for this program**. After that it no longer asks for that program. If you click *Do* twice for the same program, it treats that as "always" too, and says so on the ticket. The grant is recorded in the map and can be withdrawn.

If a job has several steps with a confirmation in the middle, it continues by itself once you confirm.

### Showing it something

- **Select text, then ask.** Select a sentence in your browser and ask "what does this mean?". It can read your selection.
- **Hand it a file.** Right-click the file and choose "Hand to ysoul"; or drag the file onto the crate; or write the full path in your message. A file handed over this way can be read even if it is in a private folder. This applies to that one file only, for reading only, for half an hour.
- **Let it look at the screen.** Say "look at this". It takes one screenshot of your current window and deletes it afterwards.

### Reviewing and undoing

The paw prints beside the crate are today's executed tickets: black succeeded, red failed, faint was undone. Click one to see the output; if the action can be undone, there is an *Undo* button. Beyond five, the older ones are listed in the crate panel.

### Command line

```powershell
ysoul say hello            # ask one question
ysoul chat                 # converse in the terminal
ysoul ticket               # recent tickets; ysoul ticket do|no|undo <id>
ysoul where D:\some\path   # show the five permissions for a path
ysoul run "git status"     # submit a command to the permission check
ysoul channel              # list and choose channels
ysoul app deepl            # find a program by name; add --open to launch it
```

`ysoul --help` lists everything.

## 8. What it may and may not touch

| Action | How it is handled |
|---|---|
| Reading files, searching folders, listing processes and services, reading the registry, git status and history | Done directly, after the program checks that the location may be seen |
| Opening files, pages, programs | Done directly |
| Scrolling and paging in other programs | Done directly |
| Clicking, filling in, typing in other programs | Directly in programs you approved; a ticket otherwise |
| Editing the registry or settings, deleting, stopping services, writing where the map does not allow | A ticket |
| Any command that cannot be confirmed read-only | A ticket |
| Password fields, antivirus windows, administrator-permission prompts, chat and mail programs | Never operated |
| Contents of private folders | Never read (except the one file you hand over) |

**How this is enforced.** The limits are in the program. Each time the AI model wants to read or write a file, or run a command, the request first goes to a checking routine inside `ysoul.exe`, which consults the map and the private list and allows or refuses. The model cannot skip this step.

For commands, the routine uses PowerShell's own parser to take the command apart. A command runs directly only if it is a single command, a known read-only one, with all arguments written out literally. Commands containing variables, string building or several statements cannot be fully analysed, so they become tickets for you to review.

**Changing permissions.** Click the crate, then *Map*, to see the table; click any cell to toggle it. Or tell it "you may edit such-and-such folder from now on": it creates a ticket that changes the table, effective once you confirm. It cannot change the table by itself.

## 9. Choosing a model

ysoul looks for channels in four places:

| Source | What you need |
|---|---|
| Claude Code | Installed and logged in; `claude` works in a terminal |
| cc-switch | A provider added in cc-switch |
| Environment variables | `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `OPENROUTER_API_KEY` or `DEEPSEEK_API_KEY` is set |
| Ollama | Running on this computer |

Choose in the setup window, or:

```powershell
ysoul channel                 # list channels and their ids
ysoul channel use <id> <id>   # set the order; the next is tried when one fails
ysoul channel cheap <id>      # the channel for small jobs
ysoul channel auto            # back to defaults
```

**cc-switch is the recommended way to manage this.** [cc-switch](https://github.com/farion1231/cc-switch) is a small tool for switching API providers for Claude Code, Codex and similar tools. Install it, click *Add provider*, choose the application type, enter the endpoint URL, API key and model name, and save. Back in ysoul's setup window click *Search again* and the provider appears. The key is entered once, in cc-switch; ysoul reads it when needed and stores only which entry you picked.

**Compatibility.** Three formats are supported: calling through Claude Code, Anthropic's Messages API, and OpenAI's chat/completions API. Most services are compatible with one of the latter two. If a relay accepts requests only from the Claude Code client and you have Claude Code installed, ysoul calls it through Claude Code automatically.

One limitation: reading screenshots currently works only when the model is called through Claude Code.

## 10. Adjusting it

**Looks.** Right-click the cat or the crate: cat size (small, medium, large) and outline (none, thin white, thick white). A white outline makes the black cat easier to see on dark backgrounds.

**Name, private folders, model.** Tray icon → *Settings*.

**How it talks and behaves.** The rules are in the text file `prompts/system.md`. Edit it; changes apply from the next conversation, no rebuild needed.

**Its character.** `seed/temperament.md` is a passage you write to it before "birth", describing what you want it to be. At birth it reads this and writes its own self-description into memory. To adjust afterwards, edit `memory/me/core.md`.

**What it remembers.** Any note under `memory` can be edited or deleted.

**Timings and thresholds** are in the source and need a rebuild:

| To change | Where |
|---|---|
| How long counts as away; how short an activity is ignored | Constants at the top of `runtime/src/pulse.rs` |
| Interval between remarks, the memory threshold, when the cat leaves the crate | `behave` and `onOccasion` in `app/ui/app.js` |
| Which commands count as read-only | The three lists at the top of `runtime/src/shell.rs` |
| Which keys it may press in other programs | The key table in `runtime/src/uia.ps1` |

**The cat's drawing and motion** are defined in `design/ysoul-surface-lab.html`, a web page you can open in a browser. After editing it, run `node app/tools/extract-cat.mjs` to carry the change into the app.

## 11. Where your data goes

**Stays on this computer:**

- Everything in the `local` folder: every ticket, the action log, a copy of every request sent to the model, screenshots, and the timestamped raw activity log (which includes window titles).
- API keys. They are read at the moment of each call and exist only in memory.

**Leaves this computer in only two cases:**

- Requests sent to the model. Before sending, anything shaped like a key, an e-mail address or an account ID is replaced with a placeholder. The exact text of each request is saved in `local/outbound`, so you can check what was sent.
- If you push this folder to a remote repository with git yourself. `.gitignore` already excludes `local` and the raw activity log.

**What it does not do:**

- It does not record window titles of chat or mail programs, and does not read or capture those windows.
- It does not use the microphone.
- It does not take screenshots continuously — only one when you ask it to look, deleted afterwards.
- It does not write key contents, your real name, e-mail or accounts into memory.

**Two helper scripts:**

- `tools/suggest-private.ps1` scans the top-level folders on each drive and lists those whose names look private (chat-app data, documents, photos, downloads…), to help you complete the private list before "birth". Without arguments it only prints.
- `tools/audit.ps1` — run this before pushing your own copy of this folder (with its memory) to a remote. It checks for e-mail addresses, key-shaped strings, your Windows user name and the names of your private folders.

## 12. What it cannot do yet

- **Windows only.**
- **The interface and its default instructions are in Chinese.** See the note at the end of section 2.
- **One monitor only.** With several connected, it appears only on the primary one.
- **It can operate only the frontmost window.**
- **Some programs cannot be operated.** It works through the interface Windows provides for screen readers. Certain games, some older editors and programs that draw their own interface do not provide it, so ysoul cannot see their buttons or text and has to rely on a screenshot.
- **A comment written beside some text does not follow the text in some programs**, for the same reason.
- **Text selected with the mouse in a terminal is sometimes invisible to it.** Terminal programs that capture the mouse, such as Claude Code, draw the highlight themselves. Holding Shift while dragging usually helps.
- **Slightly complex read-only commands also become tickets.** The checking rules are conservative.
- **There is only one look**, the black cat.
- **The released binaries are not code-signed.** Windows shows a security prompt on first run.

## 13. What is in the repository

| Folder | Contents |
|---|---|
| `app` | The desktop cat. `ui` is the interface, `src-tauri` the backend, `tools` the scripts that generate interface code from the prototypes |
| `runtime` | Source of `ysoul.exe`: memory, channels, activity sensing, permission checks, actions |
| `memory` | Its notes |
| `prompts` | Instructions given to the model |
| `seed` | What you write to it before "birth" |
| `sensors` | Scripts that inventory the computer at "birth" |
| `design` | Prototypes of the interface and the cat; open them in a browser |
| `tools` | The two helper scripts above |
| `local` | Data that exists only on this computer; not in git |

## 14. References

Material and projects consulted while building this.

**Acting on a computer on someone's behalf, and limiting it**

- [Claude Code's auto mode](https://anthropic.com/engineering/claude-code-auto-mode): when judging whether an action is authorised, look only at what the user said and at the action itself, not at the model's own explanation. The permission check here follows that idea.
- OpenClaw's [exec approvals](https://docs.openclaw.ai/tools/exec-approvals) and [workspace files](https://docs.openclaw.ai/concepts/agent-workspace): keeping the assistant's memory as text files the user can edit.
- [Codex computer use](https://developers.openai.com/codex/app/computer-use): per-application approval. "Always for this program" comes from here.
- [Model Context Protocol](https://modelcontextprotocol.io): this project exposes its abilities as a local MCP server.

**When to speak unprompted**

- [Proactive Agent](https://arxiv.org/pdf/2410.12361) (Tsinghua, 2024): the main failure of proactive assistants is speaking when they should not. Silence by default comes from this paper's findings.
- [OpenClaw's heartbeat](https://docs.openclaw.ai/gateway/heartbeat): on a timed wake-up, the default answer is "nothing to report".
- [LangChain's ambient agents](https://blog.langchain.com/introducing-ambient-agents): three kinds of interruption — notify, question, review.
- [Generative Agents](https://arxiv.org/pdf/2304.03442v1) (Stanford, 2023): reflect once enough has accumulated rather than on a schedule. The trigger for the daily summary is based on it.

**Recording user activity**

- [ActivityWatch](https://github.com/ActivityWatch/activitywatch): merging window activity into time segments, and the thresholds for "away".

**Desktop pets and drawing the cat**

- [Clawd on Desk](https://github.com/rullerzhou-afk/clawd-on-desk): the interaction details a desktop pet needs (click-through, edge snapping, following the mouse, sleeping) were checked against it.
- [Black Tuxedo Cat Sprites](https://carysaurus.itch.io/black-tuxedo-cat-sprites) by Carysaurus: the main reference for the cat's look and states. The cat here is redrawn in code; none of those images are used.
- Eadweard Muybridge's sequential photographs of a walking cat in *Animal Locomotion*: reference for the four-legged gait.
- [vscode-pets](https://github.com/tonybaloney/vscode-pets): reference for the pet state machine.

**Tools used**

- [Tauri](https://tauri.app): the desktop application framework.
- [cc-switch](https://github.com/farion1231/cc-switch): model providers are read from its configuration.
- [GitHub README Best Practices](https://gingiris.github.io/growth-tools/blog/2026/03/29/github-readme-best-practices-how-to-write-a-readme-that-gets-stars/): the structure of the README.
