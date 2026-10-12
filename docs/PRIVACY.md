# Privacy start-up

Two scripts and one prompt, so that the first hour does not leak anything you would rather keep to yourself. 中文版：[PRIVACY.zh-CN.md](PRIVACY.zh-CN.md)

## 1. Find your private folders — before `birth`

`birth` surveys the machine. Folders on the private list are recorded only as "this folder exists"; nothing inside is looked at, then or later.

```powershell
pwsh tools/suggest-private.ps1          # prints candidates, changes nothing
pwsh tools/suggest-private.ps1 -Write   # writes them to sensors/census/shallow-dirs.txt
```

It lists top-level folders whose names look personal (chat-app data, mail stores, documents, photos, downloads, anything named private/personal/个人/私人…) plus your user profile. Review the result — it errs on the side of listing too much — and edit the file, or use the setup page.

## 2. Let it ask you

Paste this into the chat once, after `birth`:

> Before anything else, walk me through this machine's map. For each drive, name the top-level folders you know only by name and ask me, a few at a time: may you see inside, and may you change anything there? File a map ticket for each answer. Do not open anything I have not answered about. Stop when I say stop.

Each answer becomes a ticket; the map changes only when you click *Do*.

## 3. Audit before you push a fork

```powershell
pwsh tools/audit.ps1              # scans everything git would push
pwsh tools/audit.ps1 -Path memory # or one folder
```

It looks for e-mail addresses, account-id shapes, key-shaped strings, your Windows user name, your machine name and every name on your private-folder list. Any hit is printed with file and line (the match itself is masked) and the script exits non-zero.

What it cannot know: your real name, your school, your employer. Add them, one per line, to `local/audit-words.txt` — that file never leaves the machine.

## What the program already does

- Masks keys, e-mail addresses and account ids before any prompt leaves, and keeps a local copy of every prompt in `local/outbound/`.
- Keeps `local/` and the raw activity log out of git.
- Never records window titles of chat and mail programs.
