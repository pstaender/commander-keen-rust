// Commander Keen in the browser: the page side.
//
// The game runs in a web worker (worker.js, the WebAssembly build of the
// Rust port) and shares three buffers with this page (layout in
// src/engine/sys_web.rs, sizes and indexes from `layout()`):
//   control  keyboard events, gamepad states, the size of the game area
//   frame    the picture (RGBA), drawn here on every animation frame
//   audio    sound samples, played by an AudioWorklet (audio.js)
// The data files, saved games and config.txt are kept in IndexedDB.

const $ = (id) => document.getElementById(id);
const canvas = $('screen');
const ctx2d = canvas.getContext('2d', { alpha: false });
const overlay = $('overlay');
const params = new URLSearchParams(location.search);
const padDebug = params.has('paddebug');

function show(html) {
  overlay.innerHTML = html;
  overlay.hidden = false;
}

function fail(msg) {
  show(`<h1>Commander Keen</h1><p class="error">${escapeHtml(msg)}</p>`);
  throw new Error(msg);
}

function escapeHtml(s) {
  return String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);
}

let toastTimer = 0;
function toast(msg) {
  const t = $('toast');
  t.textContent = msg;
  t.hidden = false;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (t.hidden = true), 4000);
}

// --- cross-origin isolation (SharedArrayBuffer) ----------------------------

if (!self.crossOriginIsolated) {
  if ('serviceWorker' in navigator && !sessionStorage.getItem('keen-coi-reload')) {
    // no COOP/COEP headers from the server: a service worker adds them
    sessionStorage.setItem('keen-coi-reload', '1');
    await navigator.serviceWorker.register('coi-sw.js');
    await navigator.serviceWorker.ready;
    location.reload();
    await new Promise(() => {});
  }
  fail(
    'This page needs SharedArrayBuffer, which browsers only allow on a cross-origin isolated page ' +
      '(served with "Cross-Origin-Opener-Policy: same-origin" and "Cross-Origin-Embedder-Policy: require-corp", ' +
      'over https or from localhost). web/serve.mjs serves it that way.',
  );
}
sessionStorage.removeItem('keen-coi-reload');

// --- storage (IndexedDB): k<episode>/d/<data file>, k<episode>/w/<written file> ---

const db = await new Promise((resolve, reject) => {
  const r = indexedDB.open('commander-keen', 1);
  r.onupgradeneeded = () => r.result.createObjectStore('files');
  r.onsuccess = () => resolve(r.result);
  r.onerror = () => reject(r.error);
}).catch(() => null);

function idb(mode, f) {
  if (!db) return Promise.resolve(undefined);
  return new Promise((resolve, reject) => {
    const tx = db.transaction('files', mode);
    const r = f(tx.objectStore('files'));
    tx.oncomplete = () => resolve(r && r.result);
    tx.onerror = () => reject(tx.error);
  });
}
const idbPut = (k, v) => idb('readwrite', (s) => s.put(v, k));
const idbDelete = (k) => idb('readwrite', (s) => s.delete(k));
async function idbPrefix(prefix) {
  const range = IDBKeyRange.bound(prefix, prefix + '￿');
  const [keys, values] = await Promise.all([
    idb('readonly', (s) => s.getAllKeys(range)),
    idb('readonly', (s) => s.getAll(range)),
  ]);
  const m = new Map();
  (keys || []).forEach((k, i) => m.set(k.slice(prefix.length), new Uint8Array(values[i])));
  return m;
}

// --- the worker ---------------------------------------------------------

const worker = new Worker('worker.js', { type: 'module' });
const L = await new Promise((resolve) => {
  worker.onmessage = (e) => e.data.type === 'ready' && resolve(e.data.layout);
  worker.onerror = (e) => fail(`The game could not be loaded (${e.message || 'worker error'}). Was web/build.sh run?`);
});

// --- data files -----------------------------------------------------------

/** The episode of a set of data files (by EGAGRAPH.CKn), 0 if none. */
function episodeOf(names) {
  for (const n of names) {
    const m = /^EGAGRAPH\.CK([456])$/i.exec(n);
    if (m) return Number(m[1]);
  }
  return 0;
}

/** Data files served next to the page: data/files.json lists them. */
async function servedData() {
  try {
    const r = await fetch('data/files.json', { cache: 'no-cache' });
    if (!r.ok) return null;
    const names = await r.json();
    const files = new Map();
    await Promise.all(
      names.map(async (n) => {
        const f = await fetch('data/' + encodeURIComponent(n));
        if (f.ok) files.set(n, new Uint8Array(await f.arrayBuffer()));
      }),
    );
    const ep = episodeOf(files.keys());
    return ep ? { ep, files, source: 'from the server' } : null;
  } catch {
    return null;
  }
}

