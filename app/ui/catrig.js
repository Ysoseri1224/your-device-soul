// 由 app/tools/extract-cat.mjs 从 design/ysoul-surface-lab.html 生成，不要手改。
// 用法：var cat = window.YSOUL_CAT(舞台元素); cat.config({...}); cat.summon(x, y, 'calm');
window.YSOUL_CAT = function (dsk) {
  'use strict';
  var SKIN = window.YSOUL_SKIN, NS = 'http://www.w3.org/2000/svg';
  var box = null, kitty = null, out = false, timers = [], roam = null;
  var SLOT = { no: 'cross', ask: 'ring' };
  function el(tag, cls, html) { var e = document.createElement(tag); if (cls) e.className = cls; if (html != null) e.innerHTML = html; return e; }
  function put(e, x, y, w, h) { e.style.left = x + '%'; e.style.top = y + '%'; if (w) e.style.width = w + '%'; if (h) e.style.height = h + '%'; dsk.appendChild(e); return e; }
  function later(ms, fn) { timers.push(setTimeout(fn, ms)); }
  function clear() { timers.forEach(clearTimeout); timers = []; clearInterval(roam); roam = null; Array.prototype.slice.call(dsk.children).forEach(function (c) { if (!c.classList.contains('bar')) c.remove(); }); box = kitty = null; out = false; }
  function rel(e) { var a = e.getBoundingClientRect(), d = dsk.getBoundingClientRect(); return { x: (a.left - d.left) / d.width * 100, y: (a.top - d.top) / d.height * 100, w: a.width / d.width * 100, h: a.height / d.height * 100 }; }

  // ── 箱子 ──
  function art(state, cls) {
    var svg = el('div', '', SKIN.cat).querySelector('svg');
    svg.setAttribute('class', 's-' + state + (cls ? ' ' + cls : '')); svg.setAttribute('viewBox', '14 24 180 152'); svg.setAttribute('role', 'img');
    Array.prototype.forEach.call(svg.querySelectorAll('[data-slot]'), function (s) {
      var kind = s.getAttribute('data-fixed') || SLOT[state]; if (!kind) return;
      var use = document.createElementNS(NS, 'use'); use.setAttribute('href', '#pic-' + kind); s.appendChild(use);
    });
    return svg;
  }
  function pose(state) { var badge = box.querySelector('.count'); box.textContent = ''; box.appendChild(out ? art('joy', 'empty') : art(state || 'calm')); if (badge) box.appendChild(badge); }
  function count(n) { var b = box.querySelector('.count'); if (!n) { if (b) b.remove(); return; } if (!b) { b = el('span', 'count'); box.appendChild(b); } b.textContent = n; }
  // ── 拖动 ──
  function grab(node, opts) {
    var g = null;
    node.addEventListener('pointerdown', function (e) { var r = rel(node), d = dsk.getBoundingClientRect(); g = { dx: e.clientX - d.left - r.x / 100 * d.width, dy: e.clientY - d.top - r.y / 100 * d.height, moved: false }; node.setPointerCapture(e.pointerId); e.preventDefault(); });
    node.addEventListener('pointermove', function (e) {
      if (!g) return; var d = dsk.getBoundingClientRect();
      if (!g.moved) { g.moved = true; if (opts.start) opts.start(); }
      var px = (e.clientX - d.left) / d.width * 100, py = (e.clientY - d.top) / d.height * 100;
      if (opts.drag) { opts.drag(px, py); return; }
      var a = node.getBoundingClientRect();
      node.style.left = Math.max(-4, Math.min(100 - a.width / d.width * 60, px - g.dx / d.width * 100)) + '%';
      node.style.top = Math.max(-6, Math.min(88, py - g.dy / d.height * 100)) + '%';
    });
    node.addEventListener('pointerup', function () { var m = g && g.moved; g = null; if (m) { if (opts.end) opts.end(); } else if (opts.tap) opts.tap(); });
  }
  function summon(x, y, state) {
    box = el('div', 'box'); box.tabIndex = 0; box.setAttribute('aria-label', 'ysoul 的箱子，可以拖动'); put(box, x, y); pose(state);
    grab(box, { tap: function () { if (box.ontap) box.ontap(); }, end: function () { if (box.onmoved) box.onmoved(); } });
    kitty = el('div', 'kitty gone', KIT); dsk.appendChild(kitty); cup = el('div', 'cupx'); cup.hidden = true; dsk.appendChild(cup); rigInit();
    dsk.onclick = function (e) { if (!rig || kitty.classList.contains('held') || e.target.closest('button,.pop,.kitty,.box,.tag')) return; var d = dsk.getBoundingClientRect(), px = (e.clientX - d.left) / d.width * 100, h = e.target.closest('.wnd > header'), b = e.target.closest('.bar');
      if (h) { var r = rel(h.parentNode); if (r.y > 6) travel(px, r.y + .5, 'sit'); } else if (b) travel(px, FLOOR, 'sit'); };
    grab(kitty, {
      start: function () { rig.move = null; rig.queue = []; rig.turning = false; kitty.classList.add('held'); rig.phi = 0; rig.om = 0; rig.psi = 0; rig.pom = 0; rig.pvx = 0; rig.hx = rig.fx; rig.hy = rig.gy - 132 * UY; be('held', 220); },
      // 指针捏着的是后颈。后颈在哪，猫就从哪儿挂下来
      drag: function (px, py) { rig.hx = Math.max(2, Math.min(98, px)); rig.hy = Math.max(4, Math.min(92, py)); },
      end: function () { kitty.classList.remove('held'); settle(); },
      tap: function () { if (!idle()) return; be('joy', 260); later(2600, function () { if (rig.name === 'joy') be('sit', 460); }); }
    });
    return box;
  }

  // ════ 箱子外面的猫 ════
  // 一副照着真猫结构搭的骨架，比例是卡通的。
  //   身子：肩 S、腰 M、胯 H 三点定出脊背；背和腹的厚度不一样：胸深、腹线往后收、肩上微隆、臀圆。
  //   前腿：上臂和肘藏在胸的轮廓里，露在外面的是从胸下到地面的一根直柱子，迈步时像钟摆一样摆，只有离地时腕才弯。
  //   后腿：大腿、小腿、脚掌三节。膝朝前，跗关节朝后且离地很高，脚掌很长，只有脚尖着地。
  //   头：侧着时是侧脸，正对你时是定过的那张正脸。
  // 每个姿态是一组数。换姿态时所有的数一起从旧值滑到新值，所以任意两个姿态之间都有连续的过渡帧。
  // 坐标：地面是 y=0，往上为负，侧身时脸朝右。fz：身子正对你的程度；face：脸正对你的程度。
  var VB = [-125, -190, 250, 215], KW = 9.8, KH = KW * VB[3] / VB[2] * 1.6, UX = KW / VB[2], UY = KH / VB[3];
  var LT = 22, LS = 25, LM = 24, G = 330, FLOOR = 95;   // FLOOR：任务栏上沿在屏幕高度的百分之几。样稿里是 95，真桌面上由程序量出来
  var _unused_ = 0;   // 大腿、小腿、脚掌的长度；重力（桌面高度的百分比每秒方），猫、杯子、下落都用这一个
  var eyeSvg = function (cx, cy, r, clip) { return '<circle cx="' + cx + '" cy="' + cy + '" r="' + r + '" fill="var(--white)"/><g class="pupil"><circle cx="' + cx + '" cy="' + cy + '" r="' + (r * .72).toFixed(2) + '" fill="var(--ochre)"/><circle cx="' + cx + '" cy="' + cy + '" r="' + (r * .42).toFixed(2) + '" fill="var(--cat)"/><circle cx="' + (cx + r * .26).toFixed(2) + '" cy="' + (cy - r * .26).toFixed(2) + '" r="' + (r * .17).toFixed(2) + '" fill="var(--white)"/></g><g clip-path="url(#' + clip + ')"><rect class="lidE" x="' + (cx - r - 2) + '" y="' + (cy - r) + '" width="' + (2 * r + 4) + '" height="' + (2 * r + 4) + '" fill="var(--cat)"/></g>'; };
  var HEADF = '<g transform="scale(.82)"><path d="M-17 -31 L-40 -56 Q-47 -30.5 -46 -9 C-49 19.2 -26.68 38 0 38 C26.68 38 49 19.2 46 -9 Q47 -30.5 40 -56 L17 -31 Q0 -28.5 -17 -31 Z" fill="var(--cat)" stroke="var(--cat)" stroke-width="5" stroke-linejoin="round"/><path d="M-35.5 -41 L-25 -30 L-39 -18 Z" fill="var(--clay)"/><path d="M35.5 -41 L25 -30 L39 -18 Z" fill="var(--clay)"/>' +
    eyeSvg(-19.5, 2, 12.5, 'eyeL') + eyeSvg(19.5, 2, 12.5, 'eyeR') + '<path class="kq-es" transform="translate(-19.5 2)" d="M0 -11 Q0 0 11 0 Q0 0 0 11 Q0 0 -11 0 Q0 0 0 -11 Z" fill="var(--white)"/><path class="kq-es" transform="translate(19.5 2)" d="M0 -11 Q0 0 11 0 Q0 0 0 11 Q0 0 -11 0 Q0 0 0 -11 Z" fill="var(--white)"/>' +
    '<path class="kq-shut" d="M-26 3 q6.5 5 13 0 M13 3 q6.5 5 13 0" fill="none" stroke="var(--ochre)" stroke-width="2.6" stroke-linecap="round"/></g>';
  var HEADS = '<path d="M9 -29 L20 -54 L30 -20 Z" fill="var(--cat)" stroke="var(--cat)" stroke-width="4" stroke-linejoin="round"/>' +
    '<path d="M-31 3 C-33 -19 -16 -34 5 -33 C19 -32 29 -22 32 -8 L39 3 C39 11 31 18 22 21 C8 27 -11 27 -23 20 C-28 16 -31 10 -31 3 Z" fill="var(--cat)" stroke="var(--cat)" stroke-width="4" stroke-linejoin="round"/>' +
    '<path d="M-14 -27 L-7 -59 L15 -30 Z" fill="var(--cat)" stroke="var(--cat)" stroke-width="4" stroke-linejoin="round"/><path d="M-8 -31 L-5 -50 L8 -32 Z" fill="var(--clay)"/>' +
    eyeSvg(15, -6, 10.5, 'kqeye') + '<path class="kq-shut" d="M8 -5 q7 5 14 0" fill="none" stroke="var(--ochre)" stroke-width="2.6" stroke-linecap="round"/>';
  var SEAM = 'fill="none" stroke="var(--paper)" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" clip-path="url(#kqclip)"';
  var BANG = '<g class="kq-bang" fill="var(--clay)"><rect x="-3.2" y="-22" width="6.4" height="14" rx="3.2"/><circle cy="-2" r="3.5"/></g>';
  var KIT = '<svg viewBox="' + VB.join(' ') + '" role="img" aria-label="ysoul"><defs><clipPath id="kqclip"><path class="kq-clip"/></clipPath><clipPath id="kqeye"><circle cx="15" cy="-6" r="10.5"/></clipPath></defs><g class="kq-all">' +
    '<path class="kq-tail" fill="var(--cat)"/>' +
    '<g class="kq-far"><path class="kq-hl"/><ellipse class="kq-phl"/><path class="kq-fl"/><ellipse class="kq-pfl"/></g>' +
    '<path class="kq-body" fill="var(--cat)"/><path class="kq-scruff" fill="var(--cat)"/><path class="kq-neck" fill="var(--cat)"/>' +
    '<g fill="var(--cat)"><path class="kq-hr"/><ellipse class="kq-phr"/><path class="kq-fr"/><ellipse class="kq-pfr"/></g>' +
    '<path class="kq-seam" ' + SEAM + '/><path class="kq-seam2" ' + SEAM + '/>' +
    '<g class="kq-ball"><g transform="scale(1.35)"><circle r="8.5" fill="var(--clay)"/><path d="M-6 -3 Q0 -8 6 -3 M-7.5 2 Q0 -3 7.5 2 M-4 6.5 Q1 2 6 5" fill="none" stroke="var(--ink)" stroke-width="1.2" stroke-linecap="round" opacity=".5"/></g></g>' +
    '<g class="kq-head"><g class="kq-hs">' + HEADS + '</g><g class="kq-hf">' + HEADF + '</g></g>' +
    '<g class="kq-card"><rect x="-19" y="-19" width="38" height="38" rx="4" fill="var(--wash)" stroke="var(--ink)" stroke-width="3" transform="rotate(8)"/><circle r="9.5" fill="none" stroke="var(--clay)" stroke-width="5"/></g>' +
    BANG + BANG + BANG + Array(6).join('<path class="kq-star" d="M0 -6.5 Q0 0 6.5 0 Q0 0 0 6.5 Q0 0 -6.5 0 Q0 0 0 -6.5 Z" fill="var(--ochre)"/>') + '</g></svg>';

  // 站着（侧面）是底子，其余姿态只写和它不同的地方。
  // fr fl：右、左前脚尖的落点；hr hl：后脚尖的落点（侧面时"右"是靠你这边的）。lean：两只后脚掌各自往后斜多少度（0 是竖直踮着，90 是平放在地上）。
  // pf：两只前爪离地时往下耷拉的角度。tuck：侧面轮廓里"胸深腹收"的程度。t、tr：尾巴曲线和尾根。
  var KP = {
    stand: { S: [26, -64], M: [-4, -67], H: [-34, -62], rs: 20, rm: 20.5, rh: 21, tuck: 1, C: [54, -80], ha: 0, face: 0, fz: 0, fo: 0, ho: 0,
      fr: [31, 0], fl: [24, 0], hr: [-31, 0], hl: [-40, 0], pf: [0, 0], lean: [14, 14],
      tr: [-18, -9], t: [-68, -78, -90, -104, -70, -128], tw: [6.2, 3.2], lid: -30, gx: 0, gy: 0, ps: 1, fur: 0, hx: 0, sm: 1, ball: [0, 0] },
    // 正面站着：只在转身时路过
    front: { S: [0, -64], M: [0, -64], H: [0, -62], rs: 24, rm: 24, rh: 25, tuck: 0, C: [0, -100], face: 1, fz: 1, fo: 10, ho: 13, fr: [10, 0], fl: [-10, 0], hr: [15, 0], hl: [-15, 0], tr: [6, -16], t: [22, -84, 30, -124, 14, -146] },
    // 坐着：正对你。身子上窄下宽像个钟，两条前腿并着立在身前；后腿收在身子两侧，只露出脚尖；尾巴从身后绕到一侧
    sit: { S: [0, -66], M: [0, -46], H: [0, -30], rs: 21, rm: 29, rh: 33, tuck: 0, C: [0, -104], face: 1, fz: 1, fo: 9, ho: 0, fr: [9, 0], fl: [-9, 0], hr: [25, 0], hl: [-25, 0], lean: [80, 80], tr: [24, 22], t: [58, 0, 66, -32, 44, -42] },
    // 蜷着睡：身子横过来拱成一团，脸朝你搁在身前，尾巴绕到脸下面
    loaf: { S: [28, -27], M: [-2, -66], H: [-32, -29], rs: 26, rm: 32, rh: 30, tuck: 0, C: [24, -31], ha: 9, face: 1, fr: [40, -3], fl: [30, -3], hr: [-8, -3], hl: [-14, -3], lean: [86, 86],
      tr: [-20, 12], t: [-74, -2, -28, 8, 30, -3], lid: 0, sm: 0 },
    // 起跳前、落地后的那一蹲
    crouch: { S: [26, -46], M: [-2, -55], H: [-32, -40], rh: 23, C: [54, -60], ha: 6, lean: [34, 34], tr: [-18, -6], t: [-68, -38, -90, -32, -102, -42] },
    // 腾空：前腿往前伸，后腿往后蹬，尾巴拖在后面
    air: { S: [32, -66], M: [0, -76], H: [-34, -60], C: [64, -78], ha: -8, fr: [72, -52], fl: [64, -45], hr: [-76, -32], hl: [-68, -25], pf: [-18, -18], lean: [70, 70], t: [-70, -68, -98, -60, -118, -70] },
    // 干活（踩奶）：稳稳地坐在后腿上，脚掌平放在地，重心在臀；两只前爪轮流往下按
    work: { S: [20, -56], M: [-2, -50], H: [-24, -24], rs: 20, rm: 22, rh: 26, tuck: .6, C: [42, -70], ha: 22, gx: 1, gy: 4, lid: -21, fr: [25, 0], fl: [17, 0], hr: [0, 0], hl: [-6, 0], lean: [84, 84],
      tr: [-22, 10], t: [-66, -4, -84, -30, -66, -48] },
    // 出错：正面伏低成一团
    oops: { S: [0, -42], M: [0, -36], H: [0, -27], rs: 26, rm: 32, rh: 35, tuck: 0, C: [0, -74], face: 1, fz: 1, fo: 11, ho: 0, fr: [11, 0], fl: [-11, 0], hr: [27, 0], hl: [-27, 0], lean: [80, 80], tr: [26, 18], t: [56, 4, 42, 6, 28, 2], ps: .58, gy: 2 },
    // 炸毛：背高高弓起，四腿绷直并拢踮高，尾巴竖起炸开
    hot: { S: [22, -68], M: [0, -132], H: [-22, -66], rs: 19, rm: 20, rh: 20, tuck: .3, C: [48, -62], ha: 16, fr: [26, 0], fl: [20, 0], hr: [-17, 0], hl: [-24, 0], lean: [4, 4], tr: [-13, -12], t: [-40, -112, -48, -150, -36, -182], tw: [10, 8], ps: .5, fur: 1 },
    // 高兴：四脚站稳，尾巴笔直竖起
    joy: { C: [54, -84], ha: -6, t: [-50, -104, -54, -148, -44, -172] },
    // 攀爬：身子竖着贴在窗口的侧边，脸朝上看，前爪往上够，后脚蹬着
    climb: { S: [4, -84], M: [-12, -54], H: [2, -26], rs: 19, rm: 20, rh: 21, tuck: .4, C: [16, -108], ha: -34, fr: [24, -104], fl: [20, -92], hr: [22, -6], hl: [20, 6], pf: [-50, -50], lean: [50, 50], tr: [-14, 10], t: [-24, 4, -30, 30, -18, 46], gy: -3 },
    // 被拎着后颈：脊背微弓，头往前探着微微耷拉，前腿悬在身前，后腿松着垂下，爪子蜷着，尾巴夹在两条后腿之间
    held: { S: [0, -112], M: [-22, -78], H: [2, -42], rs: 19, rm: 20, rh: 21, tuck: .5, C: [30, -118], ha: 26, fr: [30, -66], fl: [22, -60], hr: [20, 18], hl: [10, 22], pf: [70, 70], lean: [28, 28],
      tr: [-4, 18], t: [0, -2, 12, 14, 22, -8], lid: -20, gy: 3, hx: 1, sm: .5 }
  };
  function K(name) { var p = JSON.parse(JSON.stringify(KP.stand)), s = KP[name]; for (var k in s) p[k] = JSON.parse(JSON.stringify(s[k])); return p; }
  function lerp(a, b, w) { return typeof a === 'number' ? a + (b - a) * w : a.map(function (v, i) { return lerp(v, b[i], w); }); }
  function blend(a, b, w) { var o = {}; for (var k in b) o[k] = lerp(a[k], b[k], w); return o; }
  function ease(u) { u = Math.max(0, Math.min(1, u)); return u * u * (3 - 2 * u); }
  // 尾巴的摆动：根部不动，越往尖上摆得越大、越滞后
  function wag(p, t, speed, amp) { var a = Math.sin(t * speed), b = Math.sin(t * speed - .9); p.t[2] += a * amp * .45; p.t[4] += b * amp; p.t[5] += Math.abs(b) * amp * .25; return p; }

  // 一条腿在一步里的轨迹。q 是这条腿在整个步子里走到哪了（0 到 1）。
  // 前 62% 脚尖踩在地上不动（相对身子匀速往后）；后 38% 离地往前迈。
  function step(p, key, q, len, lift, hindIdx) {
    q = ((q % 1) + 1) % 1; var x0 = p[key][0];
    if (q < .62) { var s0 = q / .62; p[key] = [x0 + len / 2 - len * s0, 0]; if (hindIdx != null) p.lean[hindIdx] = 6 + 22 * s0; }
    else { var s = (q - .62) / .38, e = ease(s); p[key] = [x0 - len / 2 + len * e, -lift * Math.sin(Math.PI * s)];
      if (hindIdx != null) p.lean[hindIdx] = 28 + 30 * Math.sin(Math.PI * s) - 22 * s; else p.pf[key === 'fr' ? 0 : 1] = 62 * Math.sin(Math.PI * Math.min(1, s * 1.3)); }
  }
  var GAIT = { walk: { T: .5, len: 38, lift: 8 }, trot: { T: .3, len: 66, lift: 13 }, run: { T: .24, len: 96, lift: 16 } };
  function gait(kind, t) {
    var g = GAIT[kind], p = K('stand'), q = t / g.T, trot = kind !== 'walk';
    // 走：右后、右前、左后、左前依次落地，任何时候至少三只脚在地上。小跑：对角的两条腿一起动。
    step(p, 'hr', q, g.len, g.lift, 0); step(p, 'fr', q + (trot ? .5 : .75), g.len, g.lift);
    step(p, 'hl', q + .5, g.len, g.lift, 1); step(p, 'fl', q + (trot ? 0 : .25), g.len, g.lift);
    var w = q * Math.PI * 2, b = trot ? 2 : 1.1;
    p.S[1] += Math.sin(w * 2 + .6) * b; p.H[1] += Math.sin(w * 2 + 2.2) * b; p.C[1] += Math.sin(w * 2 + .2) * b * .8; p.C[0] += Math.sin(w * 2 + 1.2) * 1.2; p.ha = Math.sin(w * 2) * 1.5;
    return wag(p, t, Math.PI * 2 / g.T, trot ? 6 : 4);
  }

  // 每个状态：给一个时间（秒），返回那一刻的姿态。
  var GEN = {
    stand: function (t) { return wag(K('stand'), t, 2.2, 4); },
    front: function () { return K('front'); },
    sit: function (t) { var p = K('sit'), b = Math.sin(t * 1.5); p.rm += b * .5; p.C[1] += b * .5; return wag(p, t, 1.5, 4); },
    walk: function (t) { return gait('walk', t); },
    trot: function (t) { return gait('trot', t); },
    run: function (t) { var p = gait('run', t); p.C[0] += 4; p.C[1] += 3; p.ha = 6; p.t[5] += 22; p.t[3] += 14; return p; },
    loaf: function (t) { var p = K('loaf'), b = Math.sin(t * 1.25); p.rm += b * 1.5; p.M[1] -= b * 1.2; p.C[1] -= b * .4; return p; },
    crouch: function () { return K('crouch'); },
    climb: function (t) { var p = K('climb'), w = t * 10, a = Math.sin(w), b = Math.sin(w + Math.PI); p.fr[1] += a * 9; p.fl[1] += b * 9; p.hr[1] += b * 7; p.hl[1] += a * 7; p.S[1] += Math.abs(a) * 1.5; p.C[1] += Math.abs(a) * 1.5; p.t[4] += a * 3; return p; },
    air: function () { return K('air'); },
    // 干活：坐着，正对你。毛球在两只前爪之间来回滚，滚到哪边，哪边的爪子就抬起来把它拨回去；眼睛一直跟着球
    work: function (t) {
      var p = K('sit'), T = 1.5, q = (((t / T) % 1) + 1) % 1, half = q < .5 ? q * 2 : (q - .5) * 2, e = 1 - (1 - half) * (1 - half), bx = q < .5 ? -14 + 28 * e : 14 - 28 * e;
      var near = function (side) { return Math.max(0, 1 - Math.abs(bx - side * 14) / 9); }, r = near(1), l = near(-1);
      p.fo = 11; p.fr = [19 - 4 * r, -7 * r]; p.fl = [-19 + 4 * l, -7 * l]; p.pf = [-20 * r, -20 * l]; p.ball = [bx, bx * 6.5];
      p.gx = bx / 14 * 3.4; p.gy = 4.2; p.ha = bx / 14 * 4; p.C[0] += bx / 14 * 1.5; p.C[1] += 3; p.S[1] += 1.5; p.lid = -24;
      return wag(p, t, 2.6, 5);
    },
    think: function (t) { var p = K('sit'); p.ha = -11; p.gx = -3; p.gy = -4; p.t[4] += Math.sin(t * 6.5) * 4; p.t[5] += Math.cos(t * 6.5) * 3; return p; },
    ask: function (t) { var p = K('sit'); p.fr = [26, -52 + Math.sin(t * 2.1) * 2]; p.pf[0] = -30; return wag(p, t, 1.5, 4); },
    // 不同意：眯着眼，伸一只爪子去拨杯子。杯子怎么动由物理决定，不在这里写
    no: function (t) { var p = K('sit'), u = t % 5.5, r = u < .5 ? 0 : u < .95 ? ease((u - .5) / .45) : u < 1.5 ? 1 : u < 2.1 ? 1 - ease((u - 1.5) / .6) : 0; p.lid = -14; p.gx = 3.8; p.gy = 2.6; p.fr = [9 + r * 44, -r * 6]; p.pf[0] = -r * 22; p.S[0] += r * 4; p.S[1] += r * 3; p.C[0] += r * 3; p.C[1] += r * 2; p.ha = r * 5; p.t[4] += Math.sin(t * 8) * 4; return p; },
    // 炸毛：整只猫绷着发抖，侧着身子小步弹
    hot: function (t) { var p = K('hot'), j = Math.sin(t * 44) * .8, hop = Math.max(0, Math.sin(t * 5.2)) * 3; ['fr', 'fl', 'hr', 'hl'].forEach(function (k) { p[k][1] -= hop; }); p.S[1] -= hop; p.H[1] -= hop; p.M[1] -= hop; p.C[1] -= hop; p.M[0] += j; p.C[0] += j; p.t[4] += Math.sin(t * 31) * 1.6; return p; },
    // 高兴：坐着，正对你。头左右轻轻晃，身子跟着一颠一颠，尾巴甩得欢；眼睛睁圆，眼里和头边冒小星星（星星在 draw 里画）
    joy: function (t) { var p = K('sit'), w = t * 5.2, s = Math.sin(w); p.ha = s * 7; p.C[0] += s * 2.5; p.C[1] -= Math.abs(s) * 2.6; p.S[0] += s * 1.2; p.S[1] -= Math.abs(s) * 1.2; p.rm += Math.abs(s) * .6; p.ps = 1.14; return wag(p, t, 8, 8); },
    oops: function (t) { var p = K('oops'), d = Math.exp(-t * 5) * Math.sin(t * 30) * 2.5; p.C[1] += d; p.S[1] += d * .5; return p; },
    // 被拎着：整只猫怎么摆是钟摆算出来的（见 tick）。这里只让腿和尾巴比身子再慢半拍
    held: function () { var p = K('held'), l = rig ? (rig.psi - rig.phi) * 46 : 0; p.fr[0] += l * .7; p.fl[0] += l * .7; p.hr[0] += l; p.hl[0] += l; p.t[2] += l * .6; p.t[4] += l; return p; }
  };
  var SIDE = { held: 1, loaf: 1, hot: 1, stand: 1 };
  var FRONTAL = { sit: 1, think: 1, ask: 1, no: 1, oops: 1, front: 1, work: 1, joy: 1 };

  // ── 几何 ──
  function two(R, F, a, b, dir) {
    var dx = F[0] - R[0], dy = F[1] - R[1], d = Math.hypot(dx, dy) || .01, m = Math.max(Math.abs(a - b) + .05, Math.min(d, a + b - .05)), base = Math.atan2(dy, dx), A = Math.acos((a * a + m * m - b * b) / (2 * a * m)), g = base + dir * A;
    return [R[0] + Math.cos(g) * a, R[1] + Math.sin(g) * a];
  }
  function cr(a, b, c, d, t) { return .5 * (2 * b + (-a + c) * t + (2 * a - 5 * b + 4 * c - d) * t * t + (-a + 3 * b - 3 * c + d) * t * t * t); }
  function spline(pts, rad, n) {
    var P = [], R = [], m = pts.length - 1;
    for (var i = 0; i <= n; i++) { var u = i / n * m, k = Math.min(m - 1, Math.floor(u)), f = u - k, a = pts[Math.max(0, k - 1)], b = pts[k], c = pts[k + 1], d = pts[Math.min(m, k + 2)];
      P.push([cr(a[0], b[0], c[0], d[0], f), cr(a[1], b[1], c[1], d[1], f)]); R.push(rad[k] + (rad[k + 1] - rad[k]) * f); }
    return [P, R];
  }
  // 给一条中心线，和它两侧各自的厚度，画出外轮廓。RA 是行进方向左手那一侧，RB 是右手那一侧。fur>0 时在 B 侧（或两侧）立起一排毛。
  function ribbon(P, RA, RB, fur, both) {
    RB = RB || RA; var n = P.length, A = [], B = [], i, f = function (v) { return v.toFixed(1); }, t0, t1;
    for (i = 0; i < n; i++) {
      var a = P[Math.max(0, i - 1)], b = P[Math.min(n - 1, i + 1)], tx = b[0] - a[0], ty = b[1] - a[1], l = Math.hypot(tx, ty) || 1; tx /= l; ty /= l;
      var nx = ty, ny = -tx, z = fur && i > 0 && i < n - 1 ? (i % 2 ? fur * 8.5 : fur * 1.5) : 0;
      A.push([P[i][0] + nx * (RA[i] + (both ? z : 0)), P[i][1] + ny * (RA[i] + (both ? z : 0))]); B.push([P[i][0] - nx * (RB[i] + z), P[i][1] - ny * (RB[i] + z)]);
      if (i === 0) t0 = [tx, ty, nx, ny]; if (i === n - 1) t1 = [tx, ty, nx, ny];
    }
    var cap = function (c, ra, rb, T, s) { var o = []; for (var k = 1; k < 6; k++) { var th = Math.PI * k / 6, r = s > 0 ? ra + (rb - ra) * k / 6 : rb + (ra - rb) * k / 6; o.push([c[0] + r * s * (Math.cos(th) * T[2] + Math.sin(th) * T[0]), c[1] + r * s * (Math.cos(th) * T[3] + Math.sin(th) * T[1])]); } return o; };
    var all = A.concat(cap(P[n - 1], RA[n - 1], RB[n - 1], t1, 1), B.slice().reverse(), cap(P[0], RA[0], RB[0], t0, -1));
    return { d: 'M' + all.map(function (q) { return f(q[0]) + ' ' + f(q[1]); }).join('L') + 'Z',
      edge: function (side, from, to) { var S2 = side === 'A' ? A : B, k0 = Math.floor(n * from), k1 = Math.ceil(n * (to || 1)); return 'M' + S2.slice(k0, k1).map(function (q) { return f(q[0]) + ' ' + f(q[1]); }).join('L'); } };
  }
  function qb(a, c, b, u) { var v = 1 - u; return [v * v * a[0] + 2 * v * u * c[0] + u * u * b[0], v * v * a[1] + 2 * v * u * c[1] + u * u * b[1]]; }
  function cb(a, b, c, d, u) { var v = 1 - u; return [v * v * v * a[0] + 3 * v * v * u * b[0] + 3 * v * u * u * c[0] + u * u * u * d[0], v * v * v * a[1] + 3 * v * v * u * b[1] + 3 * v * u * u * c[1] + u * u * u * d[1]]; }
  function hump(u, c, w) { var z = (u - c) / w; return Math.exp(-z * z); }

  var rig = null, cup = null;
  function rigInit() {
    var q = function (c) { return kitty.querySelector('.kq-' + c); };
    rig = { svg: kitty.querySelector('svg'), all: q('all'), el: {}, bangs: kitty.querySelectorAll('.kq-bang'), stars: kitty.querySelectorAll('.kq-star'), name: 'sit', t0: performance.now(), from: null, dur: 0, last: K('sit'),
      fx: 50, gy: FLOOR, flip: false, move: null, queue: [], turning: false, prev: performance.now(), phi: 0, om: 0, psi: 0, pom: 0, pvx: 0, hx: 50, hy: 50, cupS: null, calm: window.matchMedia('(prefers-reduced-motion: reduce)').matches };
    ['tail', 'hl', 'phl', 'fl', 'pfl', 'body', 'clip', 'scruff', 'neck', 'seam', 'seam2', 'hr', 'phr', 'fr', 'pfr', 'head', 'hs', 'hf', 'card', 'far', 'ball'].forEach(function (c) { rig.el[c] = q(c); });
    kitty.setAttribute('data-s', 'sit'); draw(rig.last, 0);
    loop();
  }
  // 逐帧循环。猫回了箱子就停下，不白白耗电；它一出来再开。隔一帧画一次，大约每秒三十帧。
  // 窗口名叫 sim 时改用定时器驱动，供无头浏览器里按时间推演、检查走和跳的全过程。
  function loop() {
    if (!rig || rig.looping || rig.calm) return; rig.looping = true; var me = rig, skip = false;
    var alive = function () { if (rig !== me || !kitty || !kitty.isConnected || kitty.classList.contains('gone')) { me.looping = false; return false; } return true; };
    if (window.name === 'sim') { var iv = setInterval(function () { if (!alive()) { clearInterval(iv); return; } tick(); }, 16); return; }
    (function frame() { if (!alive()) return; skip = !skip; if (!skip || me.move) tick(); requestAnimationFrame(frame); })();
  }
  function place() { kitty.style.left = (rig.fx - KW / 2) + '%'; kitty.style.top = (rig.gy - KH * (-VB[1] / VB[3])) + '%'; }
  function be(name, dur) {
    if (!rig) return; rig.from = rig.last; rig.name = name; rig.t0 = performance.now(); rig.dur = dur == null ? 380 : dur; kitty.setAttribute('data-s', name);
    // 停着的姿势都朝屏幕中间：在右半边就朝左，在左半边就朝右
    if (FRONTAL[name] || SIDE[name]) rig.flip = rig.fx > 50; if (name === 'no') cupReset(); else if (rig.cupS && rig.cupS.mode === 'stand') cupGone();
    if (rig.calm) { var tt = rig.testT == null ? .4 : rig.testT; rig.last = GEN[name](tt); draw(rig.last, tt); if (name === 'no') cupCalm(); }
  }

  function tick() {
    var now = performance.now(), dt = Math.min(.04, (now - rig.prev) / 1000) || .016; rig.prev = now;
    var m = rig.move;
    if (kitty.classList.contains('held')) {
      rig.flip = rig.fx > 50;      // 拎着它走过屏幕中线，它跟着换一边看
      // 被拎着：后颈跟着指针走，身子是挂在后颈上的一个钟摆。指针一加速，身子就往反方向甩；有阻尼，慢慢停下来。
      var vx = (rig.hx - rig.fx) / dt * 1.6, ax = (vx - rig.pvx) / dt; rig.pvx = vx; ax = Math.max(-2500, Math.min(2500, ax));
      var L = 9; rig.om += (-(G / L) * Math.sin(rig.phi) - (ax / L) * Math.cos(rig.phi) * .12 - 3.4 * rig.om) * dt; rig.phi += rig.om * dt; rig.phi = Math.max(-1.1, Math.min(1.1, rig.phi));
      rig.pom += (70 * (rig.phi - rig.psi) - 7 * rig.pom) * dt; rig.psi += rig.pom * dt;
      rig.fx = rig.hx; rig.gy = rig.hy + 132 * UY;
    } else if (m && m.kind === 'walk') {
      // 走：身子前进的速度正好等于脚往后蹬的速度，所以脚尖踩在地上不打滑
      var g = GAIT[m.gait], v = g.len / g.T * UX, left = m.to - rig.fx, s = v * dt;
      if (Math.abs(left) <= s) { rig.fx = m.to; rig.move = null; next(); } else rig.fx += Math.sign(left) * s;
    } else if (m && m.kind === 'jump') {
      // 跳：蹲下蓄力 → 蹬出去 → 只受重力，沿抛物线飞 → 落地一蹲缓冲 → 接后面的姿态
      var tj = (now - m.t0) / 1000;
      if (tj >= m.T) { rig.fx = m.x1; rig.gy = m.y1; rig.move = null; if (m.cling) next(); else { be('crouch', 90); rig.turning = true; later(130, function () { rig.turning = false; next(); }); } }
      else if (tj >= 0) { if (!m.up) { m.up = true; be('air', 160); } rig.fx = m.x0 + m.vx * tj; rig.gy = m.y0 + m.vy * tj + .5 * G * tj * tj; }
    } else if (m && m.kind === 'fall') {
      m.v += G * dt; rig.gy += m.v * dt;
      if (rig.gy >= m.to) { rig.gy = m.to; rig.move = null; be('crouch', 90); rig.turning = true; later(130, function () { rig.turning = false; next(); }); }
    } else if (m && m.kind === 'climb') {
      var dirc = m.to < rig.gy ? -1 : 1, sc = CLIMB_V * dt; if (Math.abs(m.to - rig.gy) <= sc) { rig.gy = m.to; rig.move = null; next(); } else rig.gy += dirc * sc;
    }
    if (!kitty.classList.contains('held')) { rig.phi *= .8; rig.psi *= .8; }
    place();
    var t = (now - rig.t0) / 1000, target = GEN[rig.name](t), w = rig.dur ? ease((now - rig.t0) / rig.dur) : 1;
    rig.last = rig.from && w < 1 ? blend(rig.from, target, w) : target; draw(rig.last, t); cupStep(dt, rig.last);
  }

  function draw(p, t) {
    var E = rig.el, fz = p.fz, side = 1 - fz, i, f1 = function (v) { return v.toFixed(1); };
    // 被拎着时整只猫绕后颈那一点摆
    var pivot = [p.S[0] - 8, p.S[1] - 20];
    rig.all.setAttribute('transform', (rig.flip ? 'scale(-1 1) ' : '') + (Math.abs(rig.phi) > .002 ? 'rotate(' + (rig.phi * 57.3).toFixed(1) + ' ' + f1(pivot[0]) + ' ' + f1(pivot[1]) + ')' : ''));

    // 身子。腹侧（A）和背侧（B）厚度不同：胸深、腹线往后收、臀圆；肩上微隆。
    var BP = [], RA = [], RB = [], n = p.fur > .02 ? 26 : 14, rmc = 2 * p.rm - (p.rs + p.rh) / 2, k = side * p.tuck;
    for (i = 0; i <= n; i++) { var u = i / n, base = (1 - u) * (1 - u) * p.rs + 2 * u * (1 - u) * rmc + u * u * p.rh; BP.push(qb(p.S, p.M, p.H, u));
      RA.push(base + k * (6.5 * hump(u, .1, .24) - 3.4 * hump(u, .66, .22) + 1.5 * hump(u, 1, .2))); RB.push(base + k * (2.5 * hump(u, .14, .16) - 1.5 * hump(u, .5, .25))); }
    var body = ribbon(BP, RA, RB, p.fur, false); E.body.setAttribute('d', body.d); E.clip.setAttribute('d', body.d);
    // 脊背在肩和胯处的方向，用来确定"腹侧"朝哪：站着时朝下，被拎着时朝前
    var dS = [BP[1][0] - BP[0][0], BP[1][1] - BP[0][1]], lS = Math.hypot(dS[0], dS[1]) || 1, vS = [dS[1] / lS, -dS[0] / lS];
    var dH = [BP[n][0] - BP[n - 1][0], BP[n][1] - BP[n - 1][1]], lH = Math.hypot(dH[0], dH[1]) || 1, vH = [dH[1] / lH, -dH[0] / lH];
    if (fz > .5) { vS = [0, 1]; vH = [0, 1]; }

    // 脖子、后颈那一撮被捏起来的皮
    var NK = [p.C[0] - 15 * (1 - p.face), p.C[1] + 12 + 10 * p.face], nk = spline([p.S, [(p.S[0] + NK[0]) / 2, (p.S[1] + NK[1]) / 2], NK], [p.rs * .82, 17, 16 - 3 * p.face], 6);
    E.neck.setAttribute('d', ribbon(nk[0], nk[1]).d);
    E.scruff.setAttribute('d', p.hx > .02 ? 'M' + f1(p.S[0] - 16) + ' ' + f1(p.S[1] - 6) + ' L' + f1(pivot[0] - 2) + ' ' + f1(pivot[1] - 6 * p.hx) + ' L' + f1(p.S[0] + 8) + ' ' + f1(p.S[1] - 14) + 'Z' : '');

    // 尾巴：从根到尖渐细
    var T0 = [p.H[0] + p.tr[0], p.H[1] + p.tr[1]], TP = [], TR = [], tn = p.fur > .02 ? 22 : 14;
    for (i = 0; i <= tn; i++) { var v = i / tn; TP.push(cb(T0, p.t.slice(0, 2), p.t.slice(2, 4), p.t.slice(4, 6), v)); TR.push(p.tw[0] + (p.tw[1] - p.tw[0]) * v); }
    E.tail.setAttribute('d', ribbon(TP, TR, TR, p.fur * .8, true).d);

    var seam = '', seam2 = '';
    // 脚尖：很小的一块，贴着地，顺着脚掌的方向往前一点点
    var toes = function (e, from, toe, rx, ry) { var ang = Math.atan2(toe[1] - from[1], toe[0] - from[0]) * 57.3 - 90, fwd = 2.4 * side;
      e.setAttribute('cx', f1(toe[0] + fwd)); e.setAttribute('cy', f1(toe[1] - ry + .4)); e.setAttribute('rx', rx); e.setAttribute('ry', ry); e.setAttribute('transform', 'rotate(' + f1(ang * .6) + ' ' + f1(toe[0]) + ' ' + f1(toe[1] - ry) + ')'); };
    // 前腿：从胸下到腕是一根直的、上粗下细的柱子；腕以下是一小截爪子，离地时往后耷拉。
    var front = function (sx, toe, droop, pathEl, pawEl, near) {
      var Eb = [p.S[0] + sx + vS[0] * p.rs * .62 - 1 * side, p.S[1] + vS[1] * p.rs * .62], d = droop * Math.PI / 180;
      var wrist = [toe[0] - 2.2 * side * Math.cos(d) + 6.5 * Math.sin(d) * side, toe[1] - 6.5 * Math.cos(d) - 1.5 * Math.sin(d)];
      var rb = ribbon([Eb, [(Eb[0] + wrist[0]) / 2, (Eb[1] + wrist[1]) / 2], wrist], [9.2, 7.2, 5.6]);
      pathEl.setAttribute('d', rb.d); toes(pawEl, wrist, toe, 5.4, 3.6);
      if (near) seam += rb.edge('A', .45) + rb.edge('B', .45); seam2 += rb.edge(sx > 0 ? 'A' : 'B', .2);
    };
    // 后腿：大腿从胯往前下到膝，小腿从膝往后下到跗关节，脚掌从跗关节往前下到脚尖。只有脚尖着地。
    var hind = function (sx, toe, leanDeg, pathEl, pawEl, near) {
      var R = [p.H[0] + sx + vH[0] * 4 + 2 * side, p.H[1] + vH[1] * 4], a = leanDeg * Math.PI / 180;
      var hock = [toe[0] - Math.sin(a) * LM, toe[1] - Math.cos(a) * LM - 2.2];
      // 膝藏在腹侧的轮廓里，看得见的只是大腿前缘微微往前鼓一点
      var dxk = hock[0] - R[0], dyk = hock[1] - R[1], dk = Math.hypot(dxk, dyk) || 1, fold = Math.max(0, 1 - dk / (LT + LS)), bow = 3 + 16 * fold;
      var knee = [R[0] + dxk * .5 + (dyk / dk) * bow, R[1] + dyk * .5 - (dxk / dk) * bow];
      var sp = spline([R, knee, hock, [toe[0] - 1.2, toe[1] - 3.4]], [16, 11.5, 5.2, 4], 14), rb = ribbon(sp[0], sp[1]);
      pathEl.setAttribute('d', rb.d); pathEl.style.opacity = side.toFixed(2); toes(pawEl, hock, toe, 5.2 + 1.2 * fz, 3.4 + .8 * fz);
      if (near) seam += rb.edge('A', .1, .6);
    };
    hind(p.ho, p.hr, p.lean[0], E.hr, E.phr, true); hind(-p.ho, p.hl, p.lean[1], E.hl, E.phl, false);
    front(p.fo, p.fr, p.pf[0], E.fr, E.pfr, true); front(-p.fo, p.fl, p.pf[1], E.fl, E.pfl, false);
    // 正对你坐着时，后腿收在身子两侧：只勾出两团臀的弧线
    [1, -1].forEach(function (sg) { seam2 += 'M' + f1(p.H[0] + sg * p.rh * .5) + ' ' + f1(p.H[1] - p.rh * .62) + ' Q' + f1(p.H[0] + sg * p.rh * 1.02) + ' ' + f1(p.H[1] - p.rh * .2) + ' ' + f1(p.H[0] + sg * p.rh * .5) + ' ' + f1(p.H[1] + p.rh * .9); });
    // 远侧的腿略浅一点，侧面时才分得出四条腿；转成正面时变回同样的黑
    var c = function (a, b) { return Math.round(a + (b - a) * fz); }; E.far.setAttribute('fill', 'rgb(' + c(62, 31) + ',' + c(58, 29) + ',' + c(54, 26) + ')');
    E.seam.setAttribute('d', seam); E.seam.style.opacity = (.34 * Math.max(0, 1 - fz * 2) * p.sm).toFixed(2); E.seam2.setAttribute('d', seam2); E.seam2.style.opacity = (.5 * Math.max(0, fz * 2 - 1) * p.sm).toFixed(2);

    // 头：侧着露侧脸，正着露正脸
    E.head.setAttribute('transform', 'translate(' + f1(p.C[0]) + ' ' + f1(p.C[1]) + ') rotate(' + f1(p.ha) + ')');
    E.hs.style.opacity = Math.max(0, Math.min(1, (.6 - p.face) / .2)).toFixed(2); E.hf.style.opacity = Math.max(0, Math.min(1, (p.face - .4) / .2)).toFixed(2);
    var st = rig.svg.style; st.setProperty('--lid', f1(p.lid) + 'px'); st.setProperty('--gx', f1(p.gx) + 'px'); st.setProperty('--gy', f1(p.gy) + 'px'); st.setProperty('--ps', p.ps.toFixed(2));
    kitty.classList.toggle('shut', p.lid > -3);
    // 牌子、三个叹号、毛球、星星
    var name = rig.name;
    E.ball.setAttribute('transform', 'translate(' + f1(p.ball[0]) + ' -11.5) rotate(' + f1(p.ball[1]) + ')');
    for (i = 0; i < rig.stars.length; i++) { var ph = name === 'joy' ? ((t * .75 + i / 5) % 1) : 0, an = (-162 + i * 36) * Math.PI / 180, rr = 44 + 20 * ph, so = name === 'joy' ? Math.sin(Math.PI * ph) : 0;
      rig.stars[i].setAttribute('transform', 'translate(' + f1(p.C[0] + Math.cos(an) * rr) + ' ' + f1(p.C[1] - 6 + Math.sin(an) * rr) + ') rotate(' + f1(t * 70 + i * 40) + ') scale(' + (1.15 + 1.35 * so).toFixed(2) + ')'); rig.stars[i].style.opacity = so.toFixed(2); }
    E.card.setAttribute('transform', 'translate(' + f1(p.fr[0] + 16) + ' ' + f1(p.fr[1] - 20) + ')');
    for (i = 0; i < 3; i++) { var kk = name === 'oops' ? Math.max(0, Math.min(1, (t - .1 - i * .13) / .16)) : 0, sc = kk < 1 ? kk * 1.25 : 1 + .25 * Math.max(0, 1 - (t - .26 - i * .13) / .1);
      rig.bangs[i].setAttribute('transform', 'translate(' + f1(p.C[0] + (i - 1) * 20) + ' ' + f1(p.C[1] - 52 - (i === 1 ? 8 : 0)) + ') rotate(' + (i - 1) * 13 + ') scale(' + sc.toFixed(2) + ')'); rig.bangs[i].style.opacity = kk > 0 ? 1 : 0; }
  }

  // ── 杯子：一个真的刚体 ──
  // 任务栏是高度为零的那条线；任何窗口的上沿都是比它高的一个台面。
  // 杯子立在猫脚下的那个面上。爪子拨到它，它绕着底边的一角翻倒：过了平衡点之前靠爪子顶，过了之后靠重力自己倒下去。
  // 倒在任务栏上，就躺在那儿。倒在窗口上沿，它会滚出台面，只受重力落到任务栏那条线上，弹一两下，损失大半能量，最后躺住。
  function unit() { return kitty.getBoundingClientRect().width / VB[2]; }
  function cupReset() {
    var d = dsk.getBoundingClientRect(), u = unit(), w = 15 * u, h = 20 * u, surf = rig.gy / 100 * d.height, ground = FLOOR / 100 * d.height;
    rig.cupS = { mode: 'stand', x: rig.fx / 100 * d.width + 58 * u, y: surf, vx: 0, vy: 0, a: 0, w: 0, W: w, H: h, surf: surf, ground: ground, high: surf < ground - 4, bounces: 0, rest: 0, px: 100, py: 100 };
    cup.hidden = false; cup.style.width = w + 'px'; cup.style.height = h + 'px'; cup.style.opacity = 1; cupDraw();
  }
  function cupGone() { if (cup) cup.hidden = true; if (rig) rig.cupS = null; }
  function cupDraw() { var c = rig.cupS; cup.style.left = (c.x - c.W / 2) + 'px'; cup.style.top = (c.y - c.H) + 'px'; cup.style.transformOrigin = c.px + '% ' + c.py + '%'; cup.style.transform = 'rotate(' + (c.a * 57.3).toFixed(1) + 'deg)'; }
  function cupCalm() { var c = rig.cupS; if (!c) return; var u = unit(), d = dsk.getBoundingClientRect(), paw = rig.fx / 100 * d.width + (rig.last.fr[0] + 9) * u; if (paw > c.x - c.W / 2) { c.a = Math.PI / 2; c.cx = c.x + c.W / 2 + c.H / 2; c.cy = c.surf - c.W / 2; c.px = 50; c.py = 50; c.x = c.cx; c.y = c.cy + c.H / 2; } cupDraw(); }
  function cupStep(dt, p) {
    var c = rig.cupS; if (!c) return; var d = dsk.getBoundingClientRect(), u = unit(), g = G / 100 * d.height;
    if (c.mode === 'stand') {
      if (rig.name === 'no') { var paw = rig.fx / 100 * d.width + (p.fr[0] + 9) * u; if (paw > c.x - c.W / 2) { c.mode = 'topple'; c.w = 2.6; } }
    } else if (c.mode === 'topple') {
      // 绕右下角翻。平衡点是重心正好在这个角正上方的那个角度：之前重力往回拉，之后重力往下带
      var bal = Math.atan2(c.W, c.H), r = Math.hypot(c.W, c.H) / 2;
      c.w += g * Math.sin(c.a - bal) / (r * 1.33) * dt; if (c.a < bal) c.w = Math.max(c.w, 2.4); c.a += c.w * dt;
      if (c.a >= Math.PI / 2) {
        // 侧面着了台面。换成以杯子中心来算位置
        c.a = Math.PI / 2; c.cx = c.x + c.W / 2 + c.H / 2; c.cy = c.surf - c.W / 2; c.px = 50; c.py = 50;
        if (c.high) { c.mode = 'roll'; c.vx = c.w * c.W * .35; c.rolled = 0; } else { c.mode = 'rest'; c.vx = c.w * c.W * .25; c.floor = c.surf; }
      }
    } else if (c.mode === 'roll') {
      // 在台面上往外滚一小段，滚出去就掉
      c.cx += c.vx * dt; c.a += c.vx / (c.W / 2) * dt * .5; c.rolled += c.vx * dt; if (c.rolled > c.W * .8) { c.mode = 'fly'; c.vy = 0; c.w = c.vx / (c.W / 2) * .5; }
    } else if (c.mode === 'fly') {
      c.vy += g * dt; c.cx += c.vx * dt; c.cy += c.vy * dt; c.a += c.w * dt;
      if (c.cy + c.W / 2 >= c.ground && c.vy > 0) { c.cy = c.ground - c.W / 2; c.bounces++; if (c.vy < g * .09 || c.bounces > 3) { c.mode = 'rest'; c.floor = c.ground; c.a = Math.round(c.a / Math.PI - .5) * Math.PI + Math.PI / 2; } else { c.vy = -c.vy * .32; c.vx *= .55; c.w *= .45; } }
    } else if (c.mode === 'rest') {
      c.vx *= Math.pow(.03, dt); c.cx += c.vx * dt; c.cy = c.floor - c.W / 2; c.rest += dt;
      if (c.rest > 2) cup.style.opacity = Math.max(0, 1 - (c.rest - 2) / .5); if (c.rest > 2.6) { if (rig.name === 'no') cupReset(); else cupGone(); return; }
    }
    if (c.mode !== 'stand' && c.mode !== 'topple') { c.x = c.cx; c.y = c.cy + c.H / 2; }
    cupDraw();
  }

  // ── 它怎么去一个地方：把窗口当猫爬架 ──
  // 任务栏是高度为零的那条线，每个窗口的上沿是一层台面。它在台面之间轻跳着走。
  // 一跳的极限：两层台面左右隔着的空当、上下的高度差，都不超过屏幕的三分之一。
  // 目的地一跳够不着，就找中间能落脚的台面一层层跳过去；实在没有落脚处，它就从屏幕的侧边跳出去，隔一小会儿，再对着目的地上沿的高度从侧边跳回来。
  var JUMP_X = 33, JUMP_Y = 33, CLIMB_V = 44;
  function surfaces() {
    var L = [{ id: 'floor', y: FLOOR, x0: 2, x1: 98 }];
    Array.prototype.forEach.call(dsk.querySelectorAll('.wnd'), function (w) { var r = rel(w); if (r.y > 6) L.push({ id: w, y: r.y + .5, x0: r.x + 2, x1: r.x + r.w - 2, left: r.x, right: r.x + r.w }); });
    return L;
  }
  function under(L, x, y) { var best = null; L.forEach(function (s) { if (Math.abs(s.y - y) < 1.6 && x >= s.x0 - 3 && x <= s.x1 + 3) best = s; }); return best; }
  function gapX(a, b) { return Math.max(0, Math.max(a.x0, b.x0) - Math.min(a.x1, b.x1)); }
  function canHop(a, b) { return gapX(a, b) <= JUMP_X && Math.abs(a.y - b.y) <= JUMP_Y; }
  // 从一层台面到另一层，最少跳几次、依次经过哪些台面
  function route(L, a, b) {
    if (a === b) return [a]; var seen = [a], q = [[a]];
    while (q.length) { var path = q.shift(), last = path[path.length - 1];
      for (var i = 0; i < L.length; i++) { var s = L[i]; if (seen.indexOf(s) >= 0 || !canHop(last, s)) continue; var np = path.concat([s]); if (s === b) return np; seen.push(s); q.push(np); } }
    return null;
  }
  // 沿着一串台面跳过去：每一跳都从这层离下一层最近的地方起跳，落在下一层离得最近的地方
  function hops(path, x, steps) {
    for (var i = 1; i < path.length; i++) { var a = path[i - 1], b = path[i], lo = Math.max(a.x0, b.x0), hi = Math.min(a.x1, b.x1), take, land;
      if (lo <= hi) { take = Math.max(lo, Math.min(hi, x)); land = take; } else if (b.x0 > a.x1) { take = a.x1; land = b.x0; } else { take = a.x0; land = b.x1; }
      steps.push({ kind: 'walk', to: take }); steps.push({ kind: 'jump', x: land, y: b.y }); x = land; }
    return x;
  }
  // 没有落脚的台面：走到这层靠屏幕边的那一头，跳出屏幕；隔一小会儿，从那一侧对着目的地的高度跳回来，落在目的地靠边的那一头
  function portal(from, to, x, steps) {
    var left = x + to.x0 <= (100 - x) + (100 - to.x1), off = left ? -8 : 108, land = left ? to.x0 : to.x1;
    steps.push({ kind: 'walk', to: left ? from.x0 : from.x1 }); steps.push({ kind: 'jump', x: off, y: from.y - 3, exit: true });
    steps.push({ kind: 'warp', x: off, y: to.y - 3, wait: 420 }); steps.push({ kind: 'jump', x: land, y: to.y });
    return land;
  }
  function plan(fx, gy, then) {
    var L = surfaces(), to = under(L, fx, gy) || L[0], from = under(L, rig.fx, rig.gy), steps = [], x = rig.fx, floorS = L[0];
    if (!from) { var here = { y: rig.gy, x0: rig.fx, x1: rig.fx }; if (to !== floorS && canHop(here, to)) { var lx = Math.max(to.x0, Math.min(to.x1, rig.fx)); steps.push({ kind: 'jump', x: lx, y: to.y }); from = to; x = lx; } else { var bx = Math.max(3, Math.min(97, rig.fx)); steps.push({ kind: 'jump', x: bx, y: FLOOR }); from = floorS; x = bx; } }
    var r = route(L, from, to);
    x = r ? hops(r, x, steps) : portal(from, to, x, steps);
    steps.push({ kind: 'walk', to: Math.max(to.x0, Math.min(to.x1, fx)) }); steps.push({ kind: 'state', name: then || 'sit' });
    return steps;
  }

  function emerge() {
    if (out) return; var r = rel(box); kitty.classList.remove('gone'); rig.prev = performance.now(); loop(); rig.fx = r.x + r.w * .45; rig.gy = r.y + r.h * .5; rig.flip = false; rig.move = null; rig.queue = []; place(); out = true; pose(); be('crouch', 0);
  }
  function idle() { return !rig || (!rig.move && !(rig.queue && rig.queue.length) && !rig.turning); }
  // 等它到了地方再做某件事
  function whenIdle(fn) { if (idle()) { fn(); return; } later(120, function () { whenIdle(fn); }); }
  function act(name, dur) { whenIdle(function () { be(name, dur); }); }
  // 朝向变了就先转身：身子转成正对你，再转向另一边
  function face(left, go) {
    if (rig.flip === left) { go(); return; }
    rig.turning = true; kitty.classList.remove('turn'); void kitty.offsetWidth; kitty.classList.add('turn'); be('front', 160); later(170, function () { rig.flip = left; rig.turning = false; go(); }); later(360, function () { kitty.classList.remove('turn'); });
  }
  function next() {
    if (!rig || !rig.queue || !rig.queue.length) return; var s = rig.queue.shift(), dx;
    if (s.kind === 'state') { kitty.classList.toggle('ground', rig.gy > FLOOR - 2); if (s.name === 'home') { kitty.classList.add('gone'); out = false; cupGone(); pose(s.box || 'calm'); return; } be(s.name, 460); next(); return; }
    if (s.kind === 'walk') { dx = s.to - rig.fx; if (Math.abs(dx) < .8) { next(); return; }
      face(dx < 0, function () { var d = Math.abs(dx), g = d < 6 ? 'walk' : d < 26 ? 'trot' : 'run'; be(g, 200); rig.move = { kind: 'walk', gait: g, to: s.to }; }); return; }
    if (s.kind === 'jump') { dx = s.x - rig.fx;
      face(Math.abs(dx) < .6 ? rig.flip : dx < 0, function () {
        // 轻轻一跳：先定最高点只比起点和落点里较高的那个高出一点，再由重力反推起跳速度和滞空时间
        be('crouch', 150); var top = Math.min(rig.gy, s.y) - (3.2 + Math.abs(dx) * .1), vy = -Math.sqrt(2 * G * (rig.gy - top)), T = (-vy + Math.sqrt(vy * vy + 2 * G * (s.y - rig.gy))) / G;
        rig.move = { kind: 'jump', t0: performance.now() + 160, T: T, x0: rig.fx, y0: rig.gy, x1: s.x, y1: s.y, vx: dx / T, vy: vy, cling: s.cling || s.exit };
      }); return; }
    // 出了屏幕：藏起来，换到目的地的高度，等一小会儿再露面
    if (s.kind === 'warp') { kitty.classList.add('away'); rig.fx = s.x; rig.gy = s.y; rig.flip = s.x > 50; be('air', 0); rig.turning = true; later(s.wait, function () { kitty.classList.remove('away'); rig.turning = false; next(); }); }
  }
  function travel(fx, gy, then) {
    emerge(); then = then || 'sit';
    if (rig.calm) { rig.fx = fx; rig.gy = gy; rig.flip = false; rig.queue = []; place(); kitty.classList.toggle('ground', gy > FLOOR - 2); be(then, 0); return; }
    rig.queue = plan(fx, gy, then); rig.move = null; next();
  }
  function perch(w, frac, then) { emerge(); var r = rel(w); travel(r.x + r.w * frac, r.y + .5, then); }
  function floor(x, then) { emerge(); travel(x, FLOOR, then); }
  function goHome(state) {
    if (!out) { pose(state); return; }
    var r = rel(box);
    if (rig.calm) { kitty.classList.add('gone'); out = false; cupGone(); pose(state || 'calm'); return; }
    var steps = plan(r.x + r.w * .45 - 6, FLOOR, 'crouch'); steps.pop(); steps.push({ kind: 'jump', x: r.x + r.w * .45, y: r.y + r.h * .5 }); steps.push({ kind: 'state', name: 'home', box: state || 'calm' });
    rig.queue = steps; rig.move = null; next();
  }
  // 被放下：从松手的地方只受重力落下去，落到下方最近的台面；松在箱子上方就回家
  function settle() {
    var b = rel(box), feet = rig.gy; rig.queue = [];
    if (rig.fx > b.x && rig.fx < b.x + b.w && feet > b.y - 8 && feet < b.y + b.h + 8) { kitty.classList.add('gone'); out = false; pose('calm'); return; }
    var y = FLOOR; surfaces().forEach(function (s) { if (s.id !== 'floor' && rig.fx > s.x0 - 2 && rig.fx < s.x1 + 2 && s.y >= feet - 3 && s.y < y) y = s.y; });
    kitty.classList.toggle('ground', y === FLOOR);
    if (rig.calm || y <= feet) { rig.gy = y; place(); be('sit', 300); return; }
    be('air', 180); rig.queue = [{ kind: 'state', name: 'sit' }]; rig.move = { kind: 'fall', v: 0, to: y };
  }
  function refuse() { emerge(); act('no', 420); }
  // 它自己逛
  function wander() {
    if (!idle()) return;
    var L = surfaces(), cur = under(L, rig.fx, rig.gy) || L[0], nearby = L.filter(function (s) { return s !== cur && canHop(cur, s); }), pool = nearby.length && Math.random() < .8 ? nearby : L, s = pool[Math.floor(Math.random() * pool.length)];
    travel(s.x0 + Math.random() * (s.x1 - s.x0), s.y, Math.random() < .2 ? 'loaf' : 'sit');
  }
  function size() { return rel(kitty); }


  return {
    summon: summon, be: be, act: act, whenIdle: whenIdle, idle: idle, travel: travel, floor: floor, goHome: goHome, emerge: emerge, settle: settle, refuse: refuse, wander: wander, pose: pose, count: count, rel: rel,
    isOut: function () { return out; }, rig: function () { return rig; }, kitty: function () { return kitty; }, box: function () { return box; }, cup: function () { return cup; },
    // 真桌面上的三样由程序量出来再告诉它：有哪些台面、任务栏在哪、猫画多大
    config: function (o) { if (o.surfaces) surfaces = o.surfaces; if (o.floor != null) FLOOR = o.floor; if (o.kw) { KW = o.kw; KH = o.kh; UX = KW / VB[2]; UY = KH / VB[3]; } }
  };
};
