"""Rebuilds the original AdLib sound effects in MJSFX, following them exactly.

An AdLib effect is one instrument playing one F-number per 140 Hz tick
(0 = key off). This script reads that data (sfx.json from
`keenN --export-sfx DIR`), takes the loudness per tick and the timbre from
the rendered DIR/NN-adlib.wav, and builds an MJSFX composition: the pitch
curve is cut into straight pieces (in semitones), each piece is a layer that
glides from its start to its end pitch, starting at its tick and as loud as
the original there. A light chorus and room are added on top.

Usage (from tools/mjsfx/): transcribe.py EP SFXDIR OUTDIR [numbers...]
  e.g. transcribe.py 6 ../../Keen6/sfx-original ../../audio-packs/mjsfx-adlib/sfx/k6
"""
import json, math, os, shutil, struct, sys, wave
from design import m, blank, setp, lfo, comp, fx, P, T

TICK = 1 / 140
RATE = 44100
SPT = RATE / 140  # samples per tick
CMULT = [0.5, 1, 2, 3, 4, 5, 6, 7, 8, 8, 10, 10, 12, 12, 15, 15]


def read_wav(path):
    w = wave.open(path)
    n = w.getnframes()
    a = struct.unpack(f"<{n * w.getnchannels()}h", w.readframes(n))[:: w.getnchannels()]
    return [x / 32768 for x in a]


def tick_rms(x, i):
    seg = x[int(i * SPT): int((i + 1) * SPT)]
    return math.sqrt(sum(v * v for v in seg) / len(seg)) if seg else 0.0


def timbre(x, ticks, freqs):
    """(tonal 0..1, brightness = zero crossings per period) over the voiced ticks."""
    tonal, bright, k = 0.0, 0.0, 0
    for i in ticks[: 40]:
        f = freqs[i]
        seg = x[int(i * SPT): int(i * SPT) + 1024]
        if len(seg) < 1024 or f <= 0 or max(map(abs, seg)) < 0.01:
            continue
        lag = RATE / f
        best = 0.0
        for L in (lag, lag * 2):
            li = int(L)
            if li + 300 >= len(seg):
                continue
            a = seg[: len(seg) - li]
            b = seg[li:]
            num = sum(p * q for p, q in zip(a, b))
            den = math.sqrt(sum(p * p for p in a) * sum(q * q for q in b)) or 1
            best = max(best, num / den)
        z = sum(1 for j in range(1, len(seg)) if (seg[j - 1] < 0) != (seg[j] < 0))
        tonal += best
        bright += z / 2 / (len(seg) / RATE) / f
        k += 1
    return (tonal / k, bright / k) if k else (0.0, 1.0)


def segments(st, tol):
    """Cuts a pitch curve (semitones per tick) into straight pieces."""
    segs, s = [], 0
    while s < len(st):
        e = s + 1
        while e < len(st) and e - s < 70:
            if abs(st[e] - st[e - 1]) > 1.5:  # a new note
                break
            ok = all(abs(st[s] + (st[e] - st[s]) * (j - s) / (e - s) - st[j]) <= tol for j in range(s, e + 1))
            if not ok:
                break
            e += 1
        segs.append((s, e))  # ticks s .. e-1
        s = e
    return segs


def zcr(x, i):
    seg = x[int(i * SPT): int((i + 2) * SPT)]
    if len(seg) < 50 or max(map(abs, seg)) < 0.01:
        return None
    return sum(1 for j in range(1, len(seg)) if (seg[j - 1] < 0) != (seg[j] < 0)) / 2 / (len(seg) / RATE)


