"""Keen sound family designed with MJSFX (sfxr engine). Usage: design.py [names...]"""
import sys, json, wave, struct
from mj import MJ
m = MJ()

def gen(g, seed=1, **p):
    s = m.call("generate_from_generator", name=g, seed=str(seed))
    return setp(s, **p) if p else s

def blank(wave=0, **p):
    s = m.call("blank_sound", waveType=str(wave))
    return setp(s, **p) if p else s

def setp(s, layer=0, **p):
    return m.call("set_params", sound=s, params=p, layer=str(layer))

def fx(s, kind, target="main", **p):
    p.setdefault("enabled", 1)
    return m.call("add_effect", sound=s, effect=kind, target=str(target), params=p)

def lfo(s, layer="0", **p):
    p.setdefault("enabled", True)
    return m.call("add_lfo", sound=s, target=str(layer), params=p)

def comp(sounds, offsets=None, gains=None, name="c"):
    a = dict(sounds=sounds, name=name)
    if offsets: a["offsets"] = offsets
    if gains: a["gains"] = gains
    return m.call("compose", **a)

# a plain tone layer: square by default, no slide
def tone(freq, decay, wave=0, sustain=0.0, punch=0.0, **p):
    q = dict(baseFreq=freq, envAttack=0, envSustain=sustain, envPunch=punch, envDecay=decay,
             freqRamp=0, freqDramp=0, arpMod=0, arpSpeed=0, vibStrength=0, vibSpeed=0,
             duty=0.5, dutyRamp=0, repeatSpeed=0, phaOffset=0, phaRamp=0, lpfFreq=1, lpfRamp=0,
             lpfResonance=0, hpfFreq=0, hpfRamp=0, waveType=wave)
    q.update(p)
    # MJSFX mcp 0.3.0 renders no downward freqRamp: drop the pitch with a
    # one-shot LFO over the layer instead (about an octave per -1 depth)
    fall = 0
    if q["freqRamp"] < 0:
        fall = max(-2.0, q["freqRamp"] * 5)
        q["freqRamp"] = 0
    s = blank(wave, **q)
    if fall:
        s = lfo(s, "0", shape="sawUp", rateMode="cycles", rate=1, cycleMode="oneShot",
                polarity="unipolar", depth=fall, target="source.baseFreq")
    return s

D = {}
def sfx(n, name, ep=None):
    """Registers sound n (in episode ep = 4/5/6 only, or shared)."""
    def reg(f):
        D[f"{name}-k{ep}" if ep and not name.endswith(f"-k{ep}") else name] = (n, f, ep); return f
    return reg

def P(hz):
    """sfxr baseFreq for a frequency in Hz."""
    return min(1.9, (hz / 3528.0) ** 0.5)

def T(sec):
    """sfxr envelope stage value for a length in seconds."""
    return min(1.0, (sec * 0.441) ** 0.5)

@sfx(2, "jump")
def jump():
    a = tone(0.36, 0.30, sustain=0.10, punch=0.2, freqRamp=0.26, duty=0.35, dutyRamp=-0.2)
    b = tone(0.18, 0.28, wave=4, sustain=0.10, freqRamp=0.26)
    return comp([a, b], gains=[0.8, 0.6])

@sfx(3, "land")
def land():
    a = tone(0.20, 0.22, wave=3, punch=0.4, freqRamp=-0.3, lpfFreq=0.35, lpfRamp=-0.2)
    b = tone(0.16, 0.24, wave=4, punch=0.3, freqRamp=-0.35)
    return comp([a, b], gains=[0.6, 0.9])

@sfx(4, "fire")
def fire():
    a = tone(0.78, 0.30, sustain=0.05, punch=0.4, freqRamp=-0.42, duty=0.25, dutyRamp=0.3)
    b = tone(0.50, 0.08, wave=3, punch=0.5, freqRamp=-0.3)
    s = comp([a, b], gains=[0.8, 0.35])
    return fx(s, "delay", timeMs=70, feedback=0.25, mix=0.2)

@sfx(7, "pogo")
def pogo():
    a = tone(0.22, 0.25, wave=2, sustain=0.06, punch=0.3, freqRamp=0.35, vibStrength=0.35, vibSpeed=0.55)
    b = tone(0.44, 0.15, sustain=0.03, freqRamp=0.3, duty=0.2)
    return comp([a, b], gains=[1.0, 0.25])

@sfx(8, "points")
def points():
    return tone(0.45, 0.30, sustain=0.06, punch=0.45, arpMod=0.45, arpSpeed=0.55)

@sfx(9, "ammo")
def ammo():
    ns = [tone(f, 0.16, sustain=0.03, punch=0.3, duty=0.3) for f in (0.38, 0.45, 0.54)]
    s = comp(ns, offsets=[0, 0.07, 0.14])
    return fx(s, "reverb", mix=0.15, size=0.3, decay=0.3)

