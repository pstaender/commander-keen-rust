// The game runs here: it keeps its own loop like the DOS original and
// blocks this worker (Atomics.wait) while it waits. Picture, keyboard,
// gamepads and sound go through SharedArrayBuffers shared with the page
// (see src/engine/sys_web.rs).
import init, { layout, start } from './pkg/keen_web.js';

await init();
postMessage({ type: 'ready', layout: layout() });

onmessage = (e) => {
  const m = e.data;
  if (m.type !== 'start') return;
  try {
    start(m.ctrl, m.frame, m.audio, m.files, m.args, (kind, path, data) => {
      if (kind === 'write') postMessage({ type: kind, path, data }, [data.buffer]);
      else postMessage({ type: kind, path });
    });
  } catch (err) {
    postMessage({ type: 'error', path: String(err) });
  }
};
