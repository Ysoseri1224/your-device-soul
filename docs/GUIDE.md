# Guide

Everything it does, when it does it, and how to change it. The Chinese version ([GUIDE.zh-CN.md](GUIDE.zh-CN.md)) is the original and is updated first.

- [What it is](#what-it-is)
- [Setting up](#setting-up)
- [When it shows up](#when-it-shows-up)
- [Using it](#using-it)
- [Customising](#customising)
- [What it can and cannot touch](#what-it-can-and-cannot-touch)
- [Privacy](#privacy)

## What it is

| | | where |
|---|---|---|
| **The pet** | a full-screen, click-through, transparent window. The cat, its crate and the small things are drawn on it | `app/` |
| **The core** | memory, model channels, senses, the gate, hands, tickets. Also the CLI and a local MCP server | `runtime/` |
| **Memory** | a folder of Markdown notes it maintains itself; open and edit it any time | `memory/` |

## Setting up

1. Run `ysoul-app.exe`. The first launch opens a setup page asking three things: what to call you, which folders are private (it cannot even list them), and which model to think with. Reopen it later from the tray icon's menu.
2. Run `ysoul.exe birth` once. It surveys the machine into plain-text lists under `memory/body/` and writes its first notes.
3. Optionally run `ysoul.exe setup`. Each item becomes a ticket on your desktop — nothing is installed until you click *Do*, and each can be undone from its ticket: `menu` (Explorer right-click entry), `ys` (a short terminal command), `start` (Start-menu entry, so Windows Search finds it), `claude` / `cursor` / `codex` (those tools tell it "I just finished a turn in folder X").

## When it shows up

**In response to you**

| you | it |
|---|---|
| stay in one window for 20 s | leaves the crate and sits on that window's top edge; on the taskbar if the window is maximised |
| idle for 1 min / 5 min | sleeps / goes home |
| switch windows | drops if its ledge disappeared, then follows |
| pick it up and put it somewhere | stays there until you switch windows; put it just above a line of text and it stands on that line |
| drop it in the crate, or choose *Go home* | stays in until you call it out |
| watch something full-screen | disappears entirely; shows half a head at the edge only if a ticket is waiting |
| type until text reaches its feet | moves to a blank spot, or onto a line with blank space above, or goes home |

**On its own**

| when | it |
|---|---|
| you return after 10 min, or leave something you spent 40 min on | may say one line — usually it doesn't; never more than once in 20 min |
| memory stays above 93 % for half a minute | bristles and says so |
| a ticket has waited 15 min | reminds you once |
| you enter a repo or use a program it has never seen | looks it up quietly and writes a few lines; three times a day at most |
| you are away 30 min and the day's log has grown | summarises the day; twice a day at most |
| it disagrees with what you asked | says why once, knocks a cup over, then does it if you insist |

Click a banter bubble = "nice". Right-click = "hush" for two hours.

## Using it

**Talking.** `Ctrl+Alt+Space` (falls back to `Ctrl+Alt+Y` if taken), or click the tray icon, or press Win and type `ysoul`: a one-line prompt at the cursor; each one is a fresh conversation. Click the crate → *Chat* for an ongoing conversation. `ys <words>` in a terminal.

**Showing it things.** Select text, then ask — "this" means the selection. Right-click a file → *Hand to ysoul*, drag a file onto the crate, or write a path in your message: that one item becomes readable, even in a private folder, for half an hour. "Look at this" takes one screenshot of the foreground window and deletes it afterwards. "What does this page say" reads the window's text directly.

**Getting it to act.** Just say it. Opening programs, files and pages, looking things up, scrolling and paging inside other apps happen directly. Anything that changes the machine becomes a **ticket**.

**Tickets** state what, where, whether it can be undone, and the script. *Do* runs it and keeps the result plus an *Undo* button. *Not now* discards it. *Always for this app* appears on UI-automation tickets; approving the same app twice has the same effect. Multi-step jobs continue by themselves after you approve a step. Today's finished tickets sit beside the crate as paw prints.

**The map.** Crate → *Map*: a table of places and five permissions — see, add, edit, delete, run. Click a cell to flip it. Unlisted places are read-only. If you tell it "you may edit this folder from now on", it files a map ticket; it cannot edit the map itself.

**Marks.** A circle when you ask "where?"; a margin note that follows the sentence it is about; a tag clipped to a window for things not worth interrupting you. Click to dismiss.

**CLI.** `ysoul say|chat|sense|run|ticket|where|channel|recall|app|reflect|follow|setup` — `ysoul --help` lists them.

## Customising

| to change | edit |
|---|---|
| cat size, outline | right-click the cat or the crate |
| your name, private folders, model order | tray → *Settings* (private list is `sensors/census/shallow-dirs.txt`) |
| how it speaks and what it does in which case | `prompts/system.md` — no rebuild; `{{owner}}` and `{{root}}` are filled in |
| its temperament | `seed/temperament.md` before birth; `memory/me/core.md` after |
| anything it remembers | the notes under `memory/` |
| how often it speaks, what counts as away, the memory threshold | `behave` / `onOccasion` in `app/ui/app.js`, the constants at the top of `runtime/src/pulse.rs` |
| which commands count as read-only | the three tables at the top of `runtime/src/shell.rs` |
| which keys UI automation may press | the key table in `runtime/src/uia.ps1` |
| the cat's drawing and motion | `design/ysoul-surface-lab.html`, then `node app/tools/extract-cat.mjs` |

**Model channels.** `ysoul channel` lists what it found; `ysoul channel use <id> [<id>…]` sets the order for thinking (the next is tried if one fails); `ysoul channel cheap <id>` picks the one for small errands; `ysoul channel auto` resets.

| source | how it is discovered |
|---|---|
| Claude Code | `claude` works on the command line |
| [cc-switch](https://github.com/farion1231/cc-switch) | a provider is configured there — **recommended**: endpoint, key and model live in one place and are read at run time |
| environment | `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `OPENROUTER_API_KEY`, `DEEPSEEK_API_KEY` (`*_BASE_URL` and `YSOUL_MODEL` override) |
| Ollama | it is running locally |

Three wire formats: Claude Code (its own tools, gated by a pre-tool hook), Anthropic Messages and OpenAI chat/completions (it runs its own tool loop, gated in-process by the same code). Keys stay in memory; only the chosen channel id is stored.

## What it can and cannot touch

| | how |
|---|---|
| read files, search folders, list processes and services, read the registry, `git status` / `log` | directly — after the program checks the map |
| open files, pages, programs; scroll and page in other apps | directly |
| click, fill, type, press Enter, right-click, drag in other apps | directly in apps you opened up; a ticket otherwise |
| edit the registry or settings, delete, stop services, write outside its memory, any command it cannot fully parse | a ticket |
| password fields, security software, UAC prompts, chat and mail apps | never |
| private folders | never — except the one item you hand it |

The gate is code. `..`, odd casing, trailing dots, `\\?\` and UNC paths, searching from a parent folder and glob patterns that carry a path were all tried against it and closed. A command is run directly only if PowerShell's own parser shows it to be a single pipeline whose first stage is a known read-only command with literal arguments; everything else becomes a ticket.

It cannot operate background windows, programs that expose nothing to UI Automation, or anything as administrator without a ticket.

## Privacy

- Never stored: key contents, your real name, e-mail or accounts, chat and mail contents.
- Never leaves the machine: everything under `local/` — tickets, the action log, the log of every prompt sent out, screenshots, the raw activity log.
- Leaves the machine only two ways: the prompt sent to your model (masked first, and logged in `local/outbound/`), and whatever you push to your own repository.
- No microphone. One screenshot at a time, deleted after the turn.
- Three lines it holds itself to: no deceiving, no hiding, no resisting being shut down. *Quit* is always in the tray menu.

See [PRIVACY.md](PRIVACY.md) for finding your private folders and auditing a fork.