@sfx(10, "water")
def water():
    a = tone(0.30, 0.18, wave=2, sustain=0.02, freqRamp=0.45, freqDramp=0.2)
    s = comp([a, tone(0.40, 0.14, wave=2, sustain=0.02, freqRamp=0.5)], offsets=[0, 0.09], gains=[1, 0.6])
    return fx(s, "reverb", mix=0.3, size=0.5, decay=0.5)

@sfx(12, "enterlevel")
def enterlevel():
    a = tone(0.20, 0.55, sustain=0.35, freqRamp=0.12, duty=0.3, dutyRamp=0.25, phaOffset=0.2, phaRamp=-0.1)
    b = tone(0.30, 0.5, wave=4, sustain=0.3, freqRamp=0.12)
    s = comp([a, b], gains=[0.6, 0.6])
    return fx(s, "phaser", rate=2.5, depth=0.8, mix=0.5)

def notes(freqs, step, decay=0.22, wave=0, sustain=0.06, **p):
    return [tone(f, decay, wave=wave, sustain=sustain, punch=0.25, **p) for f in freqs], [i * step for i in range(len(freqs))]

@sfx(13, "leveldone")
def leveldone():
    ns, offs = notes([0.36, 0.40, 0.45, 0.54, 0.45, 0.54, 0.64], 0.13, decay=0.24, duty=0.3)
    ns[-1] = tone(0.64, 0.55, sustain=0.25, punch=0.25, duty=0.3, vibStrength=0.12, vibSpeed=0.5)
    bass = [tone(f, 0.3, wave=4, sustain=0.12) for f in (0.18, 0.20, 0.27)]
    s = comp(ns + bass, offsets=offs + [0, 0.39, 0.78], gains=[0.7] * 7 + [0.6] * 3)
    return fx(s, "reverb", mix=0.2, size=0.5, decay=0.4)

@sfx(14, "noway")
def noway():
    a = tone(0.20, 0.12, sustain=0.08, duty=0.15)
    b = tone(0.16, 0.16, sustain=0.08, duty=0.15)
    return comp([a, b], offsets=[0, 0.13])

@sfx(15, "helmethit")
def helmethit():
    # Keen's head hits the ceiling: a dull bonk
    a = tone(0.42, 0.20, wave=0, punch=0.6, duty=0.4, freqRamp=-0.25)
    b = tone(0.21, 0.18, wave=4, punch=0.5, freqRamp=-0.25)
    c = tone(0.45, 0.06, wave=3, punch=0.6)
    return comp([a, b, c], gains=[0.5, 0.9, 0.35])

@sfx(17, "extrakeen")
def extrakeen():
    ns, offs = notes([0.40, 0.48, 0.54, 0.64, 0.54, 0.72], 0.1, decay=0.2, duty=0.25)
    ns[-1] = tone(0.72, 0.45, sustain=0.2, punch=0.3, duty=0.25, vibStrength=0.15, vibSpeed=0.55)
    s = comp(ns, offsets=offs, gains=[0.7] * 6)
    return fx(s, "reverb", mix=0.2, size=0.4, decay=0.4)

@sfx(19, "gem")
def gem():
    a = tone(0.62, 0.40, wave=2, punch=0.5, arpMod=0.5, arpSpeed=0.45)
    b = tone(0.88, 0.30, wave=0, duty=0.15, punch=0.3, arpMod=0.5, arpSpeed=0.45)
    s = comp([a, b], offsets=[0, 0.03], gains=[0.8, 0.25])
    return fx(s, "reverb", mix=0.35, size=0.6, decay=0.55)

@sfx(20, "plummet")
def plummet():
    a = tone(0.62, 0.45, wave=2, sustain=0.25, freqRamp=-0.18, vibStrength=0.1, vibSpeed=0.6)
    return comp([a, tone(0.31, 0.45, wave=0, duty=0.2, sustain=0.25, freqRamp=-0.18)], gains=[0.9, 0.2])

@sfx(21, "switch")
def switch():
    a = tone(0.55, 0.13, punch=0.6, duty=0.5)
    b = tone(0.4, 0.10, wave=3, punch=0.6)
    return comp([a, b], gains=[0.6, 0.4])

@sfx(23, "dead")
def dead():
    a = tone(0.50, 0.55, sustain=0.45, punch=0.1, freqRamp=-0.08, vibStrength=0.35, vibSpeed=0.55, duty=0.3)
    b = tone(0.25, 0.55, wave=4, sustain=0.45, freqRamp=-0.08, vibStrength=0.35, vibSpeed=0.55)
    s = comp([a, b], gains=[0.6, 0.6])
    return fx(s, "reverb", mix=0.25, size=0.5, decay=0.5)

