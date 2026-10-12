// 桌面上的 ysoul。
// 这一层盖住整块屏幕。箱子是它的窝，可以拖；猫出了箱子是自由的，把真的窗口当猫爬架。
// 猫和箱子的画法、动作、寻路都在 catrig.js（由样稿生成）；这里管的是"什么时候做什么"，以及箱子摊开、说话这几块。
(function () {
  'use strict';
  var T = window.__TAURI__ || null, SKIN = window.YSOUL_SKIN;
  var $ = function (id) { return document.getElementById(id); };
  var stage = $('stage'), nowEl = $('now'), bubble = $('bubble'), tray = $('tray'), talk = $('talk'), ticket = $('ticket'), map = $('map'), marksEl = $('marks'), paws = $('paws'), peek = $('peek'), quick = $('quick'), quickIn = $('quick-in'), answer = $('answer'), log = $('log'), form = $('form'), input = $('input');
  var CAT_PX = 100, BOX_PX = 132;      // 猫画多宽（像素）。右键菜单里能改：小 72、中 100、大 140
  var SIZES = { s: 72, m: 100, l: 140 }, OUTLINES = { '0': 0, '1': 1.3, '2': 2.6 };

  var style = document.createElement('style'); style.textContent = SKIN.css; document.head.appendChild(style);
  $('defs').innerHTML = SKIN.defs; $('defs').style.cssText = 'position:absolute;width:0;height:0;overflow:hidden';

  var cat = window.YSOUL_CAT(stage);
  // 此刻桌面的样子（由后端每隔一会儿送来）和它自己的几个状态
  var S = { ready: false, floor: 95, ledges: [], fore: null, idle: 0, away: false, now: null, mem: 0, fullscreen: false };
  var me = { size: 'm', outline: '1', dnd: false, bx: null, by: null, asleep: false, hot: false, hotTicks: 0, foreKey: '', foreTicks: 0, pending: null, busy: false, saidHot: 0 };

  // ── 台面：任务栏是零线，每个窗口上沿没被挡住的一截是一层 ──
  function surfaces() {
    var L = [{ id: 'floor', y: S.floor, x0: 2, x1: 98 }];
    S.ledges.forEach(function (l) { if (l.x1 - l.x0 > 6) L.push({ id: l.key, y: l.y, x0: l.x0 + 1.2, x1: l.x1 - 1.2, left: l.x0, right: l.x1, fore: l.fore }); });
    if (me.perch) L.push(me.perch);      // 它正踩着的那行字
    return L;
  }
  function under() { var r = cat.rig(); if (!r) return null; var hit = null; surfaces().forEach(function (s) { if (Math.abs(s.y - r.gy) < 1.8 && r.fx >= s.x0 - 3 && r.fx <= s.x1 + 3) hit = s; }); return hit; }
  function measure() {
    var w = window.innerWidth, h = window.innerHeight;
    cat.config({ kw: CAT_PX / w * 100, kh: CAT_PX * 215 / 250 / h * 100, floor: S.floor, surfaces: surfaces });
  }

  // ── 后端送来的"此刻" ──
  function onScene(d) {
    var sw = d.screen.right - d.screen.left, sh = d.screen.bottom - d.screen.top; if (!sw || !sh) return;
    S.floor = (d.screen.floor - d.screen.top) / sh * 100;
    S.ledges = d.ledges.map(function (l) { return { key: l.app + '@' + l.y + ':' + l.x0, y: (l.y - d.screen.top) / sh * 100, x0: (l.x0 - d.screen.left) / sw * 100, x1: (l.x1 - d.screen.left) / sw * 100, fore: l.fore, app: l.app }; });
    S.scr = d.screen;
    // 你切到了别的程序：说话的地方和就地问的那一行收起。系统自己弹出来的小东西（剪贴板、输入法、开始菜单）不算切走
    var SYS = { textinputhost: 1, shellexperiencehost: 1, startmenuexperiencehost: 1, searchapp: 1, searchhost: 1, explorer: 1, ctfmon: 1, chsime: 1, applicationframehost: 1 };
    var newFore = d.fore_app || null;
    if (newFore && S.foreApp && newFore !== S.foreApp && !SYS[newFore] && !SYS[S.foreApp] && Date.now() - (me.openedAt || 0) > 1500) {
      if (!quick.hidden) { quick.hidden = true; }
      if (!talk.hidden && !me.busy && !input.value.trim()) { talk.hidden = true; }
      layout();
    }
    S.foreApp = newFore;
    if (d.waiting !== S.waiting) { S.waiting = d.waiting; loadTicket(); loadPaws(); }
    S.fore = d.fore; S.idle = d.idle; S.away = d.away; S.now = d.now; S.mem = d.mem; S.fullscreen = d.fullscreen;
    measure();
    if (!S.ready) { S.ready = true; start(); loadPaws(); }
    behave();
    drawMarks(d.marks || []);
  }

  function start() {
    var w = window.innerWidth, h = window.innerHeight, boxH = BOX_PX * 152 / 180;
    var x = me.bx == null ? 100 - (BOX_PX + 30) / w * 100 : me.bx, y = me.by == null ? S.floor - boxH * .947 / h * 100 : me.by;
    var box = cat.summon(x, y, 'calm');
    box.ontap = function () { toggle(tray); tuck(); };
    box.addEventListener('pointerdown', function () { untuck(6000); });
    ['pointercancel', 'lostpointercapture'].forEach(function (n) { cat.kitty().addEventListener(n, function () { setTimeout(drop, 0); }); });
    cat.kitty().addEventListener('pointerup', function (e) {
      if (!cat.kitty().classList.contains('held') && !me.wasHeld) return;
      var x = e.clientX, y = cat.kitty().getBoundingClientRect().bottom;      // 松手时它脚的位置
      setTimeout(function () { landOnText(x, y); }, 650);                       // 等它先落稳
    });
    box.onmoved = function () { var r = cat.rel(box); me.bx = r.x; me.by = r.y; save(); layout(); };
    box.addEventListener('contextmenu', function (e) { e.preventDefault(); if (T) T.core.invoke('show_menu', { dnd: me.dnd, size: me.size, outline: me.outline }); });
    if (me.pending) cat.count(1);
    cat.kitty().addEventListener('contextmenu', function (e) { e.preventDefault(); if (T) T.core.invoke('show_menu', { dnd: me.dnd, size: me.size, outline: me.outline }); });      // 在猫身上点右键也是这张菜单
    dress(); layout();
  }

  // ── 什么时候做什么 ──
  // 这张表现在接得上的部分。其余的（替你做事时"干活"、有事要你定时"举牌"、不赞成时"不同意"）要等它有了手才接得上。
  function behave() {
    if (!cat.box()) return;
    var veiled = !!S.fullscreen && Date.now() > (me.unveilUntil || 0);
    document.body.classList.toggle('veil', veiled);     // 你在全屏看东西：它整个不出现
    peek.hidden = !(veiled && S.waiting > 0);            // 有票等着，才在屏幕边上露半个头
    if (veiled) return;
    var r = cat.rig(), out = cat.isOut(), held = cat.kitty().classList.contains('held'), free = cat.idle() && !held && !me.busy && !me.busyTicket && !(tk && tk.status === 'waiting');
    showNow();
    tuck();
    if (held) me.wasHeld = true;
    else if (me.wasHeld) { me.wasHeld = false; if (!out) me.stay = true; else { me.stay = false; me.placedKey = me.foreKey; } }
    if (out && !held && !r.turning && !cat.idle() && (!r.move || r.move.kind === 'walk') && !under()) { cat.settle(); return; }

    // 身体不舒服：内存连着半分钟顶在九成三以上，炸毛；回落到八成八以下才松下来
    me.hotTicks = S.mem >= 93 ? me.hotTicks + 1 : 0;
    if (!me.hot && me.hotTicks > 40) { me.hot = true; if (out && free) cat.be('hot', 300); else if (!out) cat.pose('hot'); if (!me.dnd && Date.now() - me.saidHot > 3600e3) { me.saidHot = Date.now(); say1('内存用到 ' + S.mem + '% 了，我有点喘。', 6000); } }
    if (me.hot && S.mem < 88) { me.hot = false; if (out && free) cat.be('sit', 460); else if (!out) cat.pose('calm'); }

    // 你走开五分钟：它回箱子
    if (S.away) { if (out && free) home('sleep'); else if (!out) cat.pose('sleep'); me.asleep = true; return; }
    // 你一分钟没动：它就地蜷着睡；你一回来它就醒
    var sleepy = S.idle >= 60 || me.dnd;
    if (sleepy && !me.asleep) { me.asleep = true; if (out && free) cat.be('loaf', 700); else if (!out) cat.pose('sleep'); }
    if (!sleepy && me.asleep) { me.asleep = false; if (out && free) cat.be('sit', 460); else if (!out) cat.pose(me.hot ? 'hot' : 'calm'); }
    if (me.asleep || me.hot || !free) return;

    // 它脚下的台面没了（窗口被关掉、挪走、最大化）：掉到下面最近的一层
    if (out && !under()) { cat.settle(); return; }

    // 跟着你：你在哪个窗口里待住了，它就到那个窗口的上沿蹲着。短暂切过去看一眼不算。
    var ledge = S.ledges.filter(function (l) { return l.fore; }).sort(function (a, b) { return (b.x1 - b.x0) - (a.x1 - a.x0); })[0];
    var key = ledge ? ledge.key : 'none';
    if (key === me.foreKey) me.foreTicks++; else { me.foreKey = key; me.foreTicks = 0; }
    if (me.foreTicks < 4) return;
    if (out && me.perch && Math.abs(r.gy - me.perch.y) < 1.8) return;      // 它是为了不挡字才站到那行字上的：不把它叫回去
    if (out && me.placedKey != null) { if (me.placedKey === key) return; me.placedKey = null; }     // 是你把它放在这儿的：你换了窗口它才动
    if ((me.stay || Date.now() < (me.hideUntil || 0)) && !out) return;
    var settled = S.now && (Date.now() / 1000 - S.now[2]) >= 20;   // 在同一件事上待满二十秒，它才从箱子里出来
    if (ledge) {
      var x = Math.max(ledge.x0 + 3, ledge.x1 - 7), on = out && Math.abs(r.gy - ledge.y) < 1.8 && r.fx >= ledge.x0 - 1 && r.fx <= ledge.x1 + 1;
      if (!on && (out || settled)) cat.travel(x, ledge.y, 'sit');
    } else if (out && r.gy < S.floor - 2) { cat.floor(Math.min(92, Math.max(8, r.fx)), 'sit'); }
    // 你用的窗口占满了屏幕、没有上沿可蹲：它出来坐在任务栏上，挨着箱子
    else if (!out && settled) { cat.floor(Math.max(6, cat.rel(cat.box()).x - 7), 'sit'); }
  }

  // ── 猫出来了，箱子就收到屏幕边上，只露一小半；点它、有东西摊开着、猫要回家时再出来 ──
  var untuckUntil = 0;
  function tuck() {
    var box = cat.box(); if (!box) return;
    var open = !tray.hidden || !talk.hidden || !map.hidden || !ticket.hidden, want = cat.isOut() && !open && Date.now() > untuckUntil;
    if (want === box.classList.contains('tucked')) return;
    if (want) {
      var b = box.getBoundingClientRect(), w = window.innerWidth, right = b.left + b.width / 2 > w / 2;
      box.style.setProperty('--tuck', (right ? w - b.left - b.width * .34 : -(b.right - b.width * .34)) + 'px');
    }
    box.classList.toggle('tucked', want); setTimeout(layout, 340);
  }
  function untuck(ms) { untuckUntil = Date.now() + (ms || 4000); tuck(); }
  function home(state) { untuck(60000);
    // 是你叫它回去的（不是它自己去睡）：待在箱子里，直到你叫它出来
    if (state !== 'sleep') me.stay = true;
    setTimeout(function () { cat.goHome(state); cat.whenIdle(function () { untuckUntil = 0; }); }, 340); }

  // ── 挡住字了就让开；一行字上面是空的，可以踩在那行字上 ──
  // 量法：取几根"柱子"，每根一个身子宽，从上到下切成 6 像素一条，每条各自"有多满"。一行字所在的那几条是满的，空白是空的。
  //   · 它身子占的那一段里有满的条 → 挡住字了。
  //   · 落脚处：一段够放下身子的空白，这段空白的底下要么是它本来站的那一层，要么是一行字（踩在字上，那行字临时算一层台面）。
  // 两个时候会找落脚处：
  //   一、它停着的时候每三秒量一次，连着两次挡着字就挪。字上的落脚处和旁边的空地一起比，往上抬的代价算得轻，所以近处有一行能踩的字，它会先踩上去。
  //   二、你把它拎起来放下的时候：松手的地方附近有一行能踩的字，它就站到那行字上，不一路掉到任务栏。
  var BAND = 6, FULL = 0.10, dodge = { hits: 0, until: 0 };
  var BODY_W, BODY_H, NEED, REACH = 540;
  // 换大小、换描边。身子的宽高跟着变：小的那档身子只有四十来像素高，终端里两行空白就站得下
  function dress() {
    CAT_PX = SIZES[me.size] || SIZES.m; BODY_W = CAT_PX * .46; BODY_H = CAT_PX * .58; NEED = Math.ceil(BODY_H / BAND);
    document.documentElement.style.setProperty('--kw', CAT_PX + 'px');
    var ol = OUTLINES[me.outline] || 0; document.documentElement.style.setProperty('--ol', ol + 'px');
    if (cat.kitty()) cat.kitty().classList.toggle('ol', ol > 0);
    measure(); me.perch = null;
  }
  dress();
  function columns(xs, top, base) { return T.core.invoke('under', { rects: xs.map(function (x) { return { x: x - BODY_W / 2, y: top, w: BODY_W, h: base - top }; }), band: BAND }); }
  function clearAbove(col, e) { for (var i = e - NEED; i < e; i++) if (!(col[i] <= FULL * .6)) return false; return true; }
  // 这根柱子里所有的落脚处：脚站在第 e 条的下沿。e 等于总条数就是原来那一层，小于它就是踩在一行字上
  function perches(col) { var n = col.length, out = []; for (var e = n; e >= NEED; e--) { if (e < n && !(col[e] > FULL && !(col[e - 1] > FULL))) continue; if (clearAbove(col, e)) out.push(e); } return out; }
  function covered(col) { var n = col.length, busy = 0; for (var i = Math.max(0, n - NEED); i < n; i++) if (col[i] > FULL) busy++; return busy >= 2; }
  function stand(x, y, base, onText) {
    if (onText) y -= 5;      // 脚落在那行字的上沿之上一点，不压着字
    var w = window.innerWidth, h = window.innerHeight, xp = x / w * 100, yp = y / h * 100;
    me.perch = onText ? { id: 'perch', y: yp, x0: xp - 3.2, x1: xp + 3.2, base: base } : null;
    measure(); cat.travel(xp, onText ? yp : base / h * 100, 'sit');
  }
  function spread(here, count) { var w = window.innerWidth, step = CAT_PX * .5, xs = [here]; for (var k = 1; k <= count; k++) { [-1, 1].forEach(function (dir) { var x = here + dir * k * step; if (x > w * .03 && x < w * .97) xs.push(x); }); } return xs; }

  // 你把它放下了：松手的地方附近（上下左右一百来像素）有一行能踩的字，就站上去
  function landOnText(dropX, dropY) {
    if (!T || !cat.isOut()) return;
    var w = window.innerWidth, h = window.innerHeight, base = S.floor / 100 * h, top = Math.max(0, dropY - 260), xs = spread(dropX, 2);
    if (base - top < BODY_H + 20) return;
    columns(xs, top, base).then(function (cols) {
      if (!cat.isOut() || cat.kitty().classList.contains('held')) return;
      var best = null;
      cols.forEach(function (col, i) { if (!col) return; perches(col).forEach(function (e) { if (e >= col.length) return; var y = top + e * BAND, cost = Math.abs(xs[i] - dropX) + Math.abs(y - dropY); if (cost < 190 && (!best || cost < best.cost)) best = { x: xs[i], y: y, cost: cost }; }); });
      if (best) { dodge.until = Date.now() + 6000; cat.whenIdle(function () { stand(best.x, best.y, base, true); }); }
    }, function () { /* 量不了就让它照原样落下去 */ });
  }

  setInterval(function () {
    var r = cat.rig(), w = window.innerWidth, h = window.innerHeight;
    // 它已经不在那行字上了（被你拎走、自己去别处了）：那行字不再算台面
    if (me.perch && r && cat.idle() && (Math.abs(r.gy - me.perch.y) > 1.8 || !cat.isOut())) me.perch = null;
    if (!T || !cat.box() || !cat.isOut() || !cat.idle() || S.fullscreen || S.away || me.busy || me.busyTicket || Date.now() < dodge.until) { dodge.hits = 0; return; }
    if (cat.kitty().classList.contains('held') || (tk && tk.status === 'waiting')) return;
    var s = under(); if (!r || !s) return;
    var feet = r.gy / 100 * h, here = r.fx / 100 * w, base = me.perch ? me.perch.base : feet, top = Math.max(0, base - REACH), xs = spread(here, 10);
    columns(xs, top, base).then(function (cols) {
      if (!cat.isOut() || !cat.idle() || !cols[0] || !cols[0].length) return;
      var n = cols[0].length, mine = cols[0].slice(0, Math.max(NEED, Math.round((feet - top) / BAND)));     // 它脚以上的那一段
      // 踩在字上的时候，脚下那行字没了（滚走了）：不能悬在半空，重新找地方
      var floating = me.perch && !(cols[0][mine.length] > FULL);
      if (!covered(mine) && !floating) { dodge.hits = 0; return; }
      if (++dodge.hits < 2) return;
      dodge.hits = 0; dodge.until = Date.now() + 9000;
      var best = null;
      cols.forEach(function (col, i) {
        if (!col || col.length !== n) return;
        perches(col).forEach(function (e) {
          var y = top + e * BAND; if (i === 0 && Math.abs(y - feet) < 2) return;                 // 就是它现在站的地方
          var cost = Math.abs(xs[i] - here) + (base - y) * .35;                                // 往上抬算得轻：近处有能踩的字，优先踩上去
          if (!best || cost < best.cost) best = { x: xs[i], y: y, onText: e < n, cost: cost };
        });
      });
      if (!best) { me.perch = null; dodge.until = Date.now() + 60000; me.hideUntil = Date.now() + 60000; home('calm'); me.stay = false; return; }
      stand(best.x, best.y, base, best.onText);
    }, function () { /* 量不了就不挪 */ });
  }, 3000);

  // ── 此刻条：它认为你现在在干什么 ──
  function showNow() {
    if (!S.now || S.away || me.dnd) { nowEl.hidden = true; return; }
    nowEl.innerHTML = '你在：<b></b>'; nowEl.firstElementChild.textContent = S.now[0] + (S.now[1] ? ' · ' + S.now[1] : ''); nowEl.hidden = false;
    $('tray-now').textContent = S.now[0] + (S.now[1] ? ' · ' + S.now[1] : '');
    $('tray-body').textContent = '内存用了 ' + S.mem + '%';
  }

  // ── 几块东西各放在哪 ──
  function layout() {
    var box = cat.box(); if (!box) return; var b = box.getBoundingClientRect(), w = window.innerWidth, h = window.innerHeight;
    var half = (nowEl.offsetWidth || 140) / 2 + 6; nowEl.style.left = Math.max(half, Math.min(w - half, b.left + b.width / 2)) + 'px'; nowEl.style.top = Math.max(4, b.top - 46) + 'px';
    paws.style.left = Math.max(4, Math.min(w - paws.childElementCount * 23 - 6, b.left - paws.childElementCount * 23 - 4)) + 'px'; paws.style.top = (b.bottom - 24) + 'px';
    // 摊开的几块：贴着箱子的左边（放不下就右边），底边对齐箱子的底边，内容多了往上长，不会长出屏幕
    var open = null, floorGap = h - S.floor / 100 * h;
    [tray, talk, map].forEach(function (p) {
      if (p.hidden) return; open = p; var r = p.getBoundingClientRect(), left = b.left - r.width - 14; if (left < 8) left = Math.min(w - r.width - 8, b.right + 14); left = Math.min(left, w - r.width - 8);
      p.style.left = Math.max(8, left) + 'px'; p.style.top = 'auto'; p.style.bottom = Math.max(8, Math.min(h - b.bottom, h - 8 - Math.min(r.height, h - 16)), floorGap > 4 && h - b.bottom < floorGap ? 8 : 0) + 'px';
    });
    // 票：平时在箱子上方。有一块摊开着，就放到那一块的上面；上面放不下就放到它左边。总之不被盖住
    if (!ticket.hidden) {
      var tr = ticket.getBoundingClientRect(), left2 = Math.max(8, Math.min(w - tr.width - 8, b.right - tr.width)), top2 = Math.max(8, b.top - tr.height - 56);
      if (open) {
        var pr = open.getBoundingClientRect();
        if (pr.top - tr.height - 10 >= 8) { left2 = Math.max(8, Math.min(w - tr.width - 8, pr.left)); top2 = pr.top - tr.height - 10; }
        else if (pr.left - tr.width - 10 >= 8) { left2 = pr.left - tr.width - 10; top2 = Math.max(8, Math.min(h - tr.height - 8, pr.bottom - tr.height)); }
        else { left2 = Math.max(8, Math.min(w - tr.width - 8, pr.left)); top2 = 8; }
      }
      ticket.style.left = left2 + 'px'; ticket.style.top = top2 + 'px';
      // 就地问的回话要是和票叠在一起了，回话让开：挪到票的左边，放不下就挪到上面
      if (!answer.hidden) {
        var ar = answer.getBoundingClientRect(), t2 = ticket.getBoundingClientRect();
        if (ar.left < t2.right && ar.right > t2.left && ar.top < t2.bottom && ar.bottom > t2.top) {
          if (t2.left - ar.width - 10 >= 8) answer.style.left = (t2.left - ar.width - 10) + 'px'; else answer.style.top = Math.max(8, t2.top - ar.height - 10) + 'px';
        }
      }
    }
  }
  function toggle(p) {
    var show = p.hidden, wasTalking = !talk.hidden; tray.hidden = true; talk.hidden = true; map.hidden = true;
    if (wasTalking && !(show && p === talk)) giveBack();
    if (show) { p.hidden = false; if (p === tray) fillTray(); if (p === talk) { openTalk(); } if (p === map) fillMap(); }
    layout();
  }
  function drop() { var k = cat.kitty(); if (k.classList.contains('held')) k.dispatchEvent(new PointerEvent('pointerup', { bubbles: true, clientX: curs ? curs.x : k.getBoundingClientRect().left, clientY: curs ? curs.y : 0 })); }
  function giveBack() { if (T) T.core.invoke('yield_focus'); }
  // 兜底：这一层只在你跟它说话的时候才该拿着键盘。别的时候发现键盘在它手里（启动时、别的窗口关掉时系统塞给它的），就还回去
  setInterval(function () { if (document.hasFocus() && talk.hidden && quick.hidden) giveBack(); }, 1500);
  function fillTray() {
    $('tray').querySelector('[data-do=out]').textContent = cat.isOut() ? '回箱子' : '出来';
    $('tray').querySelector('[data-do=dnd]').classList.toggle('on', me.dnd);
    var ul2 = $('tray-done'); ul2.textContent = ''; $('tray-done-col').hidden = !doneToday.length;
    doneToday.forEach(function (t) { var li = document.createElement('li'); li.className = t.status; li.textContent = (t.at || '').slice(11, 16) + '　' + t.title; li.onclick = function () { showTicket(t, 0); }; ul2.appendChild(li); });
    if (!T) return;
    T.core.invoke('diary_today').then(function (lines) {
      var ul = $('tray-day'); ul.textContent = '';
      if (!lines.length) { var li = document.createElement('li'); li.textContent = '还没有记下什么'; ul.appendChild(li); }
      lines.slice(-12).forEach(function (l) { var li = document.createElement('li'); li.textContent = l; ul.appendChild(li); });
      ul.scrollTop = ul.scrollHeight; layout();
    });
  }
  tray.addEventListener('click', function (e) {
    var b = e.target.closest('button'); if (!b) return; var what = b.getAttribute('data-do');
    if (what === 'talk') toggle(talk);
    else if (what === 'out') { tray.hidden = true; if (cat.isOut()) home('calm'); else { me.stay = false; var r = cat.rel(cat.box()); cat.floor(Math.max(6, r.x - 8), 'sit'); } }
    else if (what === 'map') toggle(map);
    else if (what === 'notes' && T) T.core.invoke('open_memory', { rel: 'you/said.md' });
    else if (what === 'diary' && T) T.core.invoke('open_memory', { rel: 'diary/today' }).catch(function () { say1('今天还没记下什么。', 3000); });
    else if (what === 'dnd') { me.dnd = !me.dnd; b.classList.toggle('on', me.dnd); save(); }
  });

  // ── 票 ──
  // 它要动这台机器之前递一张票。你点了"做"它才动手；做完，票上留着结果和撤回的办法。
  var tk = null;
  function showTicket(t, more) {
    tk = t; var waiting = t.status === 'waiting', done = t.status === 'done' || t.status === 'failed';
    $('tk-title').textContent = ({ done: '做完了：', failed: '没做成：', undone: '撤回了：', refused: '不做：' }[t.status] || '') + t.title;
    $('tk-place').textContent = t.place || '—';
    $('tk-undo').textContent = t.undo.trim() ? (t.status === 'undone' ? '已经撤回' : '可以撤回') : '撤不回';
    $('tk-script').textContent = t.script; ticket.querySelector('summary').textContent = t.map ? '要记进地图的这一行' : '要运行的脚本';
    $('tk-out').textContent = t.output || ''; $('tk-out').hidden = !t.output;
    $('tk-more').textContent = more > 1 ? '还有 ' + (more - 1) + ' 张' : '';
    ticket.classList.toggle('admin', !!t.elevated);
    ticket.querySelector('[data-tk=do]').hidden = !waiting; ticket.querySelector('[data-tk=no]').hidden = !waiting; ticket.querySelector('[data-tk=always]').hidden = !(waiting && t.ui);
    ticket.querySelector('details').hidden = !!t.ui;
    ticket.querySelector('[data-tk=undo]').hidden = !(done && t.undo.trim()); ticket.querySelector('[data-tk=close]').hidden = waiting;
    ticket.hidden = false; layout();
  }
  function loadTicket() {
    if (!T || me.busyTicket) return;
    T.core.invoke('tickets_waiting').then(function (list) {
      if (cat.box()) cat.count(list.length + (me.pending ? 1 : 0));
      if (list.length) { showTicket(list[list.length - 1], list.length); mood('ask', 'ask'); }
      else if (tk && tk.status === 'waiting') { ticket.hidden = true; tk = null; mood('calm', 'sit'); }
    });
  }
  ticket.addEventListener('click', function (e) {
    var b = e.target.closest('button'); if (!b || !tk) return; var what = b.getAttribute('data-tk');
    if (what === 'close') { ticket.hidden = true; tk = null; mood('calm', 'sit'); loadTicket(); return; }
    me.busyTicket = true; ticket.querySelectorAll('button').forEach(function (x) { x.disabled = true; });
    if (what !== 'no') mood('think', 'work');
    var call = T ? T.core.invoke('ticket_decide', { id: tk.id, what: what }) : Promise.resolve(Object.assign({}, tk, { status: what === 'do' || what === 'always' ? 'done' : what === 'undo' ? 'undone' : 'refused', output: '（演示）' }));
    call.then(function (t) {
      if (t.status === 'refused') { ticket.hidden = true; tk = null; mood('calm', 'sit'); }
      else {
        showTicket(t, 0); if (t.status === 'failed') mood('oops', 'oops'); else mood('joy', 'joy'); setTimeout(function () { mood('calm', 'sit'); }, 2600);
        carryOn(t);
      }
    }, function (err) { $('tk-out').textContent = String(err); $('tk-out').hidden = false; mood('oops', 'oops'); layout(); })
      .then(function () { loadPaws(); me.busyTicket = false; ticket.querySelectorAll('button').forEach(function (x) { x.disabled = false; }); if (!tk) loadTicket(); });
  });

  // ── 家的地图：它在每个地方能做什么。你点一格就改，这是你亲手改的，不用票 ──
  var ACTS = ['see', 'put', 'edit', 'drop', 'run'];
  function drawMap(view) {
    var body = $('map-rows'); body.textContent = '';
    view.rows.forEach(function (p) {
      var tr = document.createElement('tr'), td = document.createElement('td'), b = document.createElement('b'), sp = document.createElement('span');
      b.textContent = p.path; sp.textContent = p.what + ' · ' + p.source; td.appendChild(b); td.appendChild(sp); tr.appendChild(td);
      ACTS.forEach(function (a) {
        var c = document.createElement('td'), btn = document.createElement('button');
        btn.textContent = p[a] ? '✓' : '–'; btn.className = p[a] ? 'yes' : ''; btn.title = (p[a] ? '现在可以，点一下改成不行' : '现在不行，点一下改成可以');
        btn.onclick = function () { if (T) T.core.invoke('place_toggle', { path: p.path, act: a }).then(drawMap); else { p[a] = !p[a]; drawMap(view); } };
        c.appendChild(btn); tr.appendChild(c);
      });
      body.appendChild(tr);
    });
    $('map-private').textContent = view.private ? '另有 ' + view.private + ' 处你列为私人的地方不在这张表上，它连看都不能看。' : '';
    layout();
  }
  function fillMap() { if (T) T.core.invoke('places_view').then(drawMap); }
  map.addEventListener('click', function (e) {
    var b = e.target.closest('[data-map]'); if (!b) return;
    if (b.getAttribute('data-map') === 'file' && T) T.core.invoke('open_memory', { rel: 'us/places.md' });
    else if (b.getAttribute('data-map') === 'close') { map.hidden = true; }
  });

  // ── 爪印：今天它动过手的事，一件一个，排在箱子脚边。点开是那张票，能撤回 ──
  var PAW = '<svg viewBox="0 0 20 20"><ellipse cx="10" cy="13.4" rx="5" ry="4.2"/><ellipse cx="4" cy="8.4" rx="1.9" ry="2.5"/><ellipse cx="8" cy="5" rx="1.9" ry="2.6"/><ellipse cx="12.4" cy="5" rx="1.9" ry="2.6"/><ellipse cx="16.2" cy="8.4" rx="1.9" ry="2.5"/></svg>';
  var doneToday = [];
  function drawPaws(list) {
    doneToday = list; paws.textContent = '';
    if (list.length > 5) { var more = document.createElement('button'); more.className = 'more'; more.textContent = '+' + (list.length - 5); more.title = '更早的在箱子里'; more.onclick = function () { if (tray.hidden) toggle(tray); }; paws.appendChild(more); }
    list.slice(0, 5).reverse().forEach(function (t) {
      var b = document.createElement('button'); b.className = t.status; b.innerHTML = PAW; b.title = t.title;
      b.onclick = function () { showTicket(t, 0); };
      paws.appendChild(b);
    });
    paws.hidden = !list.length; layout();
  }
  function loadPaws() { if (T) T.core.invoke('tickets_today').then(drawPaws); }

  // ── 记号：夹在窗口上的条、圈、页边批注。都是它用工具留的，后端每跳一下送来还没过期的 ──
  var marksKey = '';
  function drawMarks(list) {
    var dpr = window.devicePixelRatio || 1, scr = S.scr || { left: 0, top: 0 }, w = window.innerWidth, h = window.innerHeight;
    // 条要跟着窗口走，所以每一跳都重摆；圈和批注位置不变，内容没变就不重画，免得动画反复重来
    list = list.filter(function (m) { return m.kind === 'tag' || m.rect[2] - m.rect[0] > 3; });      // 跟着字走的批注，字滚出去了就先不画
    var key = JSON.stringify(list.map(function (m) { return [m.id, m.rect]; })) + '|' + JSON.stringify(S.ledges.map(function (l) { return [l.app, Math.round(l.y), Math.round(l.x0)]; })) + '|' + S.foreApp;
    if (key === marksKey) return; marksKey = key;
    marksEl.textContent = '';
    var stack = {};
    list.forEach(function (m) {
      var clear = function () { if (T) T.core.invoke('mark_clear', { id: m.id }); marksKey = ''; el.remove(); if (say) say.remove(); }, el, say = null;
      if (m.kind === 'tag') {
        var ledge = S.ledges.filter(function (l) { return l.app === m.app; }).sort(function (a, b) { return (b.fore ? 1 : 0) - (a.fore ? 1 : 0); })[0];
        if (!ledge && m.app !== S.foreApp) return;          // 那个窗口现在看不见：条先不出现
        var n = stack[m.app] = (stack[m.app] || 0) + 1;
        el = document.createElement('div'); el.className = 'clip'; el.textContent = m.text; el.title = '点一下收掉';
        el.style.left = (ledge ? ledge.x0 / 100 * w + 16 : w * .5 - 120) + (n - 1) * 26 + 'px';
        el.style.top = (ledge ? Math.max(8, ledge.y / 100 * h - 12) : 10) + (n - 1) * 30 + 'px';
        el.onclick = clear;
      } else {
        var x = (m.rect[0] - scr.left) / dpr, y = (m.rect[1] - scr.top) / dpr, rw = (m.rect[2] - m.rect[0]) / dpr, rh = (m.rect[3] - m.rect[1]) / dpr, pad = m.kind === 'ring' ? 9 : 2;
        el = document.createElement('div'); el.className = 'ring' + (m.kind === 'note' ? ' aside' : '');
        el.style.left = (x - pad) + 'px'; el.style.top = (y - pad) + 'px'; el.style.width = (rw + pad * 2) + 'px'; el.style.height = (rh + pad * 2) + 'px';
        el.innerHTML = '<svg><ellipse pathLength="100" cx="50%" cy="50%" rx="50%" ry="50%"/><ellipse pathLength="100" cx="50.6%" cy="49%" rx="51.5%" ry="53%"/></svg>';
        if (m.text) {
          say = document.createElement('div'); say.className = 'mark-say'; say.textContent = m.text; say.title = '点一下收掉'; say.onclick = clear;
          // 批注写在右边的页边，右边没地方就写在下面；圈的说明写在圈的右下
          var right = x + rw + pad + 10, roomRight = w - right > 180;
          if (m.anchor) { roomRight = false; }
          say.style.left = (m.kind === 'note' && !roomRight ? x : Math.min(w - 270, m.kind === 'note' ? right : x + rw * .6)) + 'px';
          say.style.top = Math.min(h - 40, m.kind === 'note' && roomRight ? y : y + rh + pad + 6) + 'px';
          marksEl.appendChild(say);
        }
      }
      marksEl.appendChild(el);
    });
  }

  // ── 光标旁的一行字：不开对话框，就地问一句。回话也就地出现，点一下收掉 ──
  var curs = { x: 300, y: 300 }, quickAt = null;
  function openQuick() {
    if (!quick.hidden) { closeQuick(); return; }
    var w = window.innerWidth, h = window.innerHeight; quickAt = { x: Math.max(8, Math.min(w - 356, curs.x + 14)), y: Math.max(8, Math.min(h - 60, curs.y + 18)) };
    quick.style.left = quickAt.x + 'px'; quick.style.top = quickAt.y + 'px'; quick.hidden = false; answer.hidden = true;
    me.openedAt = Date.now(); quickIn.value = ''; quickIn.style.height = 'auto'; quickIn.focus(); if (T) T.window.getCurrentWindow().setFocus().then(function () { quickIn.focus(); });
  }
  function closeQuick() { quick.hidden = true; giveBack(); }
  function showAnswer(text, waiting) {
    answer.textContent = text; answer.classList.toggle('wait', !!waiting); answer.hidden = false;
    var r = answer.getBoundingClientRect(), w = window.innerWidth, h = window.innerHeight;
    var ax = Math.max(8, Math.min(w - r.width - 8, quickAt.x)), ay = Math.max(8, Math.min(h - r.height - 8, quickAt.y));
    if (!ticket.hidden) { var t = ticket.getBoundingClientRect(); if (ax < t.right && ax + r.width > t.left && ay < t.bottom && ay + r.height > t.top) { if (t.left - r.width - 10 >= 8) ax = t.left - r.width - 10; else ay = Math.max(8, t.top - r.height - 10); } }
    answer.style.left = ax + 'px'; answer.style.top = ay + 'px';
  }
  quick.addEventListener('submit', function (e) {
    e.preventDefault(); var t = quickIn.value.trim(); if (!t || me.busy) return;
    closeQuick(); showAnswer('……', true);                 // 问完键盘就还给你，回话到了再出现在这里
    send(t, function (text, failed) { showAnswer(text, failed); });
  });
  quickIn.addEventListener('keydown', function (e) {
    if (e.key === 'Escape') { e.stopPropagation(); closeQuick(); }
    else if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) { e.preventDefault(); quick.requestSubmit(); }
  });
  quickIn.addEventListener('input', function () {
    quickIn.style.height = 'auto'; quickIn.style.height = Math.min(quickIn.scrollHeight + 2, 160) + 'px';
    var r = quick.getBoundingClientRect(), h = window.innerHeight; if (r.bottom > h - 8) quick.style.top = Math.max(8, h - 8 - r.height) + 'px';     // 长高了别长出屏幕
  });
  answer.addEventListener('click', function () { answer.hidden = true; });
  answer.addEventListener('contextmenu', function (e) { e.preventDefault(); answer.hidden = true; if (talk.hidden) toggle(talk); });   // 右键：接着聊

  function collapse() {
    tray.hidden = true; map.hidden = true; if (!answer.classList.contains('wait') && quick.hidden) answer.hidden = true;
    var was = false;
    if (tk && tk.status !== 'waiting' && !me.busyTicket) { ticket.hidden = true; tk = null; }
    if (was) giveBack(); layout(); tuck();
  }

  peek.addEventListener('click', function () { me.unveilUntil = Date.now() + 20000; behave(); loadTicket(); });

  // 交给它的文件（拖到箱子上，或者从右键菜单交来）：打开说话的地方，路径已经填好，等你说要拿它做什么
  function given(paths) {
    if (!paths.length) return;
    untuck(8000); if (talk.hidden) toggle(talk);
    input.value = paths.map(function (p) { return '「' + p + '」'; }).join(' ') + ' ' + input.value; grow(); input.focus(); input.setSelectionRange(input.value.length, input.value.length);
    mood('peek', 'sit');
  }
  // 惦记：一张票等了一刻钟还没人理，提醒一次。只一次
  var nagged = {};
  setInterval(function () {
    if (!tk || tk.status !== 'waiting' || nagged[tk.id] || me.dnd || S.away || S.idle > 60) return;
    var m = /(\d{4})-(\d\d)-(\d\d)T(\d\d):(\d\d):(\d\d)/.exec(tk.at || ''); if (!m) return;
    if (Date.now() - new Date(+m[1], m[2] - 1, +m[3], +m[4], +m[5], +m[6]).getTime() > 15 * 60e3) { nagged[tk.id] = 1; untuck(8000); say1('那张票还等着你点头。', 6000); }
  }, 30000);

  // 一张票办完了（或者没办成），而它是刚才那件事里的一步：把结果告诉它，让它接着做剩下的。
  // 只接它在这件事里递的票（十分钟之内的），一件事最多这样接六次，免得没完没了
  function carryOn(t) {
    if (!T || !chain.session || chain.left <= 0 || t.status === 'undone') return;
    var m = /(\d{4})-(\d\d)-(\d\d)T(\d\d):(\d\d):(\d\d)/.exec(t.at || ''); if (!m || Date.now() - new Date(+m[1], m[2] - 1, +m[3], +m[4], +m[5], +m[6]).getTime() > 10 * 60e3) return;
    chain.left--;
    var words = '〔程序：你递的票「' + t.title + '」他点了"做"，结果：' + (t.status === 'done' ? '做完了' : '没做成') + (t.output ? '，输出：' + String(t.output).slice(0, 600) : '') + '。他刚才要你做的事要是还有剩下的步骤，现在接着做；没做成就换个办法或者照实说；都做完了就用一句话说结果。〕';
    var go = function () { if (me.busy) { setTimeout(go, 800); return; } if (chain.where === 'quick' && quickAt) showAnswer('……', true); send(words, chain.reply, { program: true, session: chain.session, where: chain.where }); };
    setTimeout(go, 900);
  }

  // ── 一句会自己淡掉的话 ──
  var bubbleTimer = null;
  // 闲话：时机由后端报上来，说不说由它自己定。两次之间至少隔二十分钟；勿扰、正在说话、有票等着的时候不起念头。
  // 点一下气泡是"听见了，挺好"；右键是"别说了"，之后两小时不再开口。
  var lastMuse = 0, hushUntil = 0, musing = false;
  function onOccasion(occasion) {
    var t = Date.now();
    if (!T || musing || me.dnd || me.busy || me.busyTicket || S.fullscreen || !talk.hidden || t < hushUntil || t - lastMuse < 20 * 60e3) return;
    musing = true; lastMuse = t;
    T.core.invoke('muse', { occasion: occasion }).then(function (line) { if (line && !me.dnd && talk.hidden) { say1(line, 9000); bubble.dataset.banter = '1'; } }, function () { /* 想不了就不说 */ }).then(function () { musing = false; });
  }
  bubble.addEventListener('click', function () { if (bubble.dataset.banter) { T.core.invoke('banter_reaction', { reaction: 'liked' }); delete bubble.dataset.banter; mood('joy', 'joy'); setTimeout(function () { mood('calm', 'sit'); }, 1800); } bubble.hidden = true; });
  bubble.addEventListener('contextmenu', function (e) { e.preventDefault(); if (bubble.dataset.banter) { T.core.invoke('banter_reaction', { reaction: 'shushed' }); delete bubble.dataset.banter; hushUntil = Date.now() + 2 * 3600e3; } bubble.hidden = true; });

  function say1(text, stay) {
    var who = cat.isOut() ? cat.kitty() : cat.box(); if (!who) return; var r = who.getBoundingClientRect();
    delete bubble.dataset.banter; bubble.textContent = text; bubble.classList.remove('fade'); bubble.hidden = false;
    var br = bubble.getBoundingClientRect(); bubble.style.left = Math.max(8, r.left - br.width + r.width * .3) + 'px'; bubble.style.top = Math.max(8, r.top - br.height + 6) + 'px';
    clearTimeout(bubbleTimer); bubbleTimer = setTimeout(function () { bubble.classList.add('fade'); bubbleTimer = setTimeout(function () { bubble.hidden = true; }, 1500); }, stay || 4000);
  }

  // ── 说话 ──
  function line(cls, text) { var p = document.createElement('p'); p.className = cls; p.textContent = text; log.appendChild(p); log.scrollTop = log.scrollHeight; layout(); }
  function openTalk() {
    if (me.pending) { line('ys', me.pending); me.pending = null; cat.count(0); if (T) T.core.invoke('opening_seen'); }
    me.openedAt = Date.now(); input.focus(); if (T) T.window.getCurrentWindow().setFocus();
  }
  function mood(inBox, outside) { if (cat.isOut()) { if (cat.idle()) cat.be(outside, 380); } else cat.pose(inBox); }
  // where：'talk' 是说话的地方那一场连着的对话；'quick' 是就地问的一句，每次新起一场。
  // opts.session：接着某一场往下说；opts.program：这一句是程序推的（票办完了），不是你说的
  var chain = { session: null, where: 'talk', reply: null, left: 0 };
  function send(text, reply, opts) {
    opts = opts || {};
    if (me.busy || !text) return;
    var where = opts.where || (reply ? 'quick' : 'talk');
    if (!opts.program) { chain.left = 6; chain.where = where; chain.reply = reply || null; }
    me.busy = true; input.disabled = true; if (!opts.program) line('me', text); mood('think', 'think');
    var ask = function () { return T.core.invoke('say', { text: text, fresh: where === 'quick' && !opts.session, session: opts.session || null, program: !!opts.program }); };
    var done = !T ? null : opts.program ? ask() : T.core.invoke('quick_launch', { text: text }).then(function (name) { return name ? { text: '打开了 ' + name + '。', noted: [] } : ask(); }, ask);
    done = done || Promise.resolve({ text: '（没有连上后端，这是界面的演示。）', noted: [] });
    done.then(function (r) {
      // 回话第一行的记号：它不赞成。记号不显示，猫把杯子碰倒
      var objects = /^\s*〔不同意〕\s*/.test(r.text); if (objects) r.text = r.text.replace(/^\s*〔不同意〕\s*/, '');
      if (r.session) chain.session = r.session;
      line('ys', r.text); if (reply) reply(r.text);
      if (objects) { if (cat.isOut()) cat.refuse(); else cat.pose('no'); setTimeout(function () { mood('calm', 'sit'); }, 5000); return; }
      if (r.noted && r.noted.length) line('kept', '记下了：' + r.noted.map(function (f) { return f.split('memory/').join(''); }).join('、'));
      mood('calm', 'sit');
    }, function (e) { line('err', '我现在想不了。' + String(e)); if (reply) reply('我现在想不了。', true); mood('oops', 'oops'); setTimeout(function () { mood('calm', 'sit'); }, 4000); })
      .then(function () { me.busy = false; input.disabled = false; layout(); if (!talk.hidden) input.focus(); });
  }
  function grow() { input.style.height = 'auto'; input.style.height = Math.min(input.scrollHeight + 3, 110) + 'px'; layout(); }
  form.addEventListener('submit', function (e) { e.preventDefault(); var t = input.value.trim(); input.value = ''; grow(); send(t); });
  // 回车发送，Shift+回车换行。输入法正在选字时的回车不算。
  input.addEventListener('keydown', function (e) { if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) { e.preventDefault(); form.requestSubmit(); } });
  input.addEventListener('input', grow);
  document.addEventListener('keydown', function (e) { if (e.key === 'Escape') { tray.hidden = true; map.hidden = true; if (!talk.hidden) { talk.hidden = true; giveBack(); } } });

  // ── 哪几块是实心的：告诉后端，其余地方的点击穿过去 ──
  var lastHit = '';
  setInterval(function () {
    var list = [], add = function (e, shrink) { if (!e || e.hidden || e.classList.contains('gone')) return; var r = e.getBoundingClientRect(); if (r.width < 2) return; var dx = r.width * (shrink || 0), dy = r.height * (shrink || 0); list.push({ x: r.left + dx, y: r.top + dy, w: r.width - 2 * dx, h: r.height - dy }); };
    if (!document.body.classList.contains('veil')) { add(cat.box(), .06); if (cat.isOut()) add(cat.kitty(), .14); add(tray); add(talk); add(ticket); add(bubble); add(map); add(quick); add(answer); if (!paws.hidden) add(paws); Array.prototype.forEach.call(marksEl.querySelectorAll('.clip, .mark-say'), function (e) { add(e); }); }
    add(peek);
    var s = JSON.stringify(list); if (s !== lastHit) { lastHit = s; if (T) T.core.invoke('set_hit_regions', { list: list }); }
  }, 120);

  function save() { if (T) T.core.invoke('ui_state_save', { json: JSON.stringify({ bx: me.bx, by: me.by, dnd: me.dnd, size: me.size, outline: me.outline }) }); }
  function onMenu(id) {
    if (id === 'talk') toggle(talk);
    else if (id === 'quick') openQuick();
    else if (id === 'map') { if (map.hidden) toggle(map); }
    else if (id === 'home') { if (cat.isOut()) home('calm'); }
    else if (id === 'dnd') { me.dnd = !me.dnd; save(); }
    else if (/^size-[sml]$/.test(id)) { me.size = id.slice(5); dress(); save(); if (cat.isOut() && cat.idle()) cat.settle(); }
    else if (/^outline-[012]$/.test(id)) { me.outline = id.slice(8); dress(); save(); }
  }
  window.addEventListener('resize', function () { measure(); layout(); });

  // ── 启动 ──
  if (T) {
    T.core.invoke('ui_state_load').then(function (j) { try { var o = JSON.parse(j || 'null'); if (o) { me.bx = o.bx; me.by = o.by; me.dnd = !!o.dnd; if (SIZES[o.size]) me.size = o.size; if (o.outline in OUTLINES) me.outline = String(o.outline); dress(); } } catch (e) { /* 第一次启动没有存档 */ } })
      .then(function () { return T.window.getCurrentWindow().show(); })
      .then(giveBack)
      .then(function () {
        T.event.listen('scene', function (e) { onScene(e.payload); });
        T.event.listen('outside', collapse);
        // 后端说鼠标松开了，猫却还是被拎着的样子：松手的那一下丢了，替它补上
        T.event.listen('released', function () { setTimeout(drop, 90); });
        T.event.listen('cursor', function (e) { curs = e.payload; });
        T.event.listen('handoff', function (e) { given([String(e.payload)]); });
        T.event.listen('tauri://drag-drop', function (e) { given((e.payload && e.payload.paths) || []); });
        T.event.listen('occasion', function (e) { onOccasion(String(e.payload)); });
        T.event.listen('menu', function (e) { onMenu(String(e.payload)); });
        T.core.invoke('opening').then(function (text) { if (text) { me.pending = text; if (cat.box()) cat.count(1); } });
      });
  } else {
    // 不带后端的预览：造一份假的"此刻"，用来在浏览器里截图检查
    document.body.style.background = '#C9D3CF';
    var fake = { screen: { left: 0, top: 0, right: 1440, bottom: 960, floor: 920 }, ledges: [{ app: 'code', y: 300, x0: 120, x1: 900, fore: true }, { app: 'firefox', y: 430, x0: 980, x1: 1380, fore: false }], fore: 'VS Code', fullscreen: false, idle: 0, away: false, now: ['VS Code', 'SOUL', Date.now() / 1000 - 60], mem: 81 };
    onScene(fake);
    var h = location.hash;
    if (/tray/.test(h)) toggle(tray);
    if (/talk/.test(h)) { toggle(talk); line('me', '昨晚我弄到哪了？'); line('ys', '昨晚停在表单校验，第三版能跑了，测试还红着两个。'); layout(); }
    if (/out/.test(h)) cat.travel(84, S.ledges[0].y, 'sit');
    if (/ticket/.test(h)) { cat.travel(84, S.ledges[0].y, 'ask'); showTicket({ id: 't1', title: '关掉开机时自动启动的“某更新助手”', place: '注册表 HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run', script: "Remove-ItemProperty -Path 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Run' -Name 'UpdateHelper'", undo: 'reg import backup.reg', elevated: false, status: 'waiting', output: '' }, 2); }
    if (/map/.test(h)) { toggle(map); drawMap({ private: 16, rows: [{ path: 'D:\\SOUL', what: '我自己', see: true, put: false, edit: false, drop: false, run: false, source: '我猜的，还没问' }, { path: 'D:\\my-blog', what: '博客', see: true, put: true, edit: true, drop: false, run: false, source: '他说的 2026-10-12' }, { path: 'C:\\Windows', what: '系统', see: true, put: false, edit: false, drop: false, run: false, source: '我猜的，还没问' }] }); }
    if (/paws/.test(h)) drawPaws([{ status: 'done', title: '关掉更新助手的自启动', place: '注册表', undo: 'x', script: 'x', output: '好了' }, { status: 'failed', title: '停掉某服务', place: '服务', undo: '', script: 'x', output: '拒绝访问' }, { status: 'undone', title: '改 hosts', place: 'C:\\Windows', undo: 'x', script: 'x', output: '' }]);
    if (/marks/.test(h)) { fake.fore_app = 'code'; fake.marks = [{ id: 'a', kind: 'tag', app: 'code', text: '测试还红着两个，都在 form.test.ts', rect: [0, 0, 0, 0] }, { id: 'b', kind: 'ring', app: '', text: '少了个分号', rect: [300, 480, 520, 520] }, { id: 'c', kind: 'note', app: '', text: '这一段和第三节说重了', rect: [200, 600, 700, 700] }, { id: 'd', kind: 'tag', app: 'firefox', text: '这页上回看到一半', rect: [0, 0, 0, 0] }]; }
    if (/crowd/.test(h)) { toggle(talk); for (var i = 0; i < 9; i++) { line('me', '第 ' + i + ' 句：你不能看是指你不能主动查看，还是要以我的要求为准'); line('ys', '你说得对，以你的要求为准。私人是说我不主动去翻，你点名发过来的那一张，我该直接想办法看。'); } showTicket({ id: 't9', title: '把你发来的那张图复制一份到 local', place: 'D:\\SOUL\\local', script: 'Copy-Item a b', undo: 'Remove-Item b', elevated: false, status: 'waiting', output: '' }, 1); drawPaws([{ status: 'done', title: 'a', place: '', undo: '', script: 'x', output: '' }, { status: 'failed', title: 'b', place: '', undo: '', script: 'x', output: '' }]); }
    if (/quick/.test(h)) { curs = { x: 520, y: 380 }; openQuick(); if (/answer/.test(h)) { quick.hidden = true; showAnswer('昨晚停在表单校验，第三版能跑了，测试还红着两个。'); } }
    if (/peek/.test(h)) { fake.fullscreen = true; fake.waiting = 1; }
    var sz = /size-([sml])/.exec(h); if (sz) { me.size = sz[1]; } var ol = /ol-([012])/.exec(h); if (ol) { me.outline = ol[1]; } if (sz || ol) { dress(); document.body.style.background = '#2A2630'; }
    if (/say/.test(h)) say1('内存用到 93% 了，我有点喘。', 60000);
    setInterval(function () { fake.now[2] = Date.now() / 1000 - 60; onScene(fake); }, 700);
  }
})();
