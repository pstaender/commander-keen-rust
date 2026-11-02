#!/usr/bin/env python3
"""Minimal UNLZEXE (LZEXE 0.91) unpacker. Usage: unlzexe.py in.exe out.exe"""
import struct, sys

def main(src, dst):
    d = open(src, 'rb').read()
    assert d[:2] == b'MZ' and d[0x1c:0x20] == b'LZ91', 'not LZ91'
    hdr_par = struct.unpack_from('<H', d, 8)[0]
    ip, cs = struct.unpack_from('<HH', d, 0x14)
    stub = hdr_par * 16 + cs * 16
    rip, rcs, rsp, rss, csize, inc, usize, chk = struct.unpack_from('<8H', d, stub)
    # relocations
    p = stub + 0x158; seg = 0; off = 0; relocs = []
    while True:
        span = d[p]; p += 1
        if span == 0:
            span = struct.unpack_from('<H', d, p)[0]; p += 2
            if span == 0:
                seg += 0x0fff; continue
            elif span == 1:
                break
        off += span
        seg += (off & ~0x0f) >> 4
        off &= 0x0f
        relocs.append((off, seg))
    # decompress
    p = hdr_par * 16
    out = bytearray()
    st = {'buf': 0, 'cnt': 0, 'p': p}
    def word():
        w = struct.unpack_from('<H', d, st['p'])[0]; st['p'] += 2; return w
    st['buf'] = word(); st['cnt'] = 16
    def bit():
        b = st['buf'] & 1
        st['cnt'] -= 1
        if st['cnt'] == 0:
            st['buf'] = word(); st['cnt'] = 16
        else:
            st['buf'] >>= 1
        return b
    def byte():
        b = d[st['p']]; st['p'] += 1; return b
    while True:
        if bit():
            out.append(byte()); continue
        if not bit():
            ln = (bit() << 1) | bit(); ln += 2
            span = byte() | 0xff00
        else:
            lo = byte(); hi = byte()
            span = lo | ((hi & 0xf8) << 5) | 0xe000
            ln = hi & 7
            if ln == 0:
                ln = byte()
                if ln == 0: break
                if ln == 1: continue
                ln += 1
            else:
                ln += 2
        span -= 0x10000
        for _ in range(ln):
            out.append(out[span])
    # build new exe
    nrel = len(relocs)
    hsize = 0x1c + nrel * 4
    hsize = (hsize + 0x1ff) & ~0x1ff
    total = hsize + len(out)
    minalloc_orig, maxalloc = struct.unpack_from('<HH', d, 0x0a)
    # memory needed by stub vs. real program: keep generous minalloc
    hdr = bytearray(hsize)
    struct.pack_into('<14H', hdr, 0, 0x5a4d, total % 512, (total + 511) // 512, nrel,
                     hsize // 16, minalloc_orig + (usize if False else 0), maxalloc, rss, rsp, 0, rip, rcs, 0x1c, 0)
    for i, (o, s) in enumerate(relocs):
        struct.pack_into('<HH', hdr, 0x1c + i * 4, o, s)
    open(dst, 'wb').write(bytes(hdr) + bytes(out))
    print(f'unpacked {len(out)} bytes, {nrel} relocs, entry {rcs:04x}:{rip:04x} ss:sp {rss:04x}:{rsp:04x}')

if __name__ == '__main__':
    main(sys.argv[1], sys.argv[2])