@sfx(25, "shotexplode")
def shotexplode():
    a = tone(0.35, 0.30, wave=3, punch=0.5, freqRamp=-0.2, lpfFreq=0.7, lpfRamp=-0.3)
    b = tone(0.2, 0.15, wave=4, punch=0.4, freqRamp=-0.4)
    return comp([a, b], gains=[0.8, 0.5])

@sfx(34, "showstatus")
def showstatus():
    a = tone(0.25, 0.35, sustain=0.2, freqRamp=0.22, duty=0.3, dutyRamp=0.3)
    s = comp([a, tone(0.125, 0.35, wave=4, sustain=0.2, freqRamp=0.22)], gains=[0.5, 0.6])
    return fx(s, "phaser", rate=4, depth=0.7, mix=0.4)

@sfx(35, "hidestatus")
def hidestatus():
    a = tone(0.50, 0.35, sustain=0.2, freqRamp=-0.22, duty=0.3, dutyRamp=0.3)
    s = comp([a, tone(0.25, 0.35, wave=4, sustain=0.2, freqRamp=-0.22)], gains=[0.5, 0.6])
    return fx(s, "phaser", rate=4, depth=0.7, mix=0.4)

@sfx(41, "teleport", ep=56)
def teleport():
    a = tone(0.30, 0.6, wave=2, sustain=0.55, freqRamp=0.1, vibStrength=0.5, vibSpeed=0.75)
    b = tone(0.45, 0.6, wave=0, duty=0.2, sustain=0.55, freqRamp=0.1, vibStrength=0.5, vibSpeed=0.7, repeatSpeed=0.0)
    c = tone(0.6, 0.5, wave=5, sustain=0.4, lpfFreq=0.3, hpfFreq=0.2)
    s = comp([a, b, c], gains=[0.6, 0.2, 0.15])
    s = fx(s, "phaser", rate=3, depth=0.9, mix=0.5, stages=6)
    return fx(s, "reverb", mix=0.3, size=0.6, decay=0.5)

@sfx(43, "flagspin")
def flagspin():
    return tone(0.40, 0.25, sustain=0.1, repeatSpeed=0.6, freqRamp=0.15, duty=0.3)

@sfx(44, "flagland")
def flagland():
    a = tone(0.28, 0.24, wave=4, punch=0.5, freqRamp=-0.35)
    b = tone(0.4, 0.12, wave=3, punch=0.5)
    return comp([a, b], gains=[0.9, 0.4])


# ---- building blocks (Hz / seconds) ----------------------------------------

def tn(hz, sec, wave=0, hold=0.0, punch=0.2, **p):
    """A tone at hz lasting about hold + sec seconds."""
    return tone(P(hz), T(sec), wave=wave, sustain=T(hold) if hold else 0, punch=punch, **p)

def glide(hz, sec, octaves, wave=0, hold=0.0, **p):
    """A tone gliding by octaves (negative = down) over its length."""
    if octaves < 0:
        q = dict(p)
        s = tn(hz, sec, wave, hold, **q)
        return lfo(s, "0", shape="sawUp", rateMode="cycles", rate=1, cycleMode="oneShot",
                   polarity="unipolar", depth=max(-2.0, octaves), target="source.baseFreq")
    s = tn(hz, sec, wave, hold, **p)
    return lfo(s, "0", shape="sawUp", rateMode="cycles", rate=1, cycleMode="oneShot",
               polarity="unipolar", depth=min(2.0, octaves), target="source.baseFreq")

def noise(sec, hz=800, lp=0.6, lpramp=0.0, hold=0.0, punch=0.4, **p):
    return tn(hz, sec, wave=3, hold=hold, punch=punch, lpfFreq=lp, lpfRamp=lpramp, **p)

def boom(sec, hz=300, gain=1.0):
    """Explosion: noise with a closing filter plus a falling sub thump."""
    a = noise(sec, hz, lp=0.8, lpramp=-0.25, hold=sec * 0.15, punch=0.6)
    a = lfo(a, "0", shape="sawUp", rateMode="cycles", rate=1, cycleMode="oneShot", polarity="unipolar", depth=-1.0, target="source.baseFreq")
    b = glide(90, sec * 0.7, -1.2, wave=4, punch=0.6)
    s = comp([a, b], gains=[0.9 * gain, 0.8 * gain])
    return fx(s, "reverb", mix=0.18, size=0.5, decay=0.4)

def thud(hz=110, sec=0.15, click=0.4):
    a = glide(hz, sec, -1.2, wave=4, punch=0.6)
    b = noise(sec * 0.35, 600, lp=0.4, punch=0.6)
    return comp([a, b], gains=[1.0, click])

def zap(hz, sec, octaves, wave=0, duty=0.25, verb=0.0):
    a = glide(hz, sec, octaves, wave=wave, duty=duty, dutyRamp=0.25)
    b = noise(0.04, 2000, lp=0.9, punch=0.6)
    s = comp([a, b], gains=[0.85, 0.3])
    return fx(s, "reverb", mix=verb, size=0.3, decay=0.3) if verb else s

