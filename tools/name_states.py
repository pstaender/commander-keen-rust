#!/usr/bin/env python3
"""Proposes names for the states of Keen 4/5 (EP=4|5): aligns the state
list (re/kN/states.txt) with the named Keen 6 states by their mapped
think/contact/react functions and parameters. Prints a STATES dict;
unmatched states get S_<addr> placeholders named after their functions."""
import os, sys, difflib, importlib.util, re
ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..')
EP = os.environ.get('EP', '4')
def load(ep):
    spec = importlib.util.spec_from_file_location('st', os.path.join(ROOT, 'tools', 'states_k%s.py' % ep))
    m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m); return m
K6 = load('6')
fmap = {}
for l in open(os.path.join(ROOT, 're', 'k' + EP, 'funcs_map.txt')):
    p = l.split(); fmap[int(p[0], 16)] = (int(p[1], 16), p[2])
def parse(path):
    out = []
    for l in open(path):
        m = re.match(r'(\w{4}) L\s*(-?\d+) R\s*(-?\d+) p(\d) s(\d) f(\d) t\s*(\d+) xm\s*(-?\d+) ym\s*(-?\d+) th (\S+) co (\S+) re (\S+) nx (\w{4})', l)
        if m: out.append(m.groups())
    return out
a = parse(os.path.join(ROOT, 're', 'k' + EP, 'states.txt'))
b = [s for s in parse(os.path.join(ROOT, 're', 'states.txt'))]
def fn6(x):
    return x if x == '-' else ('%05x' % fmap[int(x, 16)][0] if int(x, 16) in fmap else 'k' + x)
def sig(s, conv):
    # numeric parameters, the relative next state and which functions exist;
    # functions are compared afterwards where both sides are known
    nx = int(s[12], 16); off = int(s[0], 16)
    return (s[3], s[4], s[5], s[6], s[7], s[8], nx == off, nx == 0) + tuple(x == "-" for x in s[9:12])
def fns(s, conv):
    return tuple(conv(x) for x in s[9:12])
sa = [sig(s, fn6) for s in a]
sb = [sig(s, lambda x: x) for s in b]
fa = [fns(s, fn6) for s in a]
fb = [fns(s, lambda x: x) for s in b]
def fn_ok(i, j):
    return all(x == y or x.startswith('k') for x, y in zip(fa[i], fb[j]))
sm = difflib.SequenceMatcher(None, sa, sb, autojunk=False)
names = {}
for tag, i1, i2, j1, j2 in sm.get_opcodes():
    if tag == 'equal' or (tag == 'replace' and i2 - i1 == j2 - j1):
        for i, j in zip(range(i1, i2), range(j1, j2)):
            k6off = int(b[j][0], 16)
            if k6off in K6.STATES and fn_ok(i, j):
                names[int(a[i][0], 16)] = K6.STATES[k6off]
used = set()
print('STATES = {')
for s in a:
    off = int(s[0], 16)
    n = names.get(off)
    if n is None or n in used:
        fns = [fmap[int(x, 16)][1] if x != '-' and int(x, 16) in fmap else x for x in s[9:12]]
        n = 'S_%04X' % off
        print('    0x%04x: %r,  # L%s %s' % (off, n, s[1], ' '.join(fns)))
    else:
        print('    0x%04x: %r,' % (off, n))
    used.add(n)
print('}')
