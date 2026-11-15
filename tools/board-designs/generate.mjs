#!/usr/bin/env node
/**
 * Procedural board studies for rucheckers. No dependencies, images, fonts, or
 * network requests. All wood, joinery, and lacquer are SVG drawing primitives.
 * Run: node tools/board-designs/generate.mjs [output-directory]
 * The production board and settings are intentionally left for the design choice.
 */
import { mkdir, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export const designs = [
  { id: '01', name: 'Цельный дуб', note: 'Мёд и коньяк', kind: 'solid', axis: 1, seed: 42,
    light: '#dfc08c', dark: '#956741', frame: '#785334', grain: .82, wave: 1, gloss: .65,
    description: 'Непрерывные волокна дуба под двумя оттенками прозрачного лака.' },
  { id: '02', name: 'Цельный орех', note: 'Дымчатый лак', kind: 'solid', axis: 0, seed: 83,
    light: '#bea17e', dark: '#725640', frame: '#523c2d', grain: .72, wave: 1.8, gloss: .45,
    description: 'Широкие спокойные волокна и глубокий дымчатый оттенок.' },
  { id: '03', name: 'Светлый ясень', note: 'Песок и табак', kind: 'solid', axis: 1, seed: 26,
    light: '#e4d9bc', dark: '#afa083', frame: '#a08d6b', grain: .48, wave: .5, gloss: .35, grid: true,
    description: 'Светлая древесина, тонкая сетка и едва заметный сатиновый блеск.' },
  { id: '04', name: 'Цельная вишня', note: 'Янтарный глянец', kind: 'solid', axis: 0, seed: 19,
    light: '#dbae84', dark: '#985d44', frame: '#774832', grain: .55, wave: 1.1, gloss: 1,
    description: 'Тёплая вишня с мягким янтарным бликом на цельной поверхности.' },
  { id: '05', name: 'Клён и орех', note: 'Классический набор', kind: 'tiles', axis: 0, seed: 128,
    light: '#eadabd', dark: '#7f5c42', frame: '#5a412e', grain: .6, wave: .75, gloss: .6, inlay: true,
    description: 'Отдельные клетки двух пород: направление волокон чередуется.' },
  { id: '06', name: 'Дуб и венге', note: 'Глубокий контраст', kind: 'tiles', axis: 1, seed: 164,
    light: '#d3bd94', dark: '#534839', frame: '#41362c', grain: .85, wave: .35, gloss: .8, inlay: true,
    description: 'Золотистый дуб и тёмный венге с тонкими продольными порами.' },
  { id: '07', name: 'Берёза и махагони', note: 'Красное дерево', kind: 'tiles', axis: 0, seed: 225,
    light: '#e5d2ab', dark: '#955a48', frame: '#723f30', grain: .52, wave: 1.5, gloss: .85,
    description: 'Светлая берёза рядом с красно-коричневыми клетками махагони.' },
  { id: '08', name: 'Торцевой массив', note: 'Дуб и орех', kind: 'endgrain', axis: 0, seed: 104,
    light: '#d4b583', dark: '#94704f', frame: '#684c34', grain: .6, wave: 1, gloss: .5,
    description: 'Торцевые бруски: небольшие дуги годовых колец внутри клеток.' },
  { id: '09', name: 'Фигурный шпон', note: 'Волнистый орех', kind: 'figured', axis: 1, seed: 59,
    light: '#cdb083', dark: '#826044', frame: '#593f2c', grain: .62, wave: 2, gloss: .9, inlay: true,
    description: 'Волнистый рисунок шпона и более глубокая полировка.' },
  { id: '10', name: 'Паркетный набор', note: 'Диагональные планки', kind: 'parquet', axis: 0, seed: 182,
    light: '#d4c3a1', dark: '#96785a', frame: '#6b533b', grain: .58, wave: .4, gloss: .45,
    description: 'Клетки из тонких планок с чередованием диагонального рисунка.' },
  { id: '11', name: 'Выбеленный дуб', note: 'Дымчатый сатин', kind: 'solid', axis: 0, seed: 91,
    light: '#dedaca', dark: '#a19787', frame: '#797363', grain: .5, wave: .8, gloss: .35, grid: true,
    description: 'Сдержанная серо-бежевая палитра, тонкая сетка и шелковистый лак.' },
  { id: '12', name: 'Оливковое дерево', note: 'Живые волокна', kind: 'solid', axis: 1, seed: 308,
    light: '#d7bf88', dark: '#927244', frame: '#6e5836', grain: .95, wave: 2.6, gloss: .7,
    description: 'Выразительные извилистые волокна проходят через всю доску.' },
];

const S = 1000;
// Same fit as BoardLayout::fit(..., 1000, 1000, false, 1): 8 * 112 + 2 * 52.
const CELL = 112;
const FRAME = 52;
const FIELD = CELL * 8;
const f = n => Number(n.toFixed(2));
const attrs = o => Object.entries(o).map(([k, v]) => ` ${k}="${v}"`).join('');
const tag = (name, a, body) => body === undefined ? `<${name}${attrs(a)}/>` : `<${name}${attrs(a)}>${body}</${name}>`;
const rect = (x, y, width, height, fill, more = {}) => tag('rect', { x: f(x), y: f(y), width: f(width), height: f(height), fill, ...more });
function random(seed) {
  let state = seed >>> 0;
  return () => { state = (Math.imul(state, 1664525) + 1013904223) >>> 0; return state / 4294967296; };
}
function tone(hex, amount) {
  const rgb = hex.slice(1).match(/../g).map(v => parseInt(v, 16));
  return '#' + rgb.map(v => Math.round(v + (amount > 0 ? 255 - v : v) * amount).toString(16).padStart(2, '0')).join('');
}
function gradient(id, colors, a = {}) {
  return tag('linearGradient', { id, x1: 0, y1: 0, x2: 0, y2: 1, ...a }, colors.map(([offset, color, opacity = 1]) => tag('stop', { offset, 'stop-color': color, 'stop-opacity': opacity })).join(''));
}
function path(points, color, width, opacity) {
  return tag('path', { d: points.map(([x, y], i) => `${i ? 'L' : 'M'}${f(x)},${f(y)}`).join(' '), fill: 'none', stroke: color, 'stroke-width': f(width), 'stroke-opacity': f(opacity), 'stroke-linecap': 'round' });
}

/** A few coherent bends shared by many irregularly spaced fibres, not white noise. */
function grain(width, height, seed, strength, bend, vertical = false, figured = false) {
  const rand = random(seed);
  const along = vertical ? height : width;
  const across = vertical ? width : height;
  const p = rand() * 6.28;
  const frequency = figured ? 2.4 : .8;
  let result = '';
  const warp = (u, v) => {
    const sweep = Math.sin(u * Math.PI * frequency + p) * .64 + Math.sin(u * 7.2 + p * .7) * .23;
    const arch = Math.exp(-((u - .44) ** 2) * 13) * Math.sin(v * 3.6 + p);
    const spread = Math.exp(-((u - .58) ** 2) * 17) * Math.tanh((v - .47) * 6);
    return (sweep + arch * .68 + spread * .8) * Math.min(across, along * .48) * (figured ? .055 : .025) * bend;
  };
  // Broad translucent fibres carry the wood colour; fine pores stay below it.
  for (let i = 0; i < 13; i++) {
    const base = rand() * across;
    const points = Array.from({ length: 35 }, (_, j) => {
      const u = j / 34;
      const v = base + warp(u, base / across);
      return vertical ? [v, u * along] : [u * along, v];
    });
    result += path(points, i % 3 ? '#442b19' : '#fff2d4', 2 + rand() * across * .015, .018 * strength);
  }
  const step = Math.max(4, across / 165);
  for (let base = -across * .05; base < across * 1.05; base += step * (.45 + rand() * 1.25)) {
    const phase = rand() * Math.PI * 2;
    const start = rand() < .68 ? 0 : rand() * .25;
    const end = rand() < .76 ? 1 : .65 + rand() * .35;
    const points = Array.from({ length: 29 }, (_, j) => {
      const u = start + (end - start) * j / 28;
      const v = base + warp(u, base / across) + Math.sin(u * 12 + phase) * step * .09;
      return vertical ? [v, u * along] : [u * along, v];
    });
    const pale = rand() < .26;
    result += path(points, pale ? '#fff5dc' : '#4a2c17', .3 + rand() * .65, (.028 + rand() * .048) * strength);
  }
  return result;
}

/** Small, incomplete arcs are enough to suggest sealed end grain. */
function endGrain(seed, strength) {
  const rand = random(seed);
  const cx = (rand() * 1.2 - .1) * CELL;
  const cy = (rand() * 1.2 - .1) * CELL;
  const squash = .75 + rand() * .5;
  const phase = rand() * Math.PI * 2;
  let result = '';
  for (let radius = 7; radius < CELL * 1.8; radius += 3 + rand() * 4.5) {
    const points = Array.from({ length: 81 }, (_, j) => {
      const a = j / 80 * Math.PI * 2;
      const r = radius * (1 + .026 * Math.sin(a * 4 + phase) + .016 * Math.cos(a * 7));
      return [cx + Math.cos(a) * r, cy + Math.sin(a) * r * squash];
    });
    result += path(points, '#4a2e1b', .55 + rand() * .7, (.05 + rand() * .035) * strength);
  }
  return result;
}

function pieces() {
  let body = '';
  // Same quiet reference position on all boards, with the middle open for grain.
  const squares = [[1, 0, false], [3, 0, false], [5, 0, false], [7, 0, false], [0, 1, false], [2, 1, false], [4, 1, false], [6, 1, false], [1, 6, true], [3, 6, true], [5, 6, true], [7, 6, true], [0, 7, true], [2, 7, true], [4, 7, true], [6, 7, true]];
  for (const [column, row, white] of squares) {
    const cx = FRAME + (column + .5) * CELL;
    const cy = FRAME + (row + .5) * CELL;
    body += tag('g', { transform: `translate(${cx} ${cy})` },
      tag('circle', { cx: 1.5, cy: 3, r: 43, fill: '#20150d', opacity: .14 }) +
      tag('circle', { cx: 1, cy: 2, r: 41, fill: '#20150d', opacity: .18 }) +
      tag('circle', { r: 39.5, fill: `url(#${white ? 'white' : 'black'}-piece)`, stroke: white ? '#a89a7f' : '#302219', 'stroke-width': 1.5 }) +
      [34, 26, 15].map(r => tag('circle', { cy: -.5, r, fill: 'none', stroke: white ? '#b3a484' : '#271d16', 'stroke-width': 1, opacity: .6 }) + tag('circle', { cy: .7, r: r + .5, fill: 'none', stroke: white ? '#fffbed' : '#ae8865', 'stroke-width': .9, opacity: white ? .75 : .22 })).join(''));
  }
  return tag('g', { class: 'pieces', 'aria-label': 'Пример шашек для проверки читаемости' }, body);
}

export function boardSvg(d, { includePieces = true } = {}) {
  let defs = tag('clipPath', { id: 'outer-clip' }, rect(0, 0, S, S, 'white', { rx: 12 })) +
    tag('clipPath', { id: 'field-clip' }, rect(FRAME, FRAME, FIELD, FIELD, 'white')) +
    tag('clipPath', { id: 'cell-clip' }, rect(0, 0, CELL, CELL, 'white'));
  for (const name of ['light', 'dark', 'frame']) {
    defs += gradient(name, [[0, tone(d[name], .07)], [.42, d[name]], [1, tone(d[name], -.055)]], { x1: 0, y1: 0, x2: .28, y2: 1 });
  }
  defs += gradient('surface-light', [[0, '#fff4da', .08], [.35, '#fff4da', 0], [1, '#241608', .075]], { x1: 0, y1: 0, x2: 1, y2: 1 });
  defs += gradient('lacquer', [[0, '#fff8e8', 0], [.13, '#fff8e8', .025 * d.gloss], [.26, '#fff8e8', .17 * d.gloss], [.36, '#fff8e8', .045 * d.gloss], [.52, '#fff8e8', 0], [1, '#fff8e8', 0]], { x1: 0, y1: 0, x2: .82, y2: 1 });
  defs += gradient('rim-sheen', [[0, '#ffeed0', .24 * d.gloss], [.35, '#ffeed0', .02], [1, '#201307', .10]]);
  defs += gradient('white-piece', [[0, '#fffbed'], [.4, '#f5edda'], [.85, '#e9dfc8'], [1, '#c0b292']], { x2: .4, y2: 1 });
  defs += gradient('black-piece', [[0, '#66503d'], [.4, '#4d392b'], [.85, '#443024'], [1, '#2f2119']], { x2: .4, y2: 1 });

  let body = '';
  for (let i = 5; i >= 1; i--) body += rect(-i * .8, i * 1.15, S + i * 1.6, S + i * .6, '#271b10', { rx: 12 + i, opacity: .017 });
  body += rect(0, 0, S, S, 'url(#frame)', { rx: 12 });
  let rim = grain(S, FRAME, d.seed + 100, d.grain * .7, .6) +
    tag('g', { transform: `translate(0 ${S - FRAME})` }, grain(S, FRAME, d.seed + 101, d.grain * .7, .6)) +
    grain(FRAME, S, d.seed + 102, d.grain * .7, .6, true) +
    tag('g', { transform: `translate(${S - FRAME} 0)` }, grain(FRAME, S, d.seed + 103, d.grain * .7, .6, true));
  rim += rect(1.5, 1.5, S - 3, S - 3, 'url(#rim-sheen)', { rx: 11 });
  for (const [x, y, dx, dy] of [[0, 0, 1, 1], [S, 0, -1, 1], [0, S, 1, -1], [S, S, -1, -1]]) {
    rim += path([[x, y], [x + dx * FRAME, y + dy * FRAME]], '#201509', .7, .16);
  }
  body += tag('g', { 'clip-path': 'url(#outer-clip)' }, rim);

  let cells = '';
  const rand = random(d.seed);
  for (let row = 0; row < 8; row++) for (let column = 0; column < 8; column++) {
    const dark = (row + column) % 2 === 1;
    const seed = d.seed * 997 + row * 137 + column * 47;
    const x = FRAME + column * CELL;
    const y = FRAME + row * CELL;
    const fill = d.kind === 'solid' ? d[dark ? 'dark' : 'light'] : `url(#${dark ? 'dark' : 'light'})`;
    let cell = rect(0, 0, CELL, CELL, fill, { 'data-cell': `${column},${row}`, 'data-tone': dark ? 'dark' : 'light' });
    if (d.kind !== 'solid') {
      cell += rect(0, 0, CELL, CELL, rand() > .5 ? '#f7dfb3' : '#241509', { opacity: f(rand() * .026) });
      let texture;
      if (d.kind === 'endgrain') texture = endGrain(seed, d.grain);
      else if (d.kind === 'parquet') {
        texture = tag('g', { transform: `translate(${CELL / 2} ${CELL / 2}) rotate(${dark ? -45 : 45}) translate(${-CELL} ${-CELL})` },
          grain(CELL * 2, CELL * 2, seed, d.grain, d.wave) +
          [CELL * .7, CELL, CELL * 1.3].map(v => path([[0, v], [CELL * 2, v]], '#493321', .6, .12)).join(''));
      } else texture = grain(CELL, CELL, seed, d.grain, d.wave, dark !== Boolean(d.axis), d.kind === 'figured');
      cell += tag('g', { 'clip-path': 'url(#cell-clip)' }, texture);
    }
    cells += tag('g', { transform: `translate(${x} ${y})` }, cell);
  }
  body += cells;
  if (d.kind === 'solid') body += tag('g', { 'clip-path': 'url(#field-clip)' },
    tag('g', { transform: `translate(${FRAME} ${FRAME})` }, grain(FIELD, FIELD, d.seed, d.grain, d.wave, Boolean(d.axis))));
  body += rect(FRAME, FRAME, FIELD, FIELD, 'url(#surface-light)');
  body += rect(FRAME, FRAME, FIELD, FIELD, 'url(#lacquer)');
  if (d.grid) {
    for (let i = 1; i < 8; i++) {
      const p = FRAME + CELL * i;
      body += path([[p, FRAME], [p, S - FRAME]], '#544b38', .65, .17);
      body += path([[FRAME, p], [S - FRAME, p]], '#544b38', .65, .17);
    }
  }
  body += rect(FRAME - 1, FRAME - 1, FIELD + 2, FIELD + 2, 'none', { stroke: '#2b1b0e', 'stroke-opacity': .43, 'stroke-width': 2 });
  body += rect(FRAME - 2.7, FRAME - 2.7, FIELD + 5.4, FIELD + 5.4, 'none', { stroke: d.inlay ? tone(d.light, -.1) : '#ffedc6', 'stroke-opacity': d.inlay ? .64 : .17, 'stroke-width': d.inlay ? 1.4 : .8 });
  body += rect(1, 1, S - 2, S - 2, 'none', { rx: 12, stroke: '#2b1c10', 'stroke-opacity': .35, 'stroke-width': 1.5 });
  body += path([[13, 3], [S - 13, 3]], '#ffedcb', 1.3, .35);
  body += path([[3, 13], [3, S - 13]], '#ffedcb', 1.1, .22);
  body += path([[13, S - 2], [S - 13, S - 2]], '#27180c', 1.5, .24);
  const labelColor = d.id === '03' ? '#3c3020' : '#eadbc1';
  let labels = '';
  for (let i = 0; i < 8; i++) {
    const middle = FRAME + (i + .5) * CELL;
    for (const y of [FRAME / 2, S - FRAME / 2]) labels += tag('text', { x: middle, y }, String.fromCharCode(97 + i));
    for (const x of [FRAME / 2, S - FRAME / 2]) labels += tag('text', { x, y: middle }, 8 - i);
  }
  body += tag('g', { fill: labelColor, 'font-family': 'Arial, sans-serif', 'font-size': 24, 'font-weight': 400, 'text-anchor': 'middle', 'dominant-baseline': 'central', opacity: .88 }, labels);
  if (includePieces) body += pieces();
  return tag('svg', { xmlns: 'http://www.w3.org/2000/svg', viewBox: '-18 -18 1036 1036', role: 'img', 'aria-label': `${d.id}. ${d.name}`, class: 'board-svg', 'data-design': d.id },
    tag('title', {}, `${d.id}. ${d.name} — ${d.description}`) + tag('defs', {}, defs) + body)
    // SVG fragment identifiers share the HTML document's namespace in the gallery.
    .replace(/id="([^"]+)"/g, `id="board-${d.id}-$1"`)
    .replace(/url\(#([^)]+)\)/g, `url(#board-${d.id}-$1)`);
}

const css = `
*{box-sizing:border-box} :root{color-scheme:light;font-family:Arial,Helvetica,sans-serif;color:#3a2d22;background:#e9e2d6}
body{margin:0} button,a{font:inherit} button{cursor:pointer} .wrap{max-width:1680px;margin:auto;padding:56px 48px 40px}
.eyebrow{font-size:12px;letter-spacing:2.6px;text-transform:uppercase;color:#7c6d5b;margin:0 0 15px}
header{display:flex;justify-content:space-between;align-items:flex-end;margin-bottom:32px;gap:28px}
h1{font-family:Georgia,'Times New Roman',serif;font-size:48px;line-height:1.08;font-weight:400;letter-spacing:-1.4px;margin:0 0 13px}
.intro{font-size:15px;line-height:1.55;color:#7b6a58;margin:0;max-width:710px}.controls{display:flex;gap:8px;flex-shrink:0}
button{border:1px solid #c6b8a2;background:transparent;border-radius:7px;padding:10px 15px;color:#554532}
button.active{background:#5e4832;color:#fbf4e8;border-color:#5e4832}button:focus-visible,a:focus-visible{outline:2px solid #6c7654;outline-offset:4px}
.grid{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:26px 24px}
.card{min-width:0;background:#f3ede3;border:1px solid #d3c7b5;border-radius:10px;overflow:hidden;color:inherit;text-decoration:none;display:block}
.card:focus-visible{outline-offset:3px}.board-wrap{background:#d6c9b2;padding:17px 17px 12px}.board-svg{width:100%;height:auto;display:block}
.caption{padding:18px 20px 20px}.title-line{display:flex;align-items:baseline;gap:12px}.number{font-size:13px;font-weight:bold;color:#8b7253;font-variant-numeric:tabular-nums}
h2{font-family:Georgia,'Times New Roman',serif;font-size:23px;letter-spacing:-.4px;line-height:1.2;font-weight:400;margin:0}
.note{color:#8b7961;font-size:12px;margin:7px 0 0 30px}.desc{font-size:13px;line-height:1.5;margin:11px 0 0;color:#7b6a58;min-height:40px}
.pieces{display:none}.with-pieces .pieces{display:inline}footer{display:flex;justify-content:space-between;gap:20px;margin-top:27px;padding-top:19px;border-top:1px solid #ccbfab;color:#8b7b67;font-size:12px;line-height:1.5}
.single.wrap{max-width:1280px;padding:35px 56px 28px}.single header{margin-bottom:20px}.single h1{font-size:38px}.single .intro{font-size:15px}
.single .card{max-width:1120px;margin:auto}.single .board-wrap{padding:22px}.single .caption{padding:20px 28px}.single h2{font-size:27px}
.single .desc{min-height:0;margin-top:9px}.single .note{font-size:13px}.single footer{margin-top:20px}
.back{color:#78654d;text-decoration:none;font-size:13px;display:inline-block;margin-bottom:15px}
@media(max-width:900px){.wrap{padding:28px 20px}header{display:block}h1{font-size:38px}.controls{margin-top:20px}.grid{grid-template-columns:repeat(2,minmax(0,1fr));gap:16px}.caption{padding:14px}h2{font-size:21px}.board-wrap{padding:9px}.single.wrap{padding:24px}.single .board-wrap{padding:12px}}
@media(max-width:540px){.grid{grid-template-columns:1fr}footer{display:block}footer span{display:block;margin:5px 0}}
@media print{.controls,.back{display:none}.wrap{padding:12px}.card{break-inside:avoid}}
`;

function card(d, clickable) {
  const body = `<div class="board-wrap">${boardSvg(d)}</div><div class="caption"><div class="title-line"><span class="number">${d.id}</span><h2>${d.name}</h2></div><p class="note">${d.note}</p><p class="desc">${d.description}</p></div>`;
  return clickable ? `<a class="card" href="${d.id}.html" aria-label="Открыть вариант ${d.id}: ${d.name}">${body}</a>` : `<article class="card">${body}</article>`;
}
function html(selected) {
  const single = Boolean(selected);
  return `<!doctype html>
<html lang="ru"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>${single ? `${selected.id} · ${selected.name}` : '12 деревянных досок'} · rucheckers</title><style>${css}</style></head>
<body><main class="wrap${single ? ' single' : ''}">
${single ? '<a class="back" href="index.html">← Все варианты</a>' : ''}
<header><div><p class="eyebrow">Rucheckers · исследование материалов</p><h1>${single ? `${selected.id} / ${selected.name}` : 'Дерево. Свет. Лак.'}</h1><p class="intro">${single ? selected.description : '12 вариантов деревянной доски. От цельного массива до наборных клеток.<br>Мягкие волокна, естественные оттенки и сдержанный блеск.'}</p></div><div class="controls" aria-label="Вид доски"><button id="empty" class="active" aria-pressed="true">Без шашек</button><button id="pieces" aria-pressed="false">С шашками</button></div></header>
${single ? card(selected, false) : `<section class="grid">${designs.map(d => card(d, true)).join('')}</section>`}
<footer><span>Пропорции доски и координаты — rucheckers / main</span><span>Программная отрисовка · 8 × 8 · без растровых текстур</span></footer></main>
<script>const buttons=[document.querySelector('#empty'),document.querySelector('#pieces')];function showPieces(value){document.body.classList.toggle('with-pieces',value);buttons.forEach((b,i)=>{b.classList.toggle('active',Boolean(i)===value);b.setAttribute('aria-pressed',String(Boolean(i)===value))});}buttons.forEach((b,i)=>b.addEventListener('click',()=>showPieces(Boolean(i))));if(new URLSearchParams(location.search).get('pieces')==='1')showPieces(true);</script>
</body></html>`;
}

export async function generate(directory) {
  await mkdir(directory, { recursive: true });
  await writeFile(resolve(directory, 'index.html'), html());
  for (const d of designs) {
    await writeFile(resolve(directory, `${d.id}.html`), html(d));
    await writeFile(resolve(directory, `${d.id}.svg`), boardSvg(d, { includePieces: false }));
  }
  await writeFile(resolve(directory, 'designs.json'), JSON.stringify(designs, null, 2) + '\n');
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const directory = resolve(process.argv[2] ?? 'work/board-designs');
  await generate(directory);
  console.log(`Generated 12 procedural boards: ${directory}/index.html`);
}