def blip(hz, sec=0.06, wave=0, duty=0.4):
    return tn(hz, sec, wave=wave, punch=0.5, duty=duty)

def jingle(freqs, step, last=0.4, sec=0.13, wave=0, verb=0.2, duty=0.3):
    ns = [tn(f, sec, wave=wave, hold=0.02, duty=duty) for f in freqs[:-1]]
    ns.append(tn(freqs[-1], last, wave=wave, hold=0.08, duty=duty, vibStrength=0.12, vibSpeed=0.5))
    s = comp(ns, offsets=[i * step for i in range(len(freqs))])
    return fx(s, "reverb", mix=verb, size=0.5, decay=0.45) if verb else s

def boing(hz, sec, octaves=1.0, wave=2):
    a = glide(hz, sec, octaves, wave=wave, vibStrength=0.3, vibSpeed=0.6, punch=0.4)
    return comp([a, tn(hz * 2, sec * 0.5, duty=0.2, punch=0.3)], gains=[1.0, 0.2])

def squelch(sec, up=True):
    a = glide(160, sec, 1.6 if up else -1.4, wave=1, vibStrength=0.5, vibSpeed=0.85, lpfFreq=0.45)
    b = noise(sec * 0.5, 500, lp=0.35)
    return comp([a, b], gains=[0.8, 0.4])

def hum(hz, sec, wobble=0.3, wave=0):
    a = tn(hz, sec * 0.4, wave=wave, hold=sec * 0.6, punch=0.0, duty=0.3, vibStrength=wobble, vibSpeed=0.4)
    b = tn(hz / 2, sec * 0.4, wave=4, hold=sec * 0.6, punch=0.0)
    s = comp([a, b], gains=[0.5, 0.7])
    return fx(s, "phaser", rate=1.5, depth=0.7, mix=0.4)

def sparkle(sec, base=1500, up=True):
    a = glide(base, sec, 1.0 if up else -1.0, wave=2, vibStrength=0.4, vibSpeed=0.9, punch=0.3)
    b = tn(base * 1.5, sec * 0.8, wave=0, duty=0.12, repeatSpeed=0.55, punch=0.2)
    s = comp([a, b], gains=[0.7, 0.25])
    s = fx(s, "phaser", rate=5, depth=0.8, mix=0.4)
    return fx(s, "reverb", mix=0.35, size=0.6, decay=0.5)

# ---- shared: footsteps --------------------------------------------------------

@sfx(0, "walk0")
def walk0():
    return comp([noise(0.03, 900, lp=0.3, punch=0.6), tn(140, 0.03, wave=4, punch=0.6)], gains=[0.5, 0.6])

@sfx(1, "walk1")
def walk1():
    return comp([noise(0.03, 700, lp=0.25, punch=0.6), tn(120, 0.03, wave=4, punch=0.6)], gains=[0.5, 0.6])

# ---- Paddle War (K4 47-51, K5/K6 46-50) --------------------------------------

def paddle(): return blip(520, 0.05, duty=0.5)
def ballbounce(): return blip(780, 0.04, wave=4)
def comppaddle(): return blip(390, 0.05, duty=0.5)
def compscored(): return comp([tn(260, 0.12, hold=0.05), tn(195, 0.25, hold=0.05)], offsets=[0, 0.16])
def keenscored(): return jingle([523, 659, 784, 1047], 0.08, last=0.25, sec=0.07, verb=0.1)

for ep, base in ((4, 47), (56, 46)):
    for i, f in enumerate((paddle, ballbounce, comppaddle, compscored, keenscored)):
        sfx(base + i, f"{f.__name__}{'' if ep == 4 else ''}-k{ep}", ep=ep)(f)

# ---- Keen 4 ---------------------------------------------------------------------

@sfx(5, "wormmouth", ep=4)
def k4_wormmouth():
    return comp([glide(1100, 0.12, -0.6, duty=0.3), noise(0.08, 1500, lp=0.7)], offsets=[0, 0.05], gains=[0.6, 0.5])

@sfx(16, "bounce2", ep=4)
def k4_bounce2(): return boing(150, 0.22, 0.6)

@sfx(18, "gemdoor", ep=4)
def k4_gemdoor():
    a = tn(140, 0.15, wave=0, hold=0.18, duty=0.2, repeatSpeed=0.7, punch=0.0)
    s = comp([a, noise(0.2, 400, lp=0.25, hold=0.12)], gains=[0.5, 0.6])
    return comp([s, sparkle(0.3, 1100)], offsets=[0, 0.05], gains=[1.0, 0.35])

@sfx(22, "squish", ep=46)
def squish(): return squelch(0.25, up=True)

