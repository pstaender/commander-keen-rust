#!/usr/bin/env python3
"""Cross-episode symbol mapping (EP=4|5).

Aligns the decompiled C of every function pair matched by match_funcs.py
(re/kN/func_match.txt, similarity >= 0.9) and votes which Keen 6 global
(DAT_3f90_xxxx) and function each DAT_/FUN_ of this episode corresponds to.
Writes re/kN/globals_map.txt, re/kN/funcs_map.txt and re/kN/xdecomp/<addr>.c:
the decompiled code with Keen 6 names (K6 globals as DAT_3f90_xxxx,
functions as their K6 name/address, unmapped ones kept)."""
import os, re, glob, difflib, collections
ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..')
EP = os.environ.get('EP', '4')
D = os.path.join(ROOT, 're', 'k' + EP)
DS = {'4': '3dbf', '5': '4034'}[EP]
TOK = re.compile(r'DAT_[0-9a-f]{4}_[0-9a-f]{4}|FUN_[0-9a-f]{4}_[0-9a-f]{4}|0x[0-9a-f]+|\w+|[^\s\w]')
def fun_lin(t):
    s, o = t[4:8], t[9:13]
    return int(s, 16) * 16 + int(o, 16)
def norm(t):
    if t.startswith('DAT_'): return 'DAT'
    if t.startswith('FUN_'): return 'FUN'
    if t.startswith('0x'): return 'HEX'
    return t
names = {}
for f in glob.glob(os.path.join(ROOT, 'src/**/*.rs'), recursive=True):
    for m in re.finditer(r'/// ([A-Za-z_0-9]+) \((0x[0-9a-f]+)', open(f).read()):
        names.setdefault(int(m.group(2), 16), m.group(1))
pairs = []
weak = []
for l in open(os.path.join(D, 'func_match.txt')):
    a, b, r = l.split()[:3]
    if b != '-----' and float(r) >= 0.9:
        pairs.append((int(a, 16), int(b, 16)))
    elif b != '-----' and float(r) >= 0.75:
        weak.append((int(a, 16), int(b, 16)))
def code(path):
    try:
        t = open(path).read()
    except OSError:
        return []
    return TOK.findall(t.split('\n', 1)[1] if '\n' in t else '')
gv = collections.defaultdict(collections.Counter)
fv = collections.defaultdict(collections.Counter)
for a, b in pairs:
    ta = code(os.path.join(D, 'decomp', '%05x.c' % a))
    tb = code(os.path.join(ROOT, 're/decomp', '%05x.c' % b))
    sm = difflib.SequenceMatcher(None, [norm(t) for t in ta], [norm(t) for t in tb], autojunk=False)
    for tag, i1, i2, j1, j2 in sm.get_opcodes():
        if tag != 'equal': continue
        for x, y in zip(ta[i1:i2], tb[j1:j2]):
            if x.startswith('DAT_') and y.startswith('DAT_'):
                gv[int(x[9:], 16)][int(y[9:], 16)] += 1
            elif x.startswith('FUN_') and y.startswith('FUN_'):
                fv[fun_lin(x)][fun_lin(y)] += 1
gmap = {k: c.most_common(1)[0][0] for k, c in gv.items()}
fmap = {k: c.most_common(1)[0][0] for k, c in fv.items()}
for a, b in pairs + weak:
    fmap.setdefault(a, b)
with open(os.path.join(D, 'globals_map.txt'), 'w') as f:
    for k in sorted(gmap):
        c = gv[k]
        f.write('%04x %04x %d/%d\n' % (k, gmap[k], c[gmap[k]], sum(c.values())))
with open(os.path.join(D, 'funcs_map.txt'), 'w') as f:
    for k in sorted(fmap):
        f.write('%05x %05x %s\n' % (k, fmap[k], names.get(fmap[k], '?')))
os.makedirs(os.path.join(D, 'xdecomp'), exist_ok=True)
def sub(m):
    t = m.group(0)
    if t.startswith('DAT_' + DS + '_'):
        v = int(t[9:], 16)
        return 'DAT_3f90_%04x' % gmap[v] if v in gmap else 'K%sDAT_%04x' % (EP, v)
    if t.startswith('FUN_'):
        v = fun_lin(t)
        if v in fmap:
            b = fmap[v]
            return names.get(b, 'K6FUN_%05x' % b)
        return 'K%sFUN_%05x' % (EP, v)
    return t
import importlib.util
def states_of(ep):
    spec = importlib.util.spec_from_file_location('st', os.path.join(ROOT, 'tools', 'states_k%s.py' % ep))
    m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m); return m.STATES
STATES = states_of(EP)
def stsub(m):
    v = int(m.group(1), 16)
    return '&' + STATES[v] if v in STATES else m.group(0)
for p in glob.glob(os.path.join(D, 'decomp', '*.c')):
    t = open(p).read()
    a = int(os.path.basename(p)[:-2], 16)
    head = '// K%s %05x' % (EP, a)
    if a in fmap:
        head += ' ~ K6 %05x %s' % (fmap[a], names.get(fmap[a], ''))
    body = re.sub(r'DAT_[0-9a-f]{4}_[0-9a-f]{4}|FUN_[0-9a-f]{4}_[0-9a-f]{4}', sub, t)
    body = re.sub(r'\b0x([0-9a-f]{3,4})\b', stsub, body)
    body = re.sub(r'LAB_[0-9a-f]{4}_[0-9a-f]{4}', 'LAB', body)
    open(os.path.join(D, 'xdecomp', os.path.basename(p)), 'w').write(head + '\n' + body)
print(len(gmap), 'globals,', len(fmap), 'functions mapped')
