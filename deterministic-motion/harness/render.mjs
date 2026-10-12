#!/usr/bin/env node
/*
 * Deterministic frame renderer for the light-show motion ports.
 *
 * Adapted from the reference spike (~/workspace/claude-anim-spike/render.mjs):
 *   1. Load a scene in headless Chromium with the preview
 *      requestAnimationFrame loop neutralised BEFORE the page script runs.
 *   2. Seek: for i in 0 .. DURATION*FPS-1, call renderFrame(i / FPS) and
 *      capture the canvas as a PNG. No playback ever happens.
 *   3. Prove determinism: re-render three sample frames and compare
 *      SHA-256 hashes against the first pass.
 *   4. Encode with FFmpeg at the scene's own FPS (240 for these ports)
 *      and build a 12-tile contact sheet for visual self-review.
 *
 * Usage: node render.mjs <scene.html> <out.mp4> <contact.png>
 * Frames are written to .frames/<scene-name>/ next to this script and
 * are scratch only — delete .frames before committing.
 */
import puppeteer from 'puppeteer-core';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, writeFileSync, existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { basename, dirname, join } from 'node:path';

const HERE = dirname(fileURLToPath(import.meta.url));
const [sceneArg, outArg, sheetArg] = process.argv.slice(2);
if (!sceneArg || !outArg || !sheetArg) {
  console.error('usage: node render.mjs <scene.html> <out.mp4> <contact.png>');
  process.exit(2);
}
const SCENE = join(HERE, sceneArg);
const OUT_MP4 = join(HERE, outArg);
const SHEET = join(HERE, sheetArg);
const FRAMES_DIR = join(HERE, '.frames', basename(sceneArg, '.html'));

function findChromium() {
  const candidates = [
    process.env.CHROMIUM_PATH,
    '/usr/bin/chromium',
    '/usr/bin/chromium-browser',
    '/usr/bin/google-chrome-stable',
    '/usr/bin/google-chrome',
  ].filter(Boolean);
  for (const c of candidates) if (existsSync(c)) return c;
  throw new Error('No Chromium binary found in: ' + candidates.join(', '));
}

function ffmpeg(args) {
  console.log('$ ffmpeg ' + args.join(' '));
  execFileSync('ffmpeg', args, { stdio: 'inherit' });
}

const sha256 = (buf) => createHash('sha256').update(buf).digest('hex');

async function main() {
  mkdirSync(FRAMES_DIR, { recursive: true });
  const browser = await puppeteer.launch({
    executablePath: findChromium(),
    headless: true,
    args: [
      '--no-sandbox',
      '--disable-dev-shm-usage',
      '--disable-gpu',
      '--force-device-scale-factor=1',
      '--hide-scrollbars',
    ],
  });
  let total = 0;
  let fps = 0;
  try {
    const page = await browser.newPage();
    await page.emulateMediaFeatures([{ name: 'prefers-reduced-motion', value: 'reduce' }]);
    await page.evaluateOnNewDocument(() => {
      window.requestAnimationFrame = () => 0;
      window.cancelAnimationFrame = () => {};
    });
    await page.setViewport({ width: 640, height: 360 });
    page.on('pageerror', (e) => { throw e; });
    page.on('console', (m) => { if (m.type() === 'error') throw new Error('page console: ' + m.text()); });
    await page.goto('file://' + SCENE, { waitUntil: 'load' });
    await page.waitForFunction(() => typeof window.renderFrame === 'function');

    const dims = await page.evaluate(() => {
      const c = document.getElementById('scene');
      return { w: c.width, h: c.height, DURATION: window.DURATION, FPS: window.FPS };
    });
    await page.setViewport({ width: dims.w, height: dims.h });
    fps = dims.FPS;
    total = Math.round(dims.DURATION * dims.FPS);
    console.log(`Scene: ${dims.DURATION}s @ ${dims.FPS}fps = ${total} frames (${dims.w}x${dims.h})`);

    const capture = async (i) => {
      const dataUrl = await page.evaluate((t) => {
        window.renderFrame(t);
        return document.getElementById('scene').toDataURL('image/png');
      }, i / dims.FPS);
      return Buffer.from(dataUrl.split(',')[1], 'base64');
    };

    const verifyFrames = [0, Math.floor(total * 0.375), Math.floor(total * 0.83)];
    const hashes = new Map();
    const started = Date.now();
    for (let i = 0; i < total; i++) {
      const png = await capture(i);
      writeFileSync(join(FRAMES_DIR, `f${String(i).padStart(4, '0')}.png`), png);
      if (verifyFrames.includes(i)) hashes.set(i, sha256(png));
      if ((i + 1) % 240 === 0) console.log(`  rendered ${i + 1}/${total}`);
    }
    const renderSecs = ((Date.now() - started) / 1000).toFixed(1);
    console.log(`Pass 1 done in ${renderSecs}s`);

    console.log('Determinism check (re-render + SHA-256 compare):');
    let deterministic = true;
    for (const i of verifyFrames) {
      const again = sha256(await capture(i));
      const match = again === hashes.get(i);
      deterministic &&= match;
      console.log(`  frame ${String(i).padStart(4, '0')}: ${again.slice(0, 16)}… ${match ? 'MATCH' : 'MISMATCH'}`);
    }
    if (!deterministic) throw new Error('DETERMINISM FAILED: re-rendered frames differ');
    console.log('Determinism: PASS');
  } finally {
    await browser.close();
  }

  ffmpeg(['-y', '-framerate', String(fps), '-i', join(FRAMES_DIR, 'f%04d.png'),
          '-c:v', 'libx264', '-pix_fmt', 'yuv420p', '-crf', '20',
          '-movflags', '+faststart', OUT_MP4]);
  const step = Math.max(1, Math.floor(total / 12));
  ffmpeg(['-y', '-framerate', String(fps), '-i', join(FRAMES_DIR, 'f%04d.png'),
          '-vf', `select='not(mod(n\\,${step}))',scale=320:180,tile=4x3:padding=4:margin=4`,
          '-fps_mode', 'vfr', '-frames:v', '1', SHEET]);
  console.log('Wrote ' + OUT_MP4);
  console.log('Wrote ' + SHEET);
}

main().catch((e) => { console.error(e); process.exit(1); });
