#!/usr/bin/env python3
"""Dumps a near jump table: EP=N jt.py <code seg (ghidra, e.g. 196c)> <table offset> <count>
prints index -> segment offset and the linear address of each target."""
import os, sys, struct
exec(open(os.path.join(os.path.dirname(os.path.abspath(__file__)), 'explore.py')).read())
seg = int(sys.argv[1], 16); off = int(sys.argv[2], 16); n = int(sys.argv[3])
base = seg * 16
for i in range(n):
    t = struct.unpack_from('<H', IMG, base + off + 2 * i - 0x10000)[0]
    print('%2d -> %04x (linear %05x)' % (i, t, base + t))
