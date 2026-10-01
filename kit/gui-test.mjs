// Play-test a terminal Tetris through a real browser terminal.
// usage: node gui-test.mjs <binary> <project dir> <out dir> [port]
// Runs the binary under ttyd (xterm.js in the page), drives it with Playwright
// key presses, reads the terminal's text buffer, saves screenshots, and writes
// <out>/gui.json with one pass/fail per check.
import { chromium } from 'playwright';
import { spawn, execSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const [bin, cwd, out, portArg] = process.argv.slice(2);
const port = Number(portArg || 7690);
fs.mkdirSync(out, { recursive: true });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const res = { checks: {}, notes: [] };

const ttyd = spawn('/home/gud/bench-harnesses/bin/ttyd',
  ['-W', '-p', String(port), '-i', '127.0.0.1', '-t', 'fontSize=14', '-t', 'disableLeaveAlert=true',
   '-t', 'disableReconnect=true', bin],
  { cwd, env: { ...process.env, TERM: 'xterm-256color' }, stdio: ['ignore', 'ignore', 'pipe'] });
let ttydLog = '';
ttyd.stderr.on('data', (d) => { ttydLog += d.toString(); });

// Is the game process (a child of ttyd) still running?
const gameAlive = () => {
  try {
    return execSync(`ps --ppid ${ttyd.pid} -o pid=`).toString().trim().length > 0;
  } catch { return false; }
};

let browser;
let shot = 0;
try {
  await sleep(700);
  browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1100, height: 760 } });
  await page.goto(`http://127.0.0.1:${port}/`);
  await page.waitForFunction(() => window.term && window.term.buffer, null, { timeout: 15000 });
  await page.click('body');

  const screen = () => page.evaluate(() => {
    const b = window.term.buffer.active; const lines = [];
    for (let i = 0; i < window.term.rows; i++) lines.push(b.getLine(b.viewportY + i)?.translateToString(true) ?? '');
    return lines.join('\n');
  });
  const snap = async (name) => {
    const file = `${String(++shot).padStart(2, '0')}-${name}.png`;
    await page.screenshot({ path: path.join(out, file) });
    fs.writeFileSync(path.join(out, file.replace('.png', '.txt')), await screen());
  };
  const view = async () => ({ t: await screen(), p: (await page.screenshot()).toString('base64') });
  const changed = (a, b) => a.t !== b.t || a.p !== b.p;
  const press = async (key, times = 1, gap = 60) => { for (let i = 0; i < times; i++) { await page.keyboard.press(key); await sleep(gap); } };
  const inked = (s) => (s.match(/[^\s]/g) || []).length;
  const score = (s) => { const m = s.match(/score[^0-9]{0,12}(\d+)/i); return m ? Number(m[1]) : null; };
  const lines = (s) => { const m = s.match(/lines?[^0-9]{0,12}(\d+)/i); return m ? Number(m[1]) : null; };

  // 1. starts and draws a board
  await sleep(2500);
  let s0 = await screen();
  res.checks.starts = gameAlive();
  res.checks.draws_board = inked(s0) > 150;
  res.notes.push(`score shown at start: ${score(s0)}`);
  await snap('start');

  // 2. responds to left/right, and survives hitting both walls
  let before = await view(); await press('ArrowLeft'); await sleep(150);
  res.checks.moves_left = changed(before, await view());
  await press('ArrowLeft', 14); await sleep(200);
  res.checks.survives_left_wall = gameAlive();
  await snap('left-wall');
  before = await view(); await press('ArrowRight'); await sleep(150);
  res.checks.moves_right = changed(before, await view());
  await press('ArrowRight', 16); await sleep(200);
  res.checks.survives_right_wall = gameAlive();
  await snap('right-wall');

  // 3. rotates (Up, else common alternatives), including next to a wall
  before = await view(); await press('ArrowUp'); await sleep(150);
  let rotated = changed(before, await view());
  for (const k of ['x', 'w', 'z', 'k']) {
    if (rotated) break;
    before = await view(); await press(k); await sleep(150);
    rotated = changed(before, await view());
    if (rotated) res.notes.push(`rotate key: ${k}`);
  }
  res.checks.rotates = rotated;
  await press('ArrowUp', 4); await sleep(150);
  res.checks.survives_rotate_at_wall = gameAlive();
  await snap('rotated');

  // 4. gravity: the piece falls on its own
  before = await view(); await sleep(2500);
  res.checks.gravity = changed(before, await view());
  await snap('gravity');

  // 5. hard drop (Space), and a new piece appears
  before = await view(); await press(' '); await sleep(400);
  res.checks.hard_drop = changed(before, await view()) && gameAlive();
  await snap('hard-drop');

  // 6. stress: many pieces with random moves until the stack tops out
  const keys = ['ArrowLeft', 'ArrowRight', 'ArrowUp'];
  let seed = 7; const rnd = () => (seed = (seed * 1103515245 + 12345) % 2147483648) / 2147483648;
  let gameOverSeen = false; let maxScore = score(await screen()) ?? 0; let maxLines = 0;
  for (let i = 0; i < 70 && gameAlive(); i++) {
    for (let j = 0; j < 1 + Math.floor(rnd() * 5); j++) await press(keys[Math.floor(rnd() * 3)], 1, 30);
    await press(' '); await sleep(120);
    const s = await screen();
    maxScore = Math.max(maxScore, score(s) ?? 0);
    maxLines = Math.max(maxLines, lines(s) ?? 0);
    if (/game\s*over/i.test(s)) { gameOverSeen = true; break; }
  }
  res.checks.survives_stress = gameAlive();
  res.checks.game_over_shown = gameOverSeen;
  res.checks.score_increases = maxScore > 0;
  res.notes.push(`max score seen ${maxScore}, max lines ${maxLines}`);
  await snap('after-stress');

  // 7. quits cleanly (q, then Esc, then Ctrl-C)
  let quitKey = null;
  for (const k of ['q', 'Escape', 'Control+c']) {
    if (!gameAlive()) break;
    await press(k); await sleep(1500);
    if (!gameAlive()) quitKey = k;
  }
  res.checks.quits = quitKey !== null;
  res.notes.push(`quit key: ${quitKey}`);
} catch (e) {
  res.error = String(e).slice(0, 400);
} finally {
  if (browser) await browser.close();
  ttyd.kill('SIGKILL');
  try { execSync(`pkill -KILL -P ${ttyd.pid}`); } catch {}
}
const c = res.checks;
res.passed = Object.values(c).filter(Boolean).length;
res.total = Object.keys(c).length;
fs.writeFileSync(path.join(out, 'gui.json'), JSON.stringify(res, null, 1));
console.log(JSON.stringify({ passed: res.passed, total: res.total, checks: c, notes: res.notes, error: res.error }));
