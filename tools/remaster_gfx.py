#!/usr/bin/env python3
"""Remasters the graphics of a `--dump` export with an OpenAI image model.

The graphics are packed into sheets (upscaled 4x with nearest neighbour,
1536x1024), each sheet is sent to the image edit endpoint together with
reference pairs (original sheet -> remastered sheet) so all sheets get the
same color mapping, and the result is cut back into the original pixel grid
(median of the inner 2x2 of every 4x4 block; the original alpha is kept, so
silhouettes and masks stay exact). The output is a `--gfx-pack` with
scale=1.

Usage: python3 -I tools/remaster_gfx.py DUMP WORK PACK [--only NAME,...]
  DUMP  output of `keen6 --dump DIR`
  WORK  sheets (*_in.png / *_out.png); existing *_out.png are reused, so a
        run can be resumed and single sheets redone by deleting them
  PACK  output pack directory
Env: OPENAI_ACCESS_TOKEN, REMASTER_MODEL (default gpt-image-2.5-sunburst),
REMASTER_REFS (sheet names used as references, default below).
"""
import base64, json, os, statistics, subprocess, sys
from concurrent.futures import ThreadPoolExecutor
import numpy as np
from PIL import Image

S, W, H = 4, 1536, 1024
GREY = (128, 128, 128, 255)
BLACK = (0, 0, 0, 255)
MODEL = os.environ.get('REMASTER_MODEL', 'gpt-image-2.5-sunburst')
REFS = os.environ.get('REMASTER_REFS', 'tile16_01,tile16m_02,sprites_keen').split(',')
MIN_MATCH = 0.75  # luminance correlation of a sheet with its original below which it is redone

PROMPT = (
    "The first image is a pixel-art sheet from the 1991 DOS game Commander Keen 6, drawn in the 16-color EGA palette and "
    "upscaled 4x with nearest neighbour (every 4x4 block is one original pixel). Remaster ONLY the colors and shading in the "
    "style of 16-bit SNES games like Super Mario World / Yoshi's Island: a rich modern palette, smooth multi-step shading with "
    "a consistent light from the top-left, soft highlights and darker outlines, saturated but harmonious colors. Replace the EGA "
    "checkerboard dithering with clean solid shading. Keep it crisp pixel art at exactly the same pixel grid (4x4 blocks): do NOT "
    "move, resize, redraw, add or remove anything; every shape, outline, silhouette, letter and detail must stay at exactly the "
    "same position. No new text, no blur, no anti-aliasing into the background. ")
REF_PROMPT = (
    "The other images are reference pairs from the same game: an original sheet followed by its finished remaster. Use exactly "
    "the same color mapping and shading style as these references: the same original colors and materials (rock, ground, "
    "plants, metal, water, Keen's clothes, outlines) must become the same remastered colors, so everything looks consistent "
    "when combined in the game. Only edit the first image and keep its own content: never copy shapes or objects from the references. ")
EXTRA = {
    'tile16': "The first image is a grid of 16x16 background tiles from a tileset; neighbouring tiles join seamlessly. Keep tile "
              "edges continuous and the layout identical. Keep pure black areas pure black. Repeating fill textures (ground, walls, sky) must stay subtle and low-contrast so the repetition is not visible.",
    'tile16m': "The first image is a grid of 16x16 foreground tiles (platforms, decorations) from a tileset; neighbouring tiles "
               "join seamlessly. Keep the flat neutral grey background (#808080) exactly as it is; it is transparent in the game.",
    'sprites': "Keep the flat neutral grey background (#808080) exactly as it is; only recolor the characters and objects.",
    'pattern': "The first image has panels of seamlessly repeating fill textures (walls, ground, sky), each made of one small tile "
               "pattern repeated many times. Remaster each panel as one continuous seamless surface: the same pattern must repeat "
               "identically in every repetition, with uniform lighting across the whole panel (no gradient, no vignette, no variation "
               "between repetitions), and subtle, low-contrast detail so the repetition is not obvious. Keep the flat neutral grey "
               "(#808080) between the panels exactly as it is.",
    'pics': "These are screens, menu graphics and pictures; keep all text and letters exactly readable and in place. Keep the "
            "flat neutral grey background (#808080) between the pictures exactly as it is.",
}