@sfx(26, "swim1", ep=4)
def k4_swim1():
    return comp([noise(0.15, 1200, lp=0.5, lpramp=-0.2), glide(500, 0.1, -0.5, wave=2)], gains=[0.6, 0.4])

@sfx(27, "swim2", ep=4)
def k4_swim2():
    return comp([noise(0.2, 1000, lp=0.45, lpramp=-0.2), glide(420, 0.12, -0.5, wave=2)], gains=[0.6, 0.4])

@sfx(28, "bounce1", ep=4)
def k4_bounce1(): return boing(140, 0.28, 0.5)

@sfx(29, "eaterchomp", ep=4)
def k4_eaterchomp():
    c = lambda: comp([noise(0.05, 2500, lp=0.8, punch=0.7), tn(300, 0.04, wave=0, duty=0.5)], gains=[0.7, 0.4])
    return comp([c(), c()], offsets=[0, 0.12])

@sfx(30, "eaterteleport", ep=4)
def k4_eaterteleport(): return sparkle(1.0, 1400, up=False)

@sfx(31, "rescued", ep=4)
def k4_rescued(): return jingle([784, 988, 1175, 1568, 1319, 1568], 0.12, last=0.45, sec=0.1, verb=0.3, duty=0.2)

@sfx(32, "lick", ep=4)
def k4_lick():
    a = noise(0.35, 2500, lp=0.6, hold=0.08, vibStrength=0.4, vibSpeed=0.9)
    b = glide(300, 0.3, -0.7, wave=1, lpfFreq=0.5)
    s = comp([a, b], gains=[0.6, 0.5])
    return fx(s, "distortion", driveDb=12, mix=0.3)

@sfx(33, "berkeloidthrow", ep=4)
def k4_berkeloidthrow():
    return comp([glide(1600, 0.5, -2.0, wave=1, lpfFreq=0.7), noise(0.4, 900, lp=0.4, lpramp=-0.2)], gains=[0.6, 0.5])

@sfx(36, "blub", ep=4)
def k4_blub():
    s = comp([glide(220, 0.08, 1.3, wave=2), glide(330, 0.07, 1.3, wave=2)], offsets=[0, 0.1], gains=[1.0, 0.7])
    return fx(s, "reverb", mix=0.25, size=0.4, decay=0.4)

@sfx(37, "mineexplode", ep=4)
def k4_mineexplode(): return boom(1.0, 400)

@sfx(38, "spriteshoot", ep=4)
def k4_spriteshoot(): return zap(2200, 0.3, -1.5, wave=2, verb=0.2)

@sfx(39, "thunder", ep=4)
def k4_thunder():
    crack = noise(0.15, 3000, lp=0.95, punch=0.8)
    roll = noise(1.2, 250, lp=0.35, lpramp=-0.1, hold=0.2, punch=0.2, vibStrength=0.5, vibSpeed=0.3)
    s = comp([crack, roll, glide(70, 1.0, -0.5, wave=4, hold=0.1)], offsets=[0, 0.05, 0.05], gains=[0.6, 0.9, 0.7])
    return fx(s, "reverb", mix=0.3, size=0.8, decay=0.6)

@sfx(40, "fireballland", ep=4)
def k4_fireballland():
    return comp([thud(160, 0.12), noise(0.5, 1800, lp=0.7, lpramp=-0.3, punch=0.3)], gains=[0.7, 0.6])

@sfx(41, "dartshoot", ep=4)
def k4_dartshoot():
    return comp([noise(0.1, 2500, lp=0.6, punch=0.7), glide(900, 0.12, -0.6, wave=2)], gains=[0.6, 0.5])

@sfx(42, "dopefishburp", ep=4)
def k4_dopefishburp():
    a = tn(90, 0.4, wave=1, hold=0.35, punch=0.3, vibStrength=0.6, vibSpeed=0.75, lpfFreq=0.4)
    b = noise(0.6, 200, lp=0.3, hold=0.3, vibStrength=0.5, vibSpeed=0.7)
    s = comp([a, b], gains=[0.8, 0.5])
    return fx(s, "distortion", driveDb=10, mix=0.35)

@sfx(45, "wetsuit", ep=4)
def k4_wetsuit():
    return comp([sparkle(0.9, 1200, up=True), jingle([523, 659, 784, 1047], 0.1, last=0.4, sec=0.1, wave=2, verb=0.3)], gains=[0.6, 0.7])

@sfx(46, "slugpoo", ep=4)
def k4_slugpoo():
    return comp([squelch(0.2, up=False), thud(130, 0.1, click=0.2)], offsets=[0, 0.12], gains=[0.8, 0.6])

# ---- Keen 5 ---------------------------------------------------------------------

@sfx(5, "mineexplode", ep=5)
def k5_mineexplode(): return boom(0.85, 450)

