// 从操作面样稿里取出"箱子和箱外的猫"的引擎与样式，生成桌宠程序用的两份文件。
// 样稿（design/ysoul-surface-lab.html）是权威；这里只做搬运，不改逻辑。样稿改了，重跑这一份即可：
//   node app/tools/extract-cat.mjs
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const lab = readFileSync(join(root, 'design', 'ysoul-surface-lab.html'), 'utf8');

function between(text, start, end) {
  const i = text.indexOf(start), j = text.indexOf(end, i + start.length);
  if (i < 0 || j < 0) throw new Error(`样稿里找不到这一段：${start} … ${end}`);
  return text.slice(i, j);
}

const helpers = between(lab, '  function el(tag, cls, html)', '  // ── 拖动 ──');
const rig = between(lab, '  // ── 拖动 ──', '  // ── 别的组件 ──');
const css = between(lab, '/* ── 箱子 ── */', '/* ── 猫和箱子以外的组件 ── */');

const js = `// 由 app/tools/extract-cat.mjs 从 design/ysoul-surface-lab.html 生成，不要手改。
// 用法：var cat = window.YSOUL_CAT(舞台元素); cat.config({...}); cat.summon(x, y, 'calm');
window.YSOUL_CAT = function (dsk) {
  'use strict';
  var SKIN = window.YSOUL_SKIN, NS = 'http://www.w3.org/2000/svg';
  var box = null, kitty = null, out = false, timers = [], roam = null;
  var SLOT = { no: 'cross', ask: 'ring' };
${helpers}${rig}
  return {
    summon: summon, be: be, act: act, whenIdle: whenIdle, idle: idle, travel: travel, floor: floor, goHome: goHome, emerge: emerge, settle: settle, refuse: refuse, wander: wander, pose: pose, count: count, rel: rel,
    isOut: function () { return out; }, rig: function () { return rig; }, kitty: function () { return kitty; }, box: function () { return box; }, cup: function () { return cup; },
    // 真桌面上的三样由程序量出来再告诉它：有哪些台面、任务栏在哪、猫画多大
    config: function (o) { if (o.surfaces) surfaces = o.surfaces; if (o.floor != null) FLOOR = o.floor; if (o.kw) { KW = o.kw; KH = o.kh; UX = KW / VB[2]; UY = KH / VB[3]; } }
  };
};
`;
writeFileSync(join(root, 'app', 'ui', 'catrig.js'), js, 'utf8');
writeFileSync(join(root, 'app', 'ui', 'cat.css'), `/* 由 app/tools/extract-cat.mjs 从 design/ysoul-surface-lab.html 生成，不要手改。 */\n${css}`, 'utf8');
console.log(`catrig.js ${js.length} 字，cat.css ${css.length} 字`);
