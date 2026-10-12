# Contributing · 参与

Thanks for wanting to help. 中文说明在每一节的后半。

## Skins · 皮肤

The most wanted contribution. Only the black cat exists today; four more are designed but not built.

- `design/ysoul-skin-lab.html` — the in-crate cat and the skin system. Open it in a browser; it is a clickable prototype.
- `design/ysoul-surface-lab.html` — the cat outside the crate (skeleton, poses, gait, physics, window-hopping) and every on-screen component.
- The app's engine is **generated** from the prototype. Do not edit `app/ui/catrig.js`, `cat.css` or `skin.js` by hand:

```powershell
node app/tools/extract-skin.mjs   # skin  → app/ui/skin.js
node app/tools/extract-cat.mjs    # cat   → app/ui/catrig.js, cat.css
```

One rule learned the hard way: class names in the page must not collide with animated classes in the skin CSS (`note`, `win`, `desk`, `thumb`, `off`, `thin`). Search `app/ui/skin.js` before naming a class.

最想要的贡献是皮肤。现在只有黑猫一件，另外四件有设计、还没做。样稿是能点的网页；程序里的引擎是从样稿**生成**的，所以改样稿，再跑上面两条命令。页面里的类名不要和皮肤 CSS 里带动画的类撞名。

## Code · 代码

```powershell
cd runtime; cargo test          # needs PowerShell 7 on PATH
cd app/src-tauri; cargo build
```

- The UI is embedded at build time: after changing `app/ui/*` or `runtime/`, rebuild and restart.
- Anything that touches the gate (`places.rs`, `gate.rs`, `shell.rs`, `judge.ps1`) needs a test that shows what is refused, not only what is allowed.
- Comments and notes in the code are in Chinese; either language is fine in PRs.

界面文件是编进程序里的。凡是碰到那道门的改动，测试里要有"哪些被拦下"，不能只有"哪些放行"。

## Before you push · 推之前

If your fork contains your own `memory/`, run:

```powershell
pwsh tools/audit.ps1
```

It looks for e-mail addresses, account IDs, key-shaped strings, your Windows user name and the names on your private-folder list. A hit stops the script.

如果你的分支里带着你自己的 `memory/`，推之前先跑一遍审查脚本。
