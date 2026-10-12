// 从皮肤实验台里取出猫的画稿，生成桌面上用的 ui/skin.js。
// 实验台是 prototype 的唯一依据；实验台改了，重新运行这一份即可：node app/tools/extract-skin.mjs
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const lab = readFileSync(join(here, '..', '..', 'design', 'ysoul-skin-lab.html'), 'utf8');

function between(text, start, end, label) {
  const a = text.indexOf(start);
  const b = text.indexOf(end, a + start.length);
  if (a < 0 || b < 0) throw new Error(`在实验台里找不到：${label}`);
  return text.slice(a, b);
}

// 画法与各状态的样式：从"画法"那一段起，到"减少动态"的规则之前
const css = between(lab, '/* ── 画法', '@media (prefers-reduced-motion', '画法的样式');
// 手抖的滤镜、记号表、眼眶的裁剪
const defs = between(lab, '<svg width="0" height="0"', '</svg>', '公共定义') + '</svg>';
// 猫
const cat = between(lab, '<template id="c-cat">', '</template>', '猫的画稿').replace('<template id="c-cat">', '');

const out = `// 由 app/tools/extract-skin.mjs 从 design/ysoul-skin-lab.html 生成，不要手改。
window.YSOUL_SKIN = ${JSON.stringify({ css, defs, cat })};
`;
writeFileSync(join(here, '..', 'ui', 'skin.js'), out, 'utf8');
console.log(`skin.js: 样式 ${css.length} 字，定义 ${defs.length} 字，猫 ${cat.length} 字`);
