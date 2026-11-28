#!/usr/bin/env node
// Serves web/dist on http://localhost:8080 with the headers that make the
// page cross-origin isolated (SharedArrayBuffer). No dependencies:
//
//   node web/serve.mjs [PORT] [DIR]     (or: bun web/serve.mjs ...)
import { createServer } from 'node:http';
import { readFile, stat } from 'node:fs/promises';
import { extname, join, normalize, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const port = Number(process.argv[2]) || 8080;
const root = resolve(process.argv[3] || join(fileURLToPath(new URL('.', import.meta.url)), 'dist'));

const TYPES = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.wasm': 'application/wasm',
  '.json': 'application/json',
  '.png': 'image/png',
  '.css': 'text/css; charset=utf-8',
};

const HEADERS = {
  'Cross-Origin-Opener-Policy': 'same-origin',
  'Cross-Origin-Embedder-Policy': 'require-corp',
  'Cache-Control': 'no-cache',
};

createServer(async (req, res) => {
  try {
    const path = decodeURIComponent(new URL(req.url, 'http://x').pathname);
    let file = normalize(join(root, path));
    // stay inside the served directory
    if (file !== root && !file.startsWith(root + sep)) throw Object.assign(new Error(), { code: 'ENOENT' });
    if ((await stat(file)).isDirectory()) file = join(file, 'index.html');
    const body = await readFile(file);
    res.writeHead(200, { ...HEADERS, 'Content-Type': TYPES[extname(file).toLowerCase()] || 'application/octet-stream' });
    res.end(req.method === 'HEAD' ? undefined : body);
  } catch (err) {
    const missing = err.code === 'ENOENT' || err.code === 'ENOTDIR';
    res.writeHead(missing ? 404 : 500, { ...HEADERS, 'Content-Type': 'text/plain' });
    res.end(missing ? 'not found' : 'error');
  }
  console.log(`${res.statusCode} ${req.method} ${req.url}`);
}).listen(port, 'localhost', () => console.log(`serving ${root} on http://localhost:${port}/`));