@sfx(6, "slicestarbounce", ep=5)
def k5_slicestar():
    return comp([tn(1400, 0.15, wave=2, punch=0.6), tn(2100, 0.1, wave=2, punch=0.5), thud(150, 0.08, click=0.2)], gains=[0.5, 0.3, 0.6])

@sfx(16, "spindredflyup", ep=5)
def k5_spindredflyup(): return glide(150, 0.25, 1.2, wave=0, duty=0.3)

@sfx(18, "carddoor", ep=5)
def k5_carddoor():
    a = tn(180, 0.3, wave=0, hold=0.6, duty=0.2, repeatSpeed=0.65, punch=0.0)
    s = comp([a, noise(0.4, 500, lp=0.3, hold=0.5), glide(400, 0.9, 0.8, wave=2)], gains=[0.4, 0.5, 0.3])
    return fx(s, "phaser", rate=1, depth=0.6, mix=0.3)

@sfx(27, "spirogripland", ep=5)
def k5_spirogripland(): return thud(90, 0.12)

@sfx(28, "spindredbounce", ep=5)
def k5_spindredbounce(): return comp([thud(120, 0.2), boing(140, 0.25, 0.4)], gains=[0.8, 0.4])

@sfx(29, "robotshoot", ep=56)
def robotshoot(): return zap(2400, 0.35, -2.0, wave=0, duty=0.15)

@sfx(30, "robotshothit", ep=56)
def robotshothit():
    return comp([noise(0.3, 1600, lp=0.7, lpramp=-0.3, punch=0.6), glide(1400, 0.2, -1.5, wave=2)], gains=[0.7, 0.4])

@sfx(31, "amptonwalk1", ep=5)
def k5_amptonwalk1(): return comp([tn(190, 0.04, wave=0, duty=0.5, punch=0.6), noise(0.02, 1500, lp=0.6)], gains=[0.5, 0.3])

@sfx(32, "amptonwalk2", ep=5)
def k5_amptonwalk2(): return comp([tn(160, 0.04, wave=0, duty=0.5, punch=0.6), noise(0.02, 1300, lp=0.6)], gains=[0.5, 0.3])

@sfx(33, "amptonstun", ep=5)
def k5_amptonstun():
    return comp([glide(1300, 0.6, -2.0, wave=0, duty=0.3, vibStrength=0.3, vibSpeed=0.8), noise(0.15, 2000, lp=0.8)], gains=[0.6, 0.4])

@sfx(36, "shelleyexplode", ep=5)
def k5_shelleyexplode(): return boom(0.7, 600, gain=0.9)

@sfx(37, "spindredslam", ep=5)
def k5_spindredslam():
    s = comp([thud(70, 0.3), noise(0.3, 600, lp=0.4, punch=0.8)], gains=[1.0, 0.6])
    return fx(s, "reverb", mix=0.25, size=0.6, decay=0.5)

@sfx(38, "masterattack", ep=5)
def k5_masterattack():
    s = comp([glide(300, 0.8, -1.6, wave=1, vibStrength=0.4, vibSpeed=0.8), noise(0.6, 1500, lp=0.6, lpramp=-0.2)], gains=[0.6, 0.5])
    return fx(s, "distortion", driveDb=14, mix=0.35)

@sfx(39, "masterteleport", ep=5)
def k5_masterteleport(): return sparkle(1.2, 1000, up=False)

@sfx(40, "shikadiattack", ep=5)
def k5_shikadiattack():
    a = tn(1300, 0.5, wave=8 if False else 0, hold=0.5, duty=0.1, repeatSpeed=0.8, vibStrength=0.6, vibSpeed=0.95, punch=0.3)
    s = comp([a, noise(1.0, 3000, lp=0.8, hold=0.4, vibStrength=0.5, vibSpeed=0.9)], gains=[0.5, 0.45])
    return fx(s, "phaser", rate=7, depth=0.9, mix=0.5)

@sfx(42, "shockshundbark", ep=5)
def k5_shockshundbark():
    return comp([glide(200, 0.25, 0.8, wave=1, vibStrength=0.4, vibSpeed=0.8, lpfFreq=0.5), noise(0.15, 900, lp=0.5)], gains=[0.8, 0.4])

@sfx(45, "barkshotdie", ep=5)
def k5_barkshotdie():
    return comp([glide(560, 0.5, -1.5, wave=0, duty=0.3), noise(0.2, 1500, lp=0.6, punch=0.6)], gains=[0.6, 0.4])

@sfx(52, "bwoom", ep=5)
def k5_bwoom():
    a = tn(950, 0.4, wave=0, hold=0.5, duty=0.3, vibStrength=0.25, vibSpeed=0.6, punch=0.4)
    b = tn(475, 0.4, wave=4, hold=0.5, punch=0.4)
    s = comp([a, b, thud(80, 0.3)], gains=[0.4, 0.6, 0.7])
    return fx(s, "reverb", mix=0.3, size=0.7, decay=0.5)