def place(img, name, kind, items, bg, work):
    big = img.resize((img.width * S, img.height * S), Image.NEAREST)
    assert big.width <= W and big.height <= H, name
    c = Image.new('RGBA', (W, H), bg)
    ox, oy = (W - big.width) // 2, (H - big.height) // 2
    c.alpha_composite(big, (ox, oy))
    c.convert('RGB').save(f'{work}/{name}_in.png')
    return {'name': name, 'kind': kind, 'items': items, 'ox': ox, 'oy': oy}


def tile_job(dump, work, kind, chunk, cols=18, rows=12):
    """18x12 tiles in tileset order, so objects made of several tiles stay together."""
    start = chunk * cols * rows
    bg = BLACK if kind == 'tile16' else GREY
    img = Image.new('RGBA', (cols * 16, rows * 16), bg)
    items = []
    for i in range(cols * rows):
        p = f'{dump}/tiles/{kind}/{start + i:04d}.png'
        if os.path.exists(p):
            x, y = (i % cols) * 16, (i // cols) * 16
            img.alpha_composite(Image.open(p).convert('RGBA'), (x, y))
            items.append([f'tiles/{kind}/{start + i:04d}.png', x, y])
    return place(img, f'{kind}_{chunk:02d}', kind, items, bg, work) if items else None


def grid_job(dump, work, name, kind, rels, cols):
    ims = [Image.open(f'{dump}/{r}').convert('RGBA') for r in rels]
    cw = max(i.width for i in ims) + 6
    ch = max(i.height for i in ims) + 6
    rows = (len(ims) + cols - 1) // cols
    img = Image.new('RGBA', (cols * cw, rows * ch), GREY)
    items = []
    for k, (r, im) in enumerate(zip(rels, ims)):
        x = (k % cols) * cw + (cw - im.width) // 2
        y = (k // cols) * ch + (ch - im.height) // 2
        img.alpha_composite(im, (x, y))
        items.append([r, x, y])
    return place(img, name, kind, items, GREY, work)


def shelf_jobs(dump, work, prefix, kind, rels, gap=6):
    """Packs images in index order (animation frames stay on one sheet) into rows."""
    maxw, maxh = W // S, H // S
    jobs, cur, x, y, rowh = [], [], gap, gap, 0

    def flush():
        if cur:
            img = Image.new('RGBA', (maxw, maxh), GREY)
            for r, px, py in cur:
                img.alpha_composite(Image.open(f'{dump}/{r}').convert('RGBA'), (px, py))
            jobs.append(place(img, f'{prefix}_{len(jobs):02d}', kind, list(cur), GREY, work))

    for r in rels:
        w, h = Image.open(f'{dump}/{r}').size
        if x + w + gap > maxw:
            x, y, rowh = gap, y + rowh + gap, 0
        if y + h + gap > maxh:
            flush()
            cur, x, y, rowh = [], gap, gap, 0
        cur.append([r, x, y])
        x += w + gap
        rowh = max(rowh, h)
    flush()
    return jobs


def call(job, work, refs):
    out = f'{work}/{job["name"]}_out.png'
    if os.path.exists(out):
        return
    imgs = [f'{work}/{job["name"]}_in.png']
    for r in refs + [r for r in job.get('refs', []) if r not in refs]:
        if r != job['name']:
            imgs += [f'{work}/{r}_in.png', f'{work}/{r}_out.png']
    prompt = PROMPT + (REF_PROMPT if len(imgs) > 1 else '') + EXTRA[job['kind']]
    args = ['curl', '-s', 'https://api.openai.com/v1/images/edits',
            '-H', f'Authorization: Bearer {os.environ["OPENAI_ACCESS_TOKEN"]}',
            '-F', f'model={MODEL}', '-F', f'prompt={prompt}', '-F', f'size={W}x{H}', '-F', 'quality=high']
    for i in imgs:
        args += ['-F', f'image[]=@{i}']
    for _ in range(3):
        r = subprocess.run(args, capture_output=True, text=True)
        try:
            data = json.loads(r.stdout)['data'][0]['b64_json']
        except Exception:
            print('retry', job['name'], r.stdout[:300], flush=True)
            continue
        open(out, 'wb').write(base64.b64decode(data))
        match = register(work, job['name'])[2]
        if match >= MIN_MATCH:
            print('ok', job['name'], flush=True)
            return
        print(f'retry {job["name"]}: match {match:.3f}', flush=True)
        os.replace(out, f'{work}/{job["name"]}_bad.png')


def _lum(img):
    a = np.asarray(img.convert('RGB')).astype(float) @ [.3, .59, .11]
    return a - a.mean()


def register(work, name):
    """Shift (dx, dy) that aligns the model output with the input sheet (the
    model sometimes moves the whole picture), and the correlation after it."""
    a = _lum(Image.open(f'{work}/{name}_in.png'))
    out = Image.open(f'{work}/{name}_out.png')
    b = _lum(out if out.size == (W, H) else out.resize((W, H), Image.LANCZOS))
    c = np.real(np.fft.ifft2(np.fft.fft2(a) * np.conj(np.fft.fft2(b))))
    dy, dx = np.unravel_index(c.argmax(), c.shape)
    dy, dx = (dy - H if dy > H // 2 else dy), (dx - W if dx > W // 2 else dx)
    # match on 8x8 blocks (2x2 original pixels), so dithering that became
    # smooth shading does not count as a difference
    bs = np.roll(np.roll(b, dy, 0), dx, 1)
    a, bs = (m.reshape(H // 8, 8, W // 8, 8).mean((1, 3)) for m in (a, bs))
    a, bs = a - a.mean(), bs - bs.mean()
    return int(dx), int(dy), float((a * bs).sum() / np.sqrt((a * a).sum() * (bs * bs).sum()))


def cut(job, dump, work, pack):
    out = Image.open(f'{work}/{job["name"]}_out.png').convert('RGB')
    if out.size != (W, H):
        out = out.resize((W, H), Image.LANCZOS)
    dx, dy, _ = register(work, job['name'])
    px = out.load()
    for rel, x, y in job['items']:
        orig = Image.open(f'{dump}/{rel}').convert('RGBA')
        o = orig.load()
        new = Image.new('RGBA', orig.size)
        n = new.load()
        for v in range(orig.height):
            for u in range(orig.width):
                if o[u, v][3] == 0:
                    continue
                bx, by = job['ox'] + (x + u) * S - dx, job['oy'] + (y + v) * S - dy
                cs = [px[min(max(bx + i, 0), W - 1), min(max(by + j, 0), H - 1)] for i in (1, 2) for j in (1, 2)]
                n[u, v] = tuple(int(statistics.median(c[k] for c in cs)) for k in range(3)) + (o[u, v][3],)
        os.makedirs(os.path.dirname(f'{pack}/{rel}'), exist_ok=True)
        new.save(f'{pack}/{rel}')


def load_maps(dump):
    """Tile planes written by --dump (maps/mapNN.txt): [(w, h, bg, fg)]."""
    maps = []
    for f in sorted(os.listdir(f'{dump}/maps')):
        if f.endswith('.txt'):
            lines = open(f'{dump}/maps/{f}').read().split('\n')
            w, h = map(int, lines[0].split())
            rows = [list(map(int, l.split())) for l in lines[1:1 + 2 * h]]
            maps.append((w, h, sum(rows[:h], []), sum(rows[h:], [])))
    return maps


def _tile(dump, kind, t):
    p = f'{dump}/tiles/{kind}/{t:04d}.png'
    return np.asarray(Image.open(p).convert('RGBA')) if os.path.exists(p) else None


def find_patterns(dump, maps, min_count=30):
    """Blocks of tiles that fill areas by repeating (walls, ground, sky): the
    usual right and lower neighbour of every tile in the block is the next
    tile of the block, wrapping around."""
    pats = {}
    for plane, kind in ((2, 'tile16'), (3, 'tile16m')):
        cnt, right, down = {}, {}, {}
        for m in maps:
            w, h, p = m[0], m[1], m[plane]
            for y in range(h):
                for x in range(w):
                    t = p[y * w + x]
                    if kind == 'tile16m' and not t:
                        continue
                    cnt[t] = cnt.get(t, 0) + 1
                    if x + 1 < w:
                        d = right.setdefault(t, {}); d[p[y * w + x + 1]] = d.get(p[y * w + x + 1], 0) + 1
                    if y + 1 < h:
                        d = down.setdefault(t, {}); d[p[(y + 1) * w + x]] = d.get(p[(y + 1) * w + x], 0) + 1

        def nxt(m, t):
            d = m.get(t)
            if not d:
                return None
            n = max(d, key=d.get)
            return n if d[n] >= 0.5 * sum(d.values()) else None

        def period(m, t):
            s = t
            for k in range(1, 7):
                s = nxt(m, s)
                if s is None:
                    return None
                if s == t:
                    return k

        for t, c in cnt.items():
            if c < min_count:
                continue
            px, py = period(right, t), period(down, t)
            if not px or not py:
                continue
            block, row = [], t
            for _ in range(py):
                line, s2 = [], row
                for _ in range(px):
                    line.append(s2)
                    s2 = nxt(right, s2)
                block.append(line)
                row = nxt(down, row)
            if any(nxt(down, block[j][i]) != block[(j + 1) % py][i] for j in range(py) for i in range(px)):
                continue
            ims = [_tile(dump, kind, v) for l in block for v in l]
            if any(im is None or (im[..., 3] < 255).any() for im in ims):
                continue
            if all((im == ims[0][0, 0]).all() for im in ims):
                continue  # one plain color: flatten() handles it
            key = (kind, frozenset(v for l in block for v in l))
            if key not in pats:
                pats[key] = {'kind': kind, 'block': block}
    return list(pats.values())


def pattern_jobs(dump, work, pats, cols=9, rows=6, gap=16):
    """Up to four patterns per sheet, each repeated over a 9x6 tile panel;
    patterns from the same tileset sheet share a sheet, which is also passed
    as a reference so the colors match the tiles around them."""
    groups = {}
    for p in pats:
        groups.setdefault((p['kind'], p['block'][0][0] // 216), []).append(p)
    jobs = []
    for (kind, chunk), ps in sorted(groups.items()):
        for k in range(0, len(ps), 4):
            pw, ph = cols * 16, rows * 16
            img = Image.new('RGBA', (2 * pw + gap, 2 * ph + gap), GREY)
            panels = []
            # every panel is used (patterns repeated); the model tends to copy
            # a reference image instead of editing a mostly empty sheet
            group = ps[k:k + 4]
            for q, p in enumerate(group[i % len(group)] for i in range(4)):
                qx, qy = (q % 2) * (pw + gap), (q // 2) * (ph + gap)
                bh, bw = len(p['block']), len(p['block'][0])
                inst = []
                for ty in range(rows):
                    for tx in range(cols):
                        t = p['block'][ty % bh][tx % bw]
                        img.alpha_composite(Image.fromarray(_tile(dump, p['kind'], t)), (qx + tx * 16, qy + ty * 16))
                        if 0 < tx < cols - 1 and 0 < ty < rows - 1:
                            inst.append([f'tiles/{kind}/{t:04d}.png', qx + tx * 16, qy + ty * 16])
                panels += inst
            job = place(img, f'pattern_{kind}_{chunk:02d}_{k // 4}', 'pattern', panels, GREY, work)
            job['refs'] = [f'{kind}_{chunk:02d}']
            jobs.append(job)
    return jobs


def cut_pattern(job, work, pack):
    """Every tile of a pattern: per-pixel median over its inner repetitions."""
    out = Image.open(f'{work}/{job["name"]}_out.png').convert('RGB')
    if out.size != (W, H):
        out = out.resize((W, H), Image.LANCZOS)
    dx, dy, _ = register(work, job['name'])
    a = np.asarray(out).astype(float)
    inst = {}
    for rel, x, y in job['items']:
        ys = np.clip(job['oy'] + (y + np.arange(16)) * S - dy, 0, H - 4)
        xs = np.clip(job['ox'] + (x + np.arange(16)) * S - dx, 0, W - 4)
        block = np.stack([a[ys + j][:, xs + i] for i in (1, 2) for j in (1, 2)])
        inst.setdefault(rel, []).append(np.median(block, 0))
    for rel, v in inst.items():
        rgb = np.median(np.stack(v), 0).astype(np.uint8)
        Image.fromarray(np.dstack([rgb, np.full((16, 16), 255, np.uint8)])).save(f'{pack}/{rel}')


def repair_key(dump, pack, tol=14):
    """Pixels of masked graphics that the model painted in the grey sheet
    background although the original pixel is not grey get the usual remaster
    of their original color."""
    grey = np.array(GREY[:3])
    egagrey = {(85, 85, 85), (170, 170, 170)}
    for kind in ('tiles/tile16m', 'tiles/tile8m', 'sprites', 'pics'):
        files = sorted(os.listdir(f'{pack}/{kind}'))
        data = []
        for f in files:
            o = np.asarray(Image.open(f'{dump}/{kind}/{f}').convert('RGBA'))
            n = np.asarray(Image.open(f'{pack}/{kind}/{f}').convert('RGBA')).copy()
            data.append((f, o, n))
        acc = {}
        for f, o, n in data:
            ok = (o[..., 3] > 0) & (np.abs(n[..., :3].astype(int) - grey).max(-1) > tol)
            for c, v in zip(map(tuple, o[ok][:, :3]), n[ok][:, :3]):
                acc.setdefault(c, []).append(v)
        usual = {c: np.median(np.array(v), 0).astype(np.uint8) for c, v in acc.items()}
        for f, o, n in data:
            bad = (o[..., 3] > 0) & (np.abs(n[..., :3].astype(int) - grey).max(-1) <= tol)
            bad &= ~np.array([[tuple(p[:3]) in egagrey for p in r] for r in o])
            if bad.any():
                for yx in zip(*np.nonzero(bad)):
                    c = tuple(o[yx][:3])
                    if c in usual:
                        n[yx][:3] = usual[c]
                Image.fromarray(n).save(f'{pack}/{kind}/{f}')


def flatten(dump, pack, maps, skip, share=0.85):
    """Tiles that are one solid color in the original, and almost solid tiles
    that sit next to such tiles in the maps (edges of plain areas): the pixels
    of that color get one flat color per original color (the median of their
    remasters), so large plain areas do not show tile-sized bands; the other
    pixels (edges, rivets) keep their remaster."""
    solid, near = {}, {}
    for kind in ('tile16', 'tile16m', 'tile8'):
        d = f'{pack}/tiles/{kind}'
        for f in sorted(os.listdir(d)) if os.path.isdir(d) else []:
            rel = f'tiles/{kind}/{f}'
            if rel in skip:
                continue
            o = np.asarray(Image.open(f'{dump}/{rel}').convert('RGBA'))
            if not (o[..., 3] == 255).all():
                continue
            vals, counts = np.unique(o.reshape(-1, 4), axis=0, return_counts=True)
            col = tuple(int(v) for v in vals[counts.argmax()])
            if len(vals) == 1:
                solid[rel] = (col, (o == col).all(-1))
            elif counts.max() >= share * counts.sum():
                near[rel] = (col, (o == col).all(-1))
    # neighbours in the maps
    nb = {}
    for m in maps:
        w, h = m[0], m[1]
        for plane, kind in ((m[2], 'tile16'), (m[3], 'tile16m')):
            for y in range(h):
                for x in range(w):
                    rel = f'tiles/{kind}/{plane[y * w + x]:04d}.png'
                    if rel not in near:
                        continue
                    for ax, ay in ((x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)):
                        if 0 <= ax < w and 0 <= ay < h:
                            r2 = f'tiles/{kind}/{plane[ay * w + ax]:04d}.png'
                            e = nb.setdefault(rel, [0, 0])
                            e[1] += 1
                            e[0] += r2 in solid and solid[r2][0] == near[rel][0]
    tiles = dict(solid)
    tiles.update({r: v for r, v in near.items() if r in nb and nb[r][0] >= 0.25 * nb[r][1]})
    groups = {}
    for r, (col, m) in tiles.items():
        groups.setdefault(col, []).append((r, m))
    for col, items in groups.items():
        pix = np.concatenate([np.asarray(Image.open(f'{pack}/{r}').convert('RGBA'))[m][:, :3] for r, m in items])
        flat = np.median(pix, 0).astype(np.uint8)
        for r, m in items:
            n = np.asarray(Image.open(f'{pack}/{r}').convert('RGBA')).copy()
            n[m, :3] = flat
            Image.fromarray(n).save(f'{pack}/{r}')


def main():
    dump, work, pack = sys.argv[1:4]
    only = sys.argv[sys.argv.index('--only') + 1].split(',') if '--only' in sys.argv else None
    os.makedirs(work, exist_ok=True)
    os.makedirs(pack, exist_ok=True)
    jobs = []
    for kind in ('tile16', 'tile16m'):
        n = max(int(f[:4]) for f in os.listdir(f'{dump}/tiles/{kind}')) // 216 + 1
        jobs += [j for j in (tile_job(dump, work, kind, c) for c in range(n)) if j]
    spr = sorted(f'sprites/{f}' for f in os.listdir(f'{dump}/sprites') if f.endswith('.png'))
    keen = [f'sprites/spr_{i:03d}.png' for i in range(6, 46)]
    jobs.append(grid_job(dump, work, 'sprites_keen', 'sprites', keen, 10))
    jobs += shelf_jobs(dump, work, 'sprites', 'sprites', [r for r in spr if r not in keen])
    pics = sorted(f'pics/{f}' for f in os.listdir(f'{dump}/pics') if f.endswith('.png'))
    big = [r for r in pics if Image.open(f'{dump}/{r}').size == (320, 200)]
    for k, r in enumerate(big):
        jobs.append(grid_job(dump, work, f'pics_screen_{k}', 'pics', [r], 1))
    small = [r for r in pics if r not in big]
    small += sorted(f'tiles/{d}/{f}' for d in ('tile8', 'tile8m') for f in os.listdir(f'{dump}/tiles/{d}'))
    jobs += shelf_jobs(dump, work, 'pics', 'pics', small)
    maps = load_maps(dump)
    pjobs = pattern_jobs(dump, work, find_patterns(dump, maps))
    todo = [j for j in jobs + pjobs if not only or j['name'] in only]
    refs = [r for r in REFS if os.path.exists(f'{work}/{r}_out.png')]
    print(len(todo), 'sheets, references:', refs, flush=True)
    with ThreadPoolExecutor(8) as ex:
        list(ex.map(lambda j: call(j, work, refs), todo))
    for j in jobs:
        if not os.path.exists(f'{work}/{j["name"]}_out.png'):
            print('missing', j['name'])
            continue
        dx, dy, match = register(work, j['name'])
        print(f'{j["name"]}: shift {dx},{dy} match {match:.3f}' + (' BAD - delete and redo' if match < MIN_MATCH else ''))
        cut(j, dump, work, pack)
    skip = set()
    for j in pjobs:
        if os.path.exists(f'{work}/{j["name"]}_out.png'):
            dx, dy, match = register(work, j['name'])
            print(f'{j["name"]}: shift {dx},{dy} match {match:.3f}')
            cut_pattern(j, work, pack)
            skip |= {r for r, _, _ in j['items']}
    repair_key(dump, pack)
    flatten(dump, pack, maps, skip)
    open(f'{pack}/pack.txt', 'w').write('scale=1\n')
    print('done')


if __name__ == '__main__':
    main()
