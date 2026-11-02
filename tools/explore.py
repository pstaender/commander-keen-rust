# Shared helpers for the analysis tools. The episode is selected with the
# environment variable EP=4|5|6 (default 6).
import struct, sys, os
def _root():
    # exec()'d by other tools, so __file__ may be the caller: search upwards
    for d in ([os.path.dirname(os.path.abspath(globals()['__file__']))] if '__file__' in globals() else []) + [os.getcwd()]:
        while d != '/':
            if os.path.exists(os.path.join(d, 'tools', 'explore.py')):
                return d
            d = os.path.dirname(d)
    return '.'
_ROOT = _root()
_EPS = {
    # unpacked exe, header size, DGROUP linear address, decomp dir
    '4': ('re/k4/KEEN4E_unpacked.exe', 0x2e00, 0x3dbf0, 're/k4/decomp'),
    '5': ('re/k5/KEEN5E_unpacked.exe', 0x2c00, 0x40340, 're/k5/decomp'),
    '6': ('re/KEEN6C_unpacked.exe', 0x2c00, 0x3f900, 're/decomp'),
}
EP = os.environ.get('EP', '6')
EXE, HDR, DGLIN, DECOMP = _EPS[EP]
EXE = os.path.join(_ROOT, EXE); DECOMP = os.path.join(_ROOT, DECOMP)
DGSEG = DGLIN // 16 - 0x1000
IMG = open(EXE, 'rb').read()[HDR:]
def lin(a,n): return IMG[a-0x10000:a-0x10000+n]
def huff(dct, src, outlen):
    nodes=[struct.unpack_from('<HH',dct,i*4) for i in range(255)]
    out=bytearray(); node=254; i=0
    while len(out)<outlen and i<len(src):
        b=src[i]; i+=1
        for bit in range(8):
            v=nodes[node][(b>>bit)&1]
            if v<256:
                out.append(v); node=254
                if len(out)>=outlen: break
            else: node=v-256
    return bytes(out)