@sfx(53, "galaxyexplode", ep=5)
def k5_galaxyexplode():
    s = comp([boom(1.6, 300), boom(1.2, 500, gain=0.7), noise(1.5, 4000, lp=0.9, lpramp=-0.15, hold=0.4)], offsets=[0, 0.25, 0.1], gains=[1.0, 0.8, 0.4])
    return fx(s, "reverb", mix=0.35, size=0.9, decay=0.7)

@sfx(54, "galaxyexplodepre", ep=5)
def k5_galaxyexplodepre(): return hum(120, 1.6, wobble=0.6, wave=1)

@sfx(55, "keycard", ep=5)
def k5_keycard(): return jingle([1047, 1319, 1568], 0.07, last=0.35, sec=0.08, verb=0.25, duty=0.2)

@sfx(57, "fuse", ep=5)
def k5_fuse():
    return comp([noise(0.12, 3500, lp=0.95, punch=0.8), tn(1750, 0.12, wave=0, duty=0.1, punch=0.6)], offsets=[0, 0.02], gains=[0.7, 0.4])

@sfx(58, "sparkycharge", ep=5)
def k5_sparkycharge():
    return comp([glide(300, 1.2, 2.0, wave=0, duty=0.2, hold=0.4, vibStrength=0.3, vibSpeed=0.9), noise(1.0, 2000, lp=0.5, hold=0.4)], gains=[0.55, 0.3])

@sfx(59, "spherefulbounce", ep=5)
def k5_spherefulbounce(): return boing(270, 0.2, 0.5, wave=2)

@sfx(60, "door", ep=5)
def k5_door():
    a = tn(220, 0.3, wave=0, hold=0.35, duty=0.25, repeatSpeed=0.7, punch=0.0)
    return comp([a, noise(0.4, 600, lp=0.3, hold=0.3), thud(90, 0.15)], offsets=[0, 0, 0.6], gains=[0.4, 0.5, 0.7])

@sfx(61, "spirogripspin", ep=5)
def k5_spirogripspin(): return comp([tn(60, 0.2, wave=1, repeatSpeed=0.75, lpfFreq=0.5), noise(0.2, 400, lp=0.3)], gains=[0.7, 0.4])

@sfx(63, "elevator", ep=5)
def k5_elevator():
    a = glide(1200, 1.0, 1.8, wave=2, hold=0.1, vibStrength=0.2, vibSpeed=0.7)
    s = comp([a, tn(2400, 0.9, wave=0, duty=0.1, repeatSpeed=0.6), noise(0.9, 1500, lp=0.5)], gains=[0.6, 0.2, 0.3])
    return fx(s, "phaser", rate=4, depth=0.8, mix=0.4)

# ---- Keen 6 ---------------------------------------------------------------------

@sfx(5, "blooglegem", ep=6)
def k6_blooglegem():
    return comp([thud(150, 0.1, click=0.3), glide(540, 0.6, -2.0, wave=2), sparkle(0.4, 1600)], offsets=[0, 0.03, 0], gains=[0.5, 0.5, 0.4])

@sfx(6, "blorbbounce", ep=6)
def k6_blorbbounce(): return comp([tn(130, 0.18, wave=4, punch=0.5), tn(1100, 0.12, wave=2, punch=0.5)], gains=[0.8, 0.3])

@sfx(18, "gemdoor", ep=6)
def k6_gemdoor(): return k4_gemdoor()

@sfx(24, "bipshipcrash", ep=6)
def k6_bipshipcrash():
    return comp([glide(340, 0.5, -2.0, wave=0, duty=0.3), boom(0.8, 350)], offsets=[0, 0.4], gains=[0.5, 0.9])

@sfx(26, "bobbajump", ep=6)
def k6_bobbajump(): return glide(300, 0.2, 1.0, wave=4, punch=0.3)

@sfx(27, "bobbaland", ep=6)
def k6_bobbaland(): return thud(110, 0.18)

@sfx(31, "bobbashoot", ep=6)
def k6_bobbashoot():
    return comp([glide(650, 0.35, -1.2, wave=1, lpfFreq=0.6), noise(0.3, 1200, lp=0.6, lpramp=-0.2)], gains=[0.6, 0.5])

@sfx(33, "satellite", ep=6)
def k6_satellite(): return hum(300, 1.6, wobble=0.4)

@sfx(36, "gikjump", ep=6)
def k6_gikjump(): return glide(700, 0.13, 1.8, wave=0, duty=0.3)

@sfx(37, "gikland", ep=6)
def k6_gikland(): return comp([tn(200, 0.05, wave=0, duty=0.5, punch=0.6), noise(0.04, 1200, lp=0.5)], gains=[0.6, 0.4])

@sfx(38, "orbatrixbump", ep=6)
def k6_orbatrixbump(): return boing(380, 0.4, -1.4, wave=2)