/** Data files stored in this browser (episode `want` or any, Keen 4 first). */
async function storedData(want) {
  for (const ep of want ? [want] : [4, 5, 6]) {
    const files = await idbPrefix(`k${ep}/d/`);
    if (episodeOf(files.keys()) === ep) return { ep, files, source: 'stored in this browser' };
  }
  return null;
}

const DATA_FILE = /\.(CK[456]|EXE)$/i;
const NOT_DATA = /^(CONFIG|SAVEGAM|DEMO)/i;

/** Asks for the original data files (picked or dropped) and stores them. */
function askForData(message) {
  show(`
    <h1>Commander Keen</h1>
    ${message ? `<p class="error">${escapeHtml(message)}</p>` : ''}
    <p>This needs the files of the original game, e.g. the shareware
    <b>Commander Keen 4</b>: <code>EGAGRAPH.CK4</code>, <code>GAMEMAPS.CK4</code>,
    <code>AUDIO.CK4</code> and <code>KEEN4E.EXE</code> (Keen 5 and 6 work too).</p>
    <p>
      <label class="button">Choose the game folder<input id="pick-dir" type="file" webkitdirectory hidden></label>
      <label class="button">Choose files<input id="pick-files" type="file" multiple hidden></label>
    </p>
    <p class="small">…or drop them here. They stay in this browser (nothing is uploaded).</p>`);
  return new Promise((resolve) => {
    const take = async (list) => {
      const files = new Map();
      for (const f of list) {
        const name = f.name.toUpperCase();
        if (DATA_FILE.test(name) && !NOT_DATA.test(name)) files.set(name, new Uint8Array(await f.arrayBuffer()));
      }
      const ep = episodeOf(files.keys());
      if (!ep) {
        resolve(askForData('No EGAGRAPH.CK4, .CK5 or .CK6 among those files.'));
        return;
      }
      for (const [n, d] of files) await idbPut(`k${ep}/d/${n}`, d);
      resolve({ ep, files, source: 'stored in this browser' });
    };
    $('pick-dir').onchange = (e) => take(e.target.files);
    $('pick-files').onchange = (e) => take(e.target.files);
    overlay.ondragover = (e) => e.preventDefault();
    overlay.ondrop = async (e) => {
      e.preventDefault();
      const list = [];
      const walk = async (entry) => {
        if (entry.isFile) list.push(await new Promise((r) => entry.file(r)));
        else if (entry.isDirectory) {
          const entries = await new Promise((r) => entry.createReader().readEntries(r));
          for (const x of entries) await walk(x);
        }
      };
      for (const item of e.dataTransfer.items) {
        const entry = item.webkitGetAsEntry && item.webkitGetAsEntry();
        if (entry) await walk(entry);
        else if (item.getAsFile()) list.push(item.getAsFile());
      }
      take(list);
    };
  });
}

const wantEp = Number(params.get('ep')) || 0;
let data = (await servedData()) || (await storedData(wantEp)) || (await askForData());
if (wantEp && data.ep !== wantEp) data = (await storedData(wantEp)) || data;

const TITLES = {
  4: 'Commander Keen 4: Secret of the Oracle',
  5: 'Commander Keen 5: The Armageddon Machine',
  6: 'Commander Keen 6: Aliens Ate My Baby Sitter!',
};
document.title = TITLES[data.ep];

// --- start (a click or key press: browsers only play sound after one) --------

await new Promise((resolve) => {
  show(`
    <h1>${escapeHtml(TITLES[data.ep])}</h1>
    <p class="small">Data files ${data.source}${data.source.includes('browser') ? ' · <a href="#" id="other">use other files</a>' : ''}</p>
    <p><button id="play" class="button big">Play</button></p>
    <p class="small">Keyboard as in the original (arrows, Ctrl jump, Alt pogo, Space fire, F1 help).
    Gamepads: press a button to connect one. F11 or a double click: fullscreen.</p>`);
  const go = () => {
    removeEventListener('keydown', key);
    resolve();
  };
  const key = (e) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      go();
    }
  };
  addEventListener('keydown', key);
  $('play').onclick = go;
  $('play').focus();
  const other = $('other');
  if (other)
    other.onclick = async (e) => {
      e.preventDefault();
      removeEventListener('keydown', key);
      for (const n of data.files.keys()) await idbDelete(`k${data.ep}/d/${n}`);
      location.reload();
    };
});
overlay.hidden = true;

