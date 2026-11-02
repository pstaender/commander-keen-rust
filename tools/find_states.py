#!/usr/bin/env python3
"""Finds the statetype records (30 bytes) in DGROUP of the unpacked
executable (EP=4|5|6) and prints them like re/states.txt.
A record is accepted when its think/contact/react pointers are entries of
analysed procedures (re/.../procedures.txt) and its nextstate is null or
another accepted record."""
import struct, os
exec(open(os.path.join(os.path.dirname(os.path.abspath(__file__)), 'explore.py')).read())
procs = set()
pf = {'4': 're/k4/procedures.txt', '5': 're/k5/procedures.txt', '6': 're/procedures.txt'}[EP]
for l in open(os.path.join(_ROOT, pf)):
    procs.add(int(l.split()[0], 16))
D = lin(DGLIN, 0xffff)
def rec(i): return struct.unpack_from('<8h7H', D, i)
def fp(o, s): return None if (o, s) == (0, 0) else (s + 0x1000) * 16 + o
def ok(i):
    if i + 30 > len(D): return False
    l, r, prog, skip, push, tic, xm, ym, t0, t1, c0, c1, r0, r1, nx = rec(i)
    if not (0 <= prog <= 4 and skip in (0, 1) and push in (0, 1) and 0 <= tic <= 32767): return False
    if not (-1 <= l < 1000 and -1 <= r < 1000): return False
    n = 0
    for o, s in ((t0, t1), (c0, c1), (r0, r1)):
        a = fp(o, s)
        if a is not None:
            if a not in procs and not (0x10000 <= a < DGLIN and lin(a, 3) == b'\x55\x8b\xec'): return False
            n += 1
    return nx == 0 or nx % 2 == 0
cand = {i for i in range(0, 0xffff - 30, 2) if ok(i)}
# fixpoint: nextstate must be accepted too
while True:
    bad = {i for i in cand if rec(i)[14] and rec(i)[14] not in cand}
    if not bad: break
    cand -= bad
# keep chains that contain at least one function pointer or are referenced
# (by another state or as a constant in the decompiled code)
refs = {rec(i)[14] for i in cand}
import glob, re
lo = min(i for i in cand if i >= 0x700); hi = 0x4000
for f in glob.glob(os.path.join(DECOMP, '*.c')):
    for m in re.finditer(r'\b0x([0-9a-f]{3,4})\b', open(f).read()):
        v = int(m.group(1), 16)
        if lo <= v < hi and v in cand: refs.add(v)
final = []
for s in sorted(cand):
    l, r, prog, skip, push, tic, xm, ym, t0, t1, c0, c1, r0, r1, nx = rec(s)
    hasfn = any(fp(o, sg) for o, sg in ((t0, t1), (c0, c1), (r0, r1)))
    if not (hasfn or s in refs) or not (lo <= s < hi): continue
    if final and s - final[-1] < 30:
        if s in refs and final[-1] not in refs: final[-1] = s
        continue
    final.append(s)
for s in final:
    l, r, prog, skip, push, tic, xm, ym, t0, t1, c0, c1, r0, r1, nx = rec(s)
    f = lambda o, sg: ('%05x' % fp(o, sg)) if fp(o, sg) else '-'
    print('%04x L%3d R%3d p%d s%d f%d t%3d xm%4d ym%4d th %s co %s re %s nx %04x' % (s, l, r, prog, skip, push, tic, xm, ym, f(t0, t1), f(c0, c1), f(r0, r1), nx))
