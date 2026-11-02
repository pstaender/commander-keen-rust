#!/usr/bin/env python3
"""Poll the original game running in DOSBox Staging (webserver_enabled=true)
and log the player's DemoMode/mapon/x/y/shapenum whenever it changes.

usage: EP=4|5|6 dosbox_trace.py OUT.txt SECONDS [DS]
DS is the data segment of the running game; default: the DGROUP paragraph
of the executable - 0xe5d (what DOSBox Staging uses with a plain `keenNx`
from C:), checked by reading Borland's copyright at DS:4.
--key: write LastScan = 1 (Esc) once after 10 s, for the "Press a key"
signon screen."""
import struct, sys, time, urllib.request, os

EP = os.environ.get('EP', '6')
# DGROUP seg (ghidra, load 0x1000), DemoMode, gamestate.mapon, player, LastScan
OFS = {'4': (0x3dbf, 0x3ac6, 0x77e7, 0xa546, 0xa103),
       '5': (0x4034, 0x332c, 0x6f68, 0x9e3e, 0x9bad),
       '6': (0x3f90, 0x35a2, 0x73f7, 0xa7b6, 0xa373)}[EP]
args = [a for a in sys.argv[1:] if not a.startswith('--')]
out, secs = args[0], float(args[1])
ds = int(args[2], 0) if len(args) > 2 else OFS[0] - 0xe5d
base = "http://127.0.0.1:8086/api/v1/memory"

def rd(off, n):
    with urllib.request.urlopen(f"{base}/{ds}/{off}/{n}") as r:
        return r.read()

def wr(off, data):
    req = urllib.request.Request(f"{base}/{ds}/{off}", data=data, method="PUT",
                                 headers={"Content-Type": "application/octet-stream"})
    urllib.request.urlopen(req).read()

last = None
start = time.time()
end = start + secs
keyed = '--key' not in sys.argv
with open(out, "w") as f:
    while time.time() < end:
        try:
            if not keyed and time.time() - start > 10:
                print('DS:4 =', rd(4, 8))
                wr(OFS[4], b'\x01\x00')
                keyed = True
            demo, = struct.unpack("<h", rd(OFS[1], 2))
            mapon, = struct.unpack("<h", rd(OFS[2], 2))
            player, = struct.unpack("<H", rd(OFS[3], 2))
            ob = rd(player, 0x4c)
        except Exception:
            time.sleep(0.2)
            continue
        x, y = struct.unpack_from("<HH", ob, 0x0a)
        shape, = struct.unpack_from("<h", ob, 0x1e)
        cur = (demo, mapon, x, y, shape)
        if cur != last:
            f.write("%d %d %d %d %d\n" % cur)
            last = cur