const ctrlSab = new SharedArrayBuffer(L.CONTROL_BYTES);
const frameSab = new SharedArrayBuffer(L.FRAME_BYTES);
const audioSab = new SharedArrayBuffer(L.AUDIO_BYTES);
const ctrl = new Int32Array(ctrlSab);
const ctrlF = new Float32Array(ctrlSab);
const frameHdr = new Int32Array(frameSab, 0, 16);

// sound
let audioCtx = null;
try {
  try {
    audioCtx = new AudioContext({ sampleRate: L.SAMPLE_RATE, latencyHint: 'interactive' });
  } catch {
    audioCtx = new AudioContext();
  }
  await audioCtx.audioWorklet.addModule('audio.js');
  const node = new AudioWorkletNode(audioCtx, 'keen-audio', {
    numberOfInputs: 0,
    numberOfOutputs: 1,
    outputChannelCount: [2],
    processorOptions: { sab: audioSab, L, ahead: L.A_AHEAD },
  });
  node.connect(audioCtx.destination);
  if (audioCtx.sampleRate !== L.SAMPLE_RATE) console.warn(`audio runs at ${audioCtx.sampleRate} Hz, the game renders ${L.SAMPLE_RATE} Hz`);
} catch (err) {
  console.warn('no sound:', err);
}
const resumeAudio = () => audioCtx && audioCtx.state !== 'running' && audioCtx.resume();
resumeAudio();
addEventListener('pointerdown', resumeAudio);

// saved games and config.txt of this episode
const prefixW = `k${data.ep}/w/`;
const written = await idbPrefix(prefixW);
const files = [...data.files, ...written];

const args = [];
for (const [k, v] of params) {
  if (k === 'ep' || k === 'paddebug') continue;
  args.push(`--${k}`);
  if (v) args.push(v);
}