def build(sound, wav):
    data, block = sound["adlib"], sound["block"]
    inst = sound["inst"]
    # the pitch you hear: the carrier's multiple, or in additive mode
    # (connection bit) the louder of both operators' multiples
    cm, mm = CMULT[inst[1] & 15], CMULT[inst[0] & 15]
    mtl, ctl = inst[2] & 63, inst[3] & 63
    mult = mm if (inst[10] & 1 and mtl <= ctl + 6 and mm > cm) else cm
    freqs = [b * 49716 / 2 ** (20 - block) * mult if b else 0 for b in data]
    x = read_wav(wav)
    voiced = [i for i, f in enumerate(freqs) if f > 0]
    if not voiced:
        return None
    # timbre from the original: zero crossings per period of that pitch
    zs = [(zcr(x, i), freqs[i]) for i in voiced]
    zs = [(z, f) for z, f in zs if z]
    zabs = sum(z for z, _ in zs) / len(zs) if zs else 0
    bright = sum(z / f for z, f in zs) / len(zs) if zs else 1
    # noise needs strong modulator feedback on the chip as well
    feedback = (inst[10] >> 1) & 7
    tonal = 0.0 if feedback >= 5 and (bright > 20 or (bright > 8 and zabs > 1500)) else 1.0
    # wave: noise, or a tone from soft (triangle) to bright (saw)
    if tonal < 0.45:
        wave_p = dict(waveType=3)
    elif bright < 1.4:
        wave_p = dict(waveType=4)
    elif bright < 2.6:
        wave_p = dict(waveType=0, duty=0.5)
    elif bright < 4.5:
        wave_p = dict(waveType=0, duty=0.25)
    else:
        wave_p = dict(waveType=1)
    peak = max(tick_rms(x, i) for i in voiced) or 1
    # runs of key-on ticks
    runs, i = [], 0
    while i < len(freqs):
        if freqs[i] > 0:
            j = i
            while j < len(freqs) and freqs[j] > 0:
                j += 1
            runs.append((i, j))
            i = j
        else:
            i += 1
    tol = 0.35
    while True:
        pieces = []
        for a, b in runs:
            st = [12 * math.log2(freqs[t] / 440) + 69 for t in range(a, b)]
            pieces += [(a + s, a + e, freqs[a + s], freqs[a + e - 1], e == b - a) for s, e in segments(st, tol)]
        if len(pieces) <= 28 or tol > 6:
            break
        tol *= 1.6
    # release after key-off, from the wav (until -30 dB)
    end = len(data)
    rel = 0
    while tick_rms(x, end + rel) > peak * 0.06 and rel < 140:
        rel += 1
    layers, offs, gains = [], [], []
    for (s, e, f0, f1, last) in pieces:
        dur = (e - s) * TICK
        g = sum(tick_rms(x, t) for t in range(s, e)) / (e - s) / peak
        if g < 0.02:
            continue
        tail = max(0.012, rel * TICK) if last else 0.012
        q = dict(baseFreq=P(max(f0, 20)), envAttack=0, envSustain=T(dur), envPunch=0, envDecay=T(tail),
                 freqRamp=0, freqDramp=0, arpMod=0, arpSpeed=0, vibStrength=0, vibSpeed=0, duty=0.5,
                 dutyRamp=0, repeatSpeed=0, phaOffset=0, phaRamp=0, lpfFreq=1, lpfRamp=0, lpfResonance=0,
                 hpfFreq=0, hpfRamp=0)
        q.update(wave_p)
        L = blank(q["waveType"], **q)
        if f1 != f0 and f0 > 0 and f1 > 0:
            # glide over the held part: the LFO spans the layer (hold + tail)
            octs = math.log2(f1 / f0) * (dur + tail) / dur
            L = lfo(L, "0", shape="sawUp", rateMode="cycles", rate=1, cycleMode="oneShot",
                    polarity="unipolar", depth=max(-2.0, min(2.0, octs)), target="source.baseFreq")
        layers.append(L)
        offs.append(round(s * TICK, 4))
        gains.append(round(g, 3))
    if not layers:
        return None
    if len(layers) <= 60:
        snd = comp(layers, offsets=offs, gains=gains)
    else:
        # compose takes at most 64 sounds per call: build groups first
        groups = [comp(layers[i:i + 50], offsets=offs[i:i + 50], gains=gains[i:i + 50]) for i in range(0, len(layers), 50)]
        snd = comp(groups)
    snd = fx(snd, "chorus", mix=0.25, depth=8, rate=0.8)
    snd = fx(snd, "reverb", mix=0.12, size=0.35, decay=0.3)
    return snd, len(layers), tonal, bright


if __name__ == "__main__":
    ep, src, out = sys.argv[1], sys.argv[2], sys.argv[3]
    only = set(int(v) for v in sys.argv[4:])
    os.makedirs(out, exist_ok=True)
    for sound in json.load(open(os.path.join(src, "sfx.json"))):
        n = sound["n"]
        if only and n not in only:
            continue
        wav = os.path.join(src, f"{n:02d}-adlib.wav")
        if not sound["adlib"] or not os.path.exists(wav):
            continue
        r = build(sound, wav)
        if not r:
            continue
        snd, nl, tonal, bright = r
        snd = m.call("normalize", sound=snd, targetPeak=0.7)
        snd["name"] = f"keen{ep}-{n:02d}-adlib"
        res = m.call("render_wav", sound=snd, name=snd["name"])
        m.call("save", sound=snd, name=f"Keen AdLib/{snd['name']}", overwrite=True)
        shutil.copy(res["path"], os.path.join(out, f"{n:02d}.wav"))
        print(f"K{ep} {n:2d}: {nl:2d} layers, tonal {tonal:.2f}, bright {bright:.1f}, {res['durationMs'] / 1000:.2f}s", flush=True)
