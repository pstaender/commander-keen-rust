#!/usr/bin/env python3
"""Shows how a Keen 6 function differs in Keen 4/5: unified diff of the
Keen 6 pseudo-C (re/decomp) and the translated pseudo-C of the matching
function (re/kN/xdecomp, globals/functions renamed to their Keen 6
equivalents by xlate.py).
Usage: EP=4 xdiff.py <k6 addr hex> [ep addr hex]   (or: xdiff.py -l to list
all matched shared functions that differ, with similarity)"""
import os, sys, difflib, re
ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..')
EP = os.environ.get('EP', '4')
D = os.path.join(ROOT, 're', 'k' + EP)
fm = {}
for l in open(os.path.join(D, 'funcs_map.txt')):
    a, b = l.split()[:2]
    fm.setdefault(int(b, 16), []).append(int(a, 16))
names = {}
for l in open(os.path.join(D, 'funcs_map.txt')):
    p = l.split(); names[int(p[1], 16)] = p[2]
import importlib.util
_spec = importlib.util.spec_from_file_location('st', os.path.join(ROOT, 'tools', 'states_k6.py'))
_m = importlib.util.module_from_spec(_spec); _spec.loader.exec_module(_m); K6ST = _m.STATES
def k6text(b):
    t = open(os.path.join(ROOT, 're/decomp', '%05x.c' % b)).read().split('\n')[1:]
    # K6 functions by name where known, like the translated side
    def sub(m):
        v = int(m.group(1), 16) * 16 + int(m.group(2), 16)
        return names.get(v, 'K6FUN_%05x' % v) if names.get(v, '?') != '?' else 'K6FUN_%05x' % v
    t = [re.sub(r'FUN_([0-9a-f]{4})_([0-9a-f]{4})', sub, l) for l in t]
    t = [re.sub(r'\b0x([0-9a-f]{3,4})\b', lambda m: '&' + K6ST[int(m.group(1), 16)] if int(m.group(1), 16) in K6ST else m.group(0), l) for l in t]
    return [re.sub(r'LAB_[0-9a-f]{4}_[0-9a-f]{4}', 'LAB', l) for l in t]
def eptext(a):
    return open(os.path.join(D, 'xdecomp', '%05x.c' % a)).read().split('\n')[2:]
if sys.argv[1] == '-l':
    sim = {}
    for l in open(os.path.join(D, 'func_match.txt')):
        p = l.split(); sim[int(p[0], 16)] = (p[1], float(p[2]), p[3])
    for a in sorted(sim):
        b, r, n = sim[a]
        if b != '-----' and r >= 0.6:
            x, y = k6text(int(b, 16)), eptext(a)
            if x != y:
                d = sum(1 for l in difflib.unified_diff(x, y, lineterm='', n=0) if l[:1] in '+-' and l[:3] not in ('+++', '---'))
                print('%05x k6 %s %.2f %-24s %d changed lines' % (a, b, r, n, d))
    sys.exit()
b = int(sys.argv[1], 16)
a = int(sys.argv[2], 16) if len(sys.argv) > 2 else fm[b][0]
print('K6 %05x  <->  K%s %05x' % (b, EP, a))
for l in difflib.unified_diff(k6text(b), eptext(a), 'k6', 'k' + EP, lineterm='', n=2):
    print(l)
