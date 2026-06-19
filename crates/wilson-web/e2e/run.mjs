// SPDX-License-Identifier: GPL-3.0-or-later
// End-to-end test of the Wilson Reborn web page in a REAL headless Chrome (Playwright),
// driven like a user — validates what unit tests can't.
//
// It adapts to the bundle in `web/`:
//   • embedded build (self-contained, data baked in): FULL test — the engine renders in the
//     browser AND sound actually plays after a user gesture (the autoplay gate).
//     Build it with:  WILSON_EMBED_DATA=<dir-with-RESOURCE.*> ../build-web.sh
//   • bring-your-own build (no data — what CI can build): SMOKE test — the page loads, the
//     wasm initialises, the picker is present, and no JS errors are raised. (The full
//     render+sound test needs the copyright data, so it runs locally, not in CI.)
//     Build it with:  ../build-web.sh
//
// Usage: node run.mjs [web-dir]   (defaults to ../web)
import { chromium } from "playwright";
import http from "node:http";
import { readFile } from "node:fs/promises";
import path from "node:path";

const WEB = path.resolve(process.argv[2] || path.join(import.meta.dirname, "..", "web"));
const MIME = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm", ".css": "text/css" };

// Minimal static server (serves the built web/ with the right MIME types for the wasm).
const server = http.createServer(async (req, res) => {
  try {
    let p = decodeURIComponent(req.url.split("?")[0]);
    if (p === "/") p = "/index.html";
    const buf = await readFile(path.join(WEB, p));
    res.setHeader("Content-Type", MIME[path.extname(p)] || "application/octet-stream");
    res.end(buf);
  } catch {
    res.statusCode = 404;
    res.end("not found");
  }
});
await new Promise((r) => server.listen(0, r));
const url = `http://127.0.0.1:${server.address().port}/`;

const browser = await chromium.launch({ headless: true, args: ["--no-sandbox"] });
const page = await browser.newPage();

// Capture any uncaught JS error (incl. a failed `await init()` of the wasm module).
const errors = [];
page.on("pageerror", (e) => errors.push(String(e.message || e)));
page.on("console", (m) => { if (m.type() === "error") console.log("  [page console.error]", m.text()); });

// Instrument the AudioContext: capture the instance + count sound buffers that actually start.
await page.addInitScript(() => {
  window.__pw = { ctx: null, starts: 0 };
  const AC = window.AudioContext || window.webkitAudioContext;
  if (AC) {
    window.AudioContext = class extends AC {
      constructor(...a) { super(...a); window.__pw.ctx = this; }
    };
  }
  const realStart = AudioBufferSourceNode.prototype.start;
  AudioBufferSourceNode.prototype.start = function (...a) { window.__pw.starts++; return realStart.apply(this, a); };
});

// Simulate the browser's own F11 fullscreen. Unlike our ⛶ button, F11 does NOT engage the
// Fullscreen API (no `fullscreenchange`, `fullscreenElement` stays null) — it only flips the
// `(display-mode: fullscreen)` media query. We can't press a real F11 in headless Chrome, so we
// stub THAT one query (everything else passes through to the real matchMedia) and let the test
// toggle it via `window.__setF11`. This drives exactly the signal the page reacts to. (That Chrome
// flips the query on F11 is browser behaviour, confirmed by the user in a real browser.)
await page.addInitScript(() => {
  const real = window.matchMedia.bind(window);
  const listeners = new Set();
  let on = false;
  window.__setF11 = (v) => {
    on = !!v;
    const ev = { matches: on, media: "(display-mode: fullscreen)" };
    listeners.forEach((cb) => { try { (cb.handleEvent || cb).call(cb, ev); } catch {} });
  };
  window.matchMedia = (q) =>
    /display-mode/.test(q)
      ? { get matches() { return on; }, media: q, onchange: null,
          addEventListener: (_t, cb) => listeners.add(cb), removeEventListener: (_t, cb) => listeners.delete(cb),
          addListener: (cb) => listeners.add(cb), removeListener: (cb) => listeners.delete(cb) }
      : real(q);
});

const poll = async (fn, ms, step = 200) => {
  const end = Date.now() + ms;
  while (Date.now() < end) { if (await fn()) return true; await page.waitForTimeout(step); }
  return false;
};
const cleanup = async () => { await browser.close(); server.close(); };
const fail = async (msg) => { console.error("E2E FAIL:", msg); await cleanup(); process.exit(1); };