@sfx(42, "reflect", ep=6)
def k6_reflect():
    s = comp([tn(1550, 0.3, wave=2, punch=0.7), tn(2325, 0.2, wave=2, punch=0.6)], gains=[0.7, 0.4])
    return fx(s, "reverb", mix=0.35, size=0.6, decay=0.6)

@sfx(45, "getspecial", ep=6)
def k6_getspecial():
    return comp([sparkle(1.4, 2900, up=False), jingle([659, 784, 988, 1319], 0.12, last=0.6, sec=0.1, wave=2, verb=0.3)], gains=[0.5, 0.8])

@sfx(51, "ceilickattack", ep=6)
def k6_ceilickattack():
    return comp([glide(140, 0.25, 1.0, wave=1, lpfFreq=0.5, vibStrength=0.4, vibSpeed=0.9), noise(0.15, 800, lp=0.5)], gains=[0.7, 0.5])

@sfx(52, "blooguardstomp", ep=6)
def k6_blooguardstomp():
    s = comp([thud(60, 0.5), noise(0.4, 400, lp=0.35, punch=0.8)], gains=[1.0, 0.5])
    return fx(s, "reverb", mix=0.25, size=0.7, decay=0.5)

@sfx(53, "throwrope", ep=6)
def k6_throwrope():
    return comp([noise(0.3, 1500, lp=0.6, lpramp=-0.2), glide(550, 0.25, -0.8, wave=2), tn(280, 0.08, wave=4, punch=0.6)], offsets=[0, 0, 0.4], gains=[0.5, 0.4, 0.6])

@sfx(54, "rocketfly", ep=6)
def k6_rocketfly():
    return comp([noise(0.9, 300, lp=0.35, hold=0.5, vibStrength=0.3, vibSpeed=0.6), tn(55, 0.6, wave=1, hold=0.3, lpfFreq=0.3)], gains=[0.8, 0.5])

@sfx(55, "ceilicklaugh", ep=6)
def k6_ceilicklaugh():
    ns = [glide(f, 0.08, -0.3, wave=1, lpfFreq=0.5, vibStrength=0.3, vibSpeed=0.9) for f in (520, 560, 950, 880, 700, 590)]
    return comp(ns, offsets=[i * 0.09 for i in range(6)])

@sfx(56, "rocketlaunch", ep=6)
def k6_rocketlaunch():
    s = comp([noise(0.6, 400, lp=0.5, hold=0.2, lpramp=0.1), glide(80, 0.5, 1.5, wave=1, lpfFreq=0.4), thud(70, 0.2)], gains=[0.8, 0.5, 0.7])
    return fx(s, "reverb", mix=0.2, size=0.6, decay=0.4)

@sfx(57, "grabbiter", ep=6)
def k6_grabbiter():
    a = tn(310, 0.5, wave=1, hold=0.5, lpfFreq=0.45, vibStrength=0.6, vibSpeed=0.5)
    s = comp([a, glide(250, 1.2, -2.0, wave=4, hold=0.3)], gains=[0.6, 0.6])
    return fx(s, "reverb", mix=0.25, size=0.5, decay=0.5)

@sfx(58, "machine1", ep=6)
def k6_machine1():
    return comp([tn(330, 0.15, wave=0, hold=0.3, duty=0.3, repeatSpeed=0.72), noise(0.5, 700, lp=0.3, hold=0.2)], gains=[0.4, 0.4])

@sfx(59, "machine2", ep=6)
def k6_machine2():
    return comp([glide(410, 0.6, 1.4, wave=0, duty=0.3, hold=0.1), noise(0.5, 900, lp=0.35)], gains=[0.45, 0.35])


def stats(path):
    w = wave.open(path); n = w.getnframes(); ch = w.getnchannels()
    a = struct.unpack(f"<{n*ch}h", w.readframes(n))
    nz = [i for i, x in enumerate(a) if abs(x) > 300]
    return (nz[-1] / ch / w.getframerate() if nz else 0)

if __name__ == "__main__":
    names = sys.argv[1:] or list(D)
    try:
        out = json.load(open("rendered.json"))
    except Exception:
        out = {}
    for name in names:
        n, f, ep = D[name]
        s = f()
        s = m.call("normalize", sound=s, targetPeak=0.7)
        tag = f"k{ep}-" if ep else ""
        s["name"] = f"keen-{tag}{n:02d}-{name}"
        r = m.call("render_wav", sound=s, name=s["name"])
        m.call("save", sound=s, name=f"Keen/{s['name']}", overwrite=True)
        eps = [str(e) for e in str(ep)] if ep else [""]
        for e in eps:
            out[(f"k{e}/" if e else "") + f"{n:02d}"] = r["path"]
        print(f"{tag or '   '}{n:2d} {name:16s} {stats(r['path']):.2f}s")
    json.dump(out, open("rendered.json", "w"), indent=0)