worker.onmessage = (e) => {
  const m = e.data;
  switch (m.type) {
    case 'write':
      if (/^screenshots\//.test(m.path) || /^DEMO\d\./i.test(m.path)) download(m.path.split('/').pop(), m.data);
      if (!/^screenshots\//.test(m.path)) idbPut(prefixW + m.path, m.data);
      break;
    case 'remove':
      idbDelete(prefixW + m.path);
      break;
    case 'quit':
      if (document.fullscreenElement) document.exitFullscreen();
      show(`<h1>${escapeHtml(TITLES[data.ep])}</h1><p><button class="button big" onclick="location.reload()">Play again</button></p>`);
      if (audioCtx) audioCtx.close();
      break;
    case 'error':
      fail(m.path);
  }
};
worker.postMessage({ type: 'start', ctrl: ctrlSab, frame: frameSab, audio: audioSab, files, args });

function download(name, bytes) {
  const a = document.createElement('a');
  a.href = URL.createObjectURL(new Blob([bytes]));
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(a.href), 1000);
  toast(`Saved ${name}`);
}

// --- keyboard -----------------------------------------------------------

const held = new Set();
function sendKey(sc, down, repeat) {
  const w = Atomics.load(ctrl, L.C_KEY_WRITE);
  Atomics.store(ctrl, L.C_KEYS + ((w >>> 0) % L.KEY_RING), sc | (down ? 0x100 : 0) | (repeat ? 0x200 : 0));
  Atomics.store(ctrl, L.C_KEY_WRITE, (w + 1) | 0);
}

function toggleFullscreen() {
  if (document.fullscreenElement) document.exitFullscreen();
  else document.documentElement.requestFullscreen({ navigationUI: 'hide' }).catch(() => {});
}

addEventListener('keydown', (e) => {
  resumeAudio();
  if (e.code === 'F11') {
    e.preventDefault();
    if (!e.repeat) toggleFullscreen();
    return;
  }
  if (e.metaKey) return; // the browser's shortcuts (Cmd+R, Cmd+Q ...)
  if (e.code === 'Pause') {
    Atomics.store(ctrl, L.C_PAUSE, 1);
    e.preventDefault();
    return;
  }
  const sc = L.keys[e.code];
  if (!sc) return;
  e.preventDefault();
  held.add(sc);
  sendKey(sc, true, e.repeat);
});
addEventListener('keyup', (e) => {
  const sc = L.keys[e.code];
  if (!sc) return;
  e.preventDefault();
  held.delete(sc);
  sendKey(sc, false, false);
});
// keys held while the page loses the focus never see their key up
addEventListener('blur', () => {
  for (const sc of held) sendKey(sc, false, false);
  held.clear();
});
canvas.addEventListener('dblclick', toggleFullscreen);
document.addEventListener('fullscreenchange', () => Atomics.store(ctrl, L.C_FULLSCREEN, document.fullscreenElement ? 1 : 0));

// the mouse pointer hides over the game
let pointerTimer = 0;
addEventListener('pointermove', () => {
  document.body.classList.remove('nocursor');
  clearTimeout(pointerTimer);
  pointerTimer = setTimeout(() => document.body.classList.add('nocursor'), 1500);
});

// --- gamepads (only the page can read them) --------------------------------

addEventListener('gamepadconnected', (e) => {
  const p = e.gamepad;
  toast(`Gamepad connected: ${p.id}${p.mapping === 'standard' ? '' : ' (no standard mapping: set up its buttons in CONFIGURE > USE GAMEPAD)'}`);
  if (padDebug) console.log('gamepad', p.index, p.id, 'mapping:', p.mapping || 'none', 'buttons:', p.buttons.length, 'axes:', p.axes.length);
});
addEventListener('gamepaddisconnected', (e) => toast(`Gamepad disconnected: ${e.gamepad.id}`));

const padLast = [];
function pollPads() {
  const pads = navigator.getGamepads ? [...navigator.getGamepads()].filter((p) => p && p.connected) : [];
  for (let k = 0; k < L.PAD_SLOTS; k++) {
    const b = L.C_PADS + k * L.PAD_STRIDE;
    const p = pads[k];
    if (!p) {
      ctrl[b] = 0;
      continue;
    }
    let mask = 0;
    p.buttons.forEach((btn, i) => {
      if (i < 32 && (btn.pressed || btn.value > 0.5)) mask |= 1 << i;
    });
    const naxes = Math.min(p.axes.length, L.PAD_MAX_AXES);
    for (let i = 0; i < naxes; i++) ctrlF[b + L.PAD_AXES_AT + i] = p.axes[i];
    ctrl[b + 1] = p.mapping === 'standard' ? 1 : 0;
    ctrl[b + 2] = p.buttons.length;
    ctrl[b + 3] = naxes;
    ctrl[b + 4] = mask;
    ctrl[b] = 1;
    if (padDebug) {
      const s = `buttons ${(mask >>> 0).toString(2)} axes ${p.axes.map((v) => v.toFixed(2)).join(' ')}`;
      if (s !== padLast[k]) console.log(`pad ${k} (${p.id}): ${s}`);
      padLast[k] = s;
    }
  }
}

// --- picture --------------------------------------------------------------

let image = null;
let lastSeq = -1;
let border = -1;
let shape = '';

function layoutCanvas() {
  Atomics.store(ctrl, L.C_AREA_W, innerWidth);
  Atomics.store(ctrl, L.C_AREA_H, innerHeight);
  const dw = Atomics.load(frameHdr, L.F_DISP_W);
  const dh = Atomics.load(frameHdr, L.F_DISP_H);
  if (!dw || !dh) return;
  const s = `${dw}x${dh} ${innerWidth}x${innerHeight}`;
  if (s === shape) return;
  shape = s;
  // fit the display shape (4:3 correction included) into the window
  const scale = Math.min(innerWidth / dw, innerHeight / dh);
  canvas.style.width = `${Math.round(dw * scale)}px`;
  canvas.style.height = `${Math.round(dh * scale)}px`;
}
addEventListener('resize', layoutCanvas);
layoutCanvas();

function drawFrame() {
  const seq = Atomics.load(frameHdr, L.F_SEQ);
  if (seq === lastSeq || seq & 1) return;
  const w = frameHdr[L.F_W];
  const h = frameHdr[L.F_H];
  if (!w || !h) return;
  if (!image || image.width !== w || image.height !== h) {
    image = new ImageData(w, h);
    canvas.width = w;
    canvas.height = h;
  }
  image.data.set(new Uint8Array(frameSab, L.F_PIXELS, w * h * 4));
  // written meanwhile: take the next one
  if (Atomics.load(frameHdr, L.F_SEQ) !== seq) return;
  lastSeq = seq;
  ctx2d.putImageData(image, 0, 0);
  const b = frameHdr[L.F_BORDER];
  if (b !== border) {
    border = b;
    document.body.style.background = `#${(b & 0xffffff).toString(16).padStart(6, '0')}`;
  }
  layoutCanvas();
}

function tick() {
  pollPads();
  drawFrame();
  requestAnimationFrame(tick);
}
requestAnimationFrame(tick);
// (animation frames stop in a hidden tab, the gamepads are read anyway)
setInterval(() => document.hidden && pollPads(), 50);
