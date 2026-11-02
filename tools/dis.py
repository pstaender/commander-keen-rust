#!/usr/bin/env python3
"""Disassemble a function of the unpacked executable (EP=4|5|6, default 6) with ndisasm.
Usage: dis.py <linear-addr hex> [maxbytes]  -- stops after the first RETF
that is followed by a PUSH BP (next function) or at maxbytes."""
import sys, subprocess, tempfile, os
exec(open(os.path.join(os.path.dirname(os.path.abspath(__file__)), 'explore.py')).read())
img = IMG
a = int(sys.argv[1], 16); n = int(sys.argv[2]) if len(sys.argv) > 2 else 2000
seg = (a - 0x10000) & ~0xf
off = a - 0x10000 - seg
data = img[a - 0x10000:a - 0x10000 + n]
with tempfile.NamedTemporaryFile(delete=False) as f:
    f.write(data); name = f.name
out = subprocess.run(['ndisasm', '-b16', '-o', hex(off), name], capture_output=True, text=True).stdout
os.unlink(name)
lines = out.splitlines()
res = []
for i, l in enumerate(lines):
    res.append(l)
    if l.split()[2:3] == ['retf'] or (len(l.split()) > 2 and l.split()[2] == 'retf'):
        nxt = lines[i + 1] if i + 1 < len(lines) else ''
        if '55' == nxt.split()[1:2][0][:2] if nxt.split()[1:2] else False:
            break
print('\n'.join(res))
