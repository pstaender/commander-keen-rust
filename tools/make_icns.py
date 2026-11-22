"""Makes a macOS .icns from icon.png (8 bit RGB or RGBA): enlarged with
nearest neighbour so pixel art stays sharp, reduced by averaging:
python3 -I tools/make_icns.py icon.png out.icns"""
import struct
import sys
import zlib


def read_png(path):
    data = open(path, "rb").read()
    assert data[:8] == b"\x89PNG\r\n\x1a\n", "not a PNG"
    pos, idat, w = 8, b"", 0
    while pos < len(data):
        n, typ = struct.unpack(">I4s", data[pos:pos + 8])
        body = data[pos + 8:pos + 8 + n]
        if typ == b"IHDR":
            w, h, depth, ctype, _, _, interlace = struct.unpack(">IIBBBBB", body)
            assert depth == 8 and ctype in (2, 6) and interlace == 0, "needs 8 bit RGB or RGBA, not interlaced"
            bpp = 4 if ctype == 6 else 3
        elif typ == b"IDAT":
            idat += body
        pos += 12 + n
    raw, stride = zlib.decompress(idat), w * bpp
    rows, prev, i = [], bytearray(stride), 0
    for _ in range(h):
        f, line = raw[i], bytearray(raw[i + 1:i + 1 + stride])
        i += 1 + stride
        for x in range(stride):
            a = line[x - bpp] if x >= bpp else 0
            b = prev[x]
            c = prev[x - bpp] if x >= bpp else 0
            if f == 1:
                line[x] = (line[x] + a) & 255
            elif f == 2:
                line[x] = (line[x] + b) & 255
            elif f == 3:
                line[x] = (line[x] + (a + b) // 2) & 255
            elif f == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[x] = (line[x] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        rows.append(bytes(line) if bpp == 4 else b"".join(bytes(line[i:i + 3]) + b"\xff" for i in range(0, stride, 3)))
        prev = line
    return w, h, rows


def write_png(w, h, rows):
    def chunk(t, b):
        return struct.pack(">I", len(b)) + t + b + struct.pack(">I", zlib.crc32(t + b))
    raw = b"".join(b"\0" + r for r in rows)
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))


def scaled(w, h, rows, size):
    out = []
    if size >= w:
        for y in range(size):
            src = rows[y * h // size]
            out.append(b"".join(src[(x * w // size) * 4:(x * w // size) * 4 + 4] for x in range(size)))
        return write_png(size, size, out)
    # smaller: the average of the source pixels each one covers
    for y in range(size):
        y0, y1 = y * h // size, max((y + 1) * h // size, y * h // size + 1)
        line = bytearray()
        for x in range(size):
            x0, x1 = x * w // size, max((x + 1) * w // size, x * w // size + 1)
            n = (x1 - x0) * (y1 - y0)
            for c in range(4):
                line.append(sum(rows[sy][sx * 4 + c] for sy in range(y0, y1) for sx in range(x0, x1)) // n)
        out.append(bytes(line))
    return write_png(size, size, out)


def main():
    w, h, rows = read_png(sys.argv[1])
    # (no icp4/icp5/icp6: macOS doesn't read PNG data in them reliably;
    # the small sizes come from ic11/ic12, the @2x variants)
    types = [(b"ic07", 128), (b"ic08", 256), (b"ic09", 512), (b"ic10", 1024),
             (b"ic11", 32), (b"ic12", 64), (b"ic13", 256), (b"ic14", 512)]
    body = b""
    for t, size in types:
        png = scaled(w, h, rows, size)
        body += t + struct.pack(">I", len(png) + 8) + png
    open(sys.argv[2], "wb").write(b"icns" + struct.pack(">I", len(body) + 8) + body)


main()
