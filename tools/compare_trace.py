#!/usr/bin/env python3
"""Compare a DOSBox trace (dosbox_trace.py) with KEEN6_TRACE files of the
Rust port: every sampled (x, y, shapenum) of the original during a demo
must appear, in order, in the port's per-frame trace of that map.

usage: compare_trace.py ORIG.txt RUST1.txt [RUST2.txt ...]"""
import sys

orig = [tuple(map(int, l.split())) for l in open(sys.argv[1])]
rust = {}
for p in sys.argv[2:]:
    rows = [tuple(map(int, l.split())) for l in open(p)]
    rust[rows[0][0]] = [r[1:] for r in rows]

# split the original into demo segments (DemoMode != 0, contiguous map)
segs, cur = [], None
for demo, mapon, x, y, shape in orig:
    if demo == 0 or mapon == 0:
        cur = None
        continue
    if cur is None or cur[0] != mapon:
        cur = (mapon, [])
        segs.append(cur)
    if not cur[1] or cur[1][-1] != (x, y, shape):
        cur[1].append((x, y, shape))

for mapon, samples in segs:
    seq = rust.get(mapon)
    if seq is None:
        print(f"map {mapon}: no port trace")
        continue
    i = matched = 0
    first_bad = None
    for s in samples:
        j = i
        while j < len(seq) and seq[j] != s:
            j += 1
        if j < len(seq):
            i = j + 1
            matched += 1
        elif first_bad is None:
            first_bad = (s, i)
    print(f"map {mapon}: {matched}/{len(samples)} original samples found in order "
          f"(port frames {len(seq)})")
    if first_bad:
        s, i = first_bad
        print(f"  first unmatched sample {s} after port frame {i}: "
              f"port has {seq[max(0, i - 2):i + 3]}")
