#!/usr/bin/env python3
"""Matches the functions of one executable (EP=4|5) to the Keen 6 ones by
comparing their instruction mnemonic sequences (ndisasm), and writes
re/kN/func_match.txt: addr  k6addr  similarity  k6name.
K6 names come from the '/// Name (0xaddr)' doc comments in src/."""
import os, re, subprocess, tempfile, difflib, glob, sys
ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..')
def load(ep):
    exe, hdr, pf = {'4': ('re/k4/KEEN4E_unpacked.exe', 0x2e00, 're/k4/procedures.txt'),
                    '5': ('re/k5/KEEN5E_unpacked.exe', 0x2c00, 're/k5/procedures.txt'),
                    '6': ('re/KEEN6C_unpacked.exe', 0x2c00, 're/procedures.txt')}[ep]
    img = open(os.path.join(ROOT, exe), 'rb').read()[hdr:]
    starts = sorted(int(l.split()[0], 16) for l in open(os.path.join(ROOT, pf)) if 'switchD' not in l and 'caseD' not in l)
    starts = [s for s in starts if s >= 0x1374e]
    out = {}
    for i, s in enumerate(starts):
        e = starts[i + 1] if i + 1 < len(starts) else s + 0x400
        e = min(e, s + 0x3000)
        data = img[s - 0x10000:e - 0x10000]
        with tempfile.NamedTemporaryFile(delete=False) as f:
            f.write(data); name = f.name
        dis = subprocess.run(['ndisasm', '-b16', name], capture_output=True, text=True).stdout
        os.unlink(name)
        toks = []
        for l in dis.splitlines():
            p = l.split(None, 2)
            if len(p) < 3: continue
            ins = p[2]
            m = ins.split()[0]
            # keep mnemonic plus register operands, drop immediates/addresses
            ops = re.sub(r'0x[0-9a-f]+|\b\d+\b', 'N', ins[len(m):])
            toks.append(m + ops.replace(' ', ''))
        out[s] = toks
    return out
ep = os.environ.get('EP', '4')
cache = {}
for e in (ep, '6'):
    cache[e] = load(e)
names = {}
for f in glob.glob(os.path.join(ROOT, 'src/**/*.rs'), recursive=True):
    for m in re.finditer(r'/// ([A-Za-z_0-9]+) \((0x[0-9a-f]+)', open(f).read()):
        names[int(m.group(2), 16)] = m.group(1)
A, B = cache[ep], cache['6']
res = []
bk = list(B)
for a, ta in A.items():
    best = (0, None)
    for b in bk:
        tb = B[b]
        if abs(len(ta) - len(tb)) > max(len(ta), len(tb)) * 0.5 + 4: continue
        sm = difflib.SequenceMatcher(None, ta, tb, autojunk=False)
        if sm.real_quick_ratio() < best[0] or sm.quick_ratio() < best[0]: continue
        r = sm.ratio()
        if r > best[0]: best = (r, b)
    res.append((a, best[1], best[0]))
out = os.path.join(ROOT, {'4': 're/k4', '5': 're/k5'}[ep], 'func_match.txt')
with open(out, 'w') as f:
    for a, b, r in res:
        f.write('%05x %s %.2f %s %d\n' % (a, '%05x' % b if b else '-----', r, names.get(b, '?') if b else '-', len(A[a])))
print('wrote', out)