console.log("serving", WEB, "→", url);
// Deterministic + fast: a fixed seed (so cues are reproducible) and 4× speed (so the engine's
// sound cues fire within a few wall-clock seconds rather than depending on a random scene).
await page.goto(url + "?seed=0&speed=400", { waitUntil: "load" });

// The embedded (self-contained) build hides the picker and auto-runs; bring-your-own shows it.
const embedded = await poll(() => page.evaluate(() => document.getElementById("panel")?.hidden === true), 6000);

if (embedded) {
  console.log("mode: EMBEDDED build — full render + sound test");

  // 1) The engine runs in-browser: the canvas fills with non-black pixels (Johnny + island).
  const rendered = await poll(
    () => page.evaluate(() => {
      const c = document.getElementById("screen");
      if (!c) return false;
      const d = c.getContext("2d").getImageData(0, 0, c.width, c.height).data;
      let n = 0;
      for (let i = 0; i < d.length; i += 4) if (d[i] || d[i + 1] || d[i + 2]) n++;
      return n > 2000;
    }),
    20000,
  );
  if (!rendered) await fail("the canvas never rendered non-black pixels (engine did not run)");
  console.log("OK ✅  canvas renders the animation in-browser");

  // 2) Audio is gated before any gesture (the browser autoplay policy).
  console.log("audio state before gesture:", await page.evaluate(() => (window.__pw.ctx ? window.__pw.ctx.state : "none")));

  // 3) After a real user gesture, audio resumes AND sound effects actually play.
  await page.mouse.click(200, 150);
  if (!(await poll(() => page.evaluate(() => window.__pw.ctx && window.__pw.ctx.state === "running"), 8000)))
    await fail("AudioContext did not resume after a click");
  console.log("OK ✅  audio context resumed after the click");

  if (!(await poll(() => page.evaluate(() => window.__pw.starts > 0), 25000)))
    await fail("no sound buffer ever played after resume (no cues fired)");
  console.log("OK ✅  sound effects actually play (buffer sources started:", await page.evaluate(() => window.__pw.starts), ")");

  // 4) The browser's own F11 fullscreen presents like our ⛶ button: the stage grows to fill the
  //    screen and the page chrome (the title bar) hides — then reverts when F11 is released.
  const before = await page.evaluate(() => {
    const s = document.getElementById("stage").getBoundingClientRect();
    return { fs: document.body.classList.contains("fs"), w: s.width, vw: innerWidth };
  });
  if (before.fs) await fail("body.fs was set before F11");
  if (before.w >= before.vw) await fail("the stage already filled the width before F11 (unexpected)");
  await page.evaluate(() => window.__setF11(true));
  if (!(await poll(() => page.evaluate(() => {
    const s = document.getElementById("stage").getBoundingClientRect();
    return document.body.classList.contains("fs") &&
      Math.abs(s.width - innerWidth) < 2 && Math.abs(s.height - innerHeight) < 2 &&
      getComputedStyle(document.querySelector("h1")).display === "none";
  }), 3000)))
    await fail("F11 did not fill the stage / hide the title bar (like the ⛶ button)");
  console.log("OK ✅  F11 fills the screen and hides the page chrome (like the ⛶ button)");
  await page.evaluate(() => window.__setF11(false));
  if (!(await poll(() => page.evaluate(() =>
    !document.body.classList.contains("fs") &&
    getComputedStyle(document.querySelector("h1")).display !== "none"), 3000)))
    await fail("leaving F11 did not restore the windowed layout");
  console.log("OK ✅  leaving F11 restores the windowed layout");
} else {
  console.log("mode: BRING-YOUR-OWN build — smoke test (no game data)");

  // The page loads in a real browser, the wasm module initialises (a failed `init()` would
  // surface as a pageerror), and the data picker (file input) is present.
  const ui = await page.evaluate(() => !!document.getElementById("files"));
  if (!ui) await fail("the data picker UI is missing (page/bundle broken?)");
  await page.waitForTimeout(1500); // let `await init()` settle so a wasm failure would have thrown
  console.log("OK ✅  page loads, wasm module initialises, picker present");
}

if (errors.length) await fail("page raised JS errors: " + errors.join(" | "));
console.log("OK ✅  no uncaught JS errors");

console.log(`\nE2E PASSED ✅  (${embedded ? "full: render + sound" : "smoke"}, real headless Chrome)`);
await cleanup();
