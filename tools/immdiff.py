#!/usr/bin/env python3
"""Lists the instructions whose operands differ between a Keen 6 function
and its Keen 4/5 counterpart (EP=4|5), after aligning the instruction
sequences. DS memory operands are translated with re/kN/globals_map.txt,
far calls with funcs_map.txt, jumps are ignored; what remains are mostly
episode constants (chunk, sprite, sound numbers, state offsets, tables).
Usage: EP=4 immdiff.py <k6addr> [epaddr]    or    EP=4 immdiff.py -f file.rs
(-f: every '/// Name (0xADDR)' function of a Rust file)"""
import os, sys, re, subprocess, tempfile, difflib, importlib.util
ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..')
EP = os.environ.get('EP', '4')
D = os.path.join(ROOT, 're', 'k' + EP)
CFG = {'4': ('re/k4/KEEN4E_unpacked.exe', 0x2e00, 're/k4/procedures.txt'),
       '5': ('re/k5/KEEN5E_unpacked.exe', 0x2c00, 're/k5/procedures.txt'),
       '6': ('re/KEEN6C_unpacked.exe', 0x2c00, 're/procedures.txt')}
def load(ep):
    exe, hdr, pf = CFG[ep]
    img = open(os.path.join(ROOT, exe), 'rb').read()[hdr:]
    starts = sorted(int(l.split()[0], 16) for l in open(os.path.join(ROOT, pf)) if 'switchD' not in l and 'caseD' not in l)
    return img, starts
IMG = {e: load(e) for e in (EP, '6')}
def dis(ep, a):
    img, starts = IMG[ep]
    nxt = [s for s in starts if s > a]
    e = min(nxt[0] if nxt else a + 0x400, a + 0x3000)
    with tempfile.NamedTemporaryFile(delete=False) as f:
        f.write(img[a - 0x10000:e - 0x10000]); name = f.name
    out = subprocess.run(['ndisasm', '-b16', '-o', str(a & 0xf), name], capture_output=True, text=True).stdout
    os.unlink(name)
    res = []
    for l in out.splitlines():
        p = l.split(None, 2)
        if len(p) == 3: res.append(p[2])
    return res
gmap = {}
for l in open(os.path.join(D, 'globals_map.txt')):
    p = l.split(); gmap[int(p[0], 16)] = int(p[1], 16)
fmap, k6names = {}, {}
for l in open(os.path.join(D, 'funcs_map.txt')):
    p = l.split(); fmap[int(p[0], 16)] = int(p[1], 16); k6names[int(p[1], 16)] = p[2]
rev = {}
for a, b in fmap.items(): rev.setdefault(b, a)
def st(ep):
    spec = importlib.util.spec_from_file_location('st', os.path.join(ROOT, 'tools', 'states_k%s.py' % ep))
    m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m); return m.STATES
ST6, STE = st('6'), st(EP)
def shape(i):
    m = i.split()[0]
    return m + re.sub(r'0x[0-9a-f]+|\b\d+\b', 'N', i[len(m):]).replace(' ', '')
def norm(ins, ep):
    # translate DS offsets to K6 ones, far calls to K6 targets
    m = re.match(r'call word (0x[0-9a-f]+):word (0x[0-9a-f]+)', ins)
    if m:
        lin = (int(m.group(1), 16) + 0x1000) * 16 + int(m.group(2), 16)
        if ep != '6': lin = fmap.get(lin, lin)
        return 'callf %s' % k6names.get(lin, '%05x' % lin)
    if ins.split()[0].startswith('j') or ins.startswith('call 0x') or ins.startswith('loop'):
        return None
    # negative displacements are DS offsets too: [bx-0x5c88] = [bx+0xa378]
    ins = re.sub(r'-0x([0-9a-f]{3,4})\]', lambda mm: '+0x%x]' % (0x10000 - int(mm.group(1), 16)), ins)
    def num(mm):
        v = int(mm.group(1), 16)
        if v >= 0x100 and ep != '6' and v in gmap:
            v = gmap[v]
        return '0x%x' % v
    return re.sub(r'0x([0-9a-f]+)', num, ins)
def stname(v, ep):
    s = ST6 if ep == '6' else STE
    return s.get(v)
def compare(b, a):
    x, y = dis('6', b), dis(EP, a)
    sm = difflib.SequenceMatcher(None, [shape(i) for i in x], [shape(i) for i in y], autojunk=False)
    out = []
    for tag, i1, i2, j1, j2 in sm.get_opcodes():
        if tag == 'equal':
            for p, q in zip(x[i1:i2], y[j1:j2]):
                np_, nq = norm(p, '6'), norm(q, EP)
                if np_ is None or np_ == nq: continue
                # same state under both names?
                vs = re.findall(r'0x([0-9a-f]+)', p); ws = re.findall(r'0x([0-9a-f]+)', q)
                if vs and ws and len(vs) == len(ws):
                    sn = [(stname(int(v, 16), '6'), stname(int(w, 16), EP)) for v, w in zip(vs, ws)]
                    if all(s1 and s1 == s2 for s1, s2 in sn): continue
                    note = ' '.join('%s/%s' % (s1, s2) for s1, s2 in sn if s1 or s2)
                else:
                    note = ''
                out.append('    k6: %-36s k%s: %-36s %s' % (p, EP, q, note))
        else:
            out.append('    [%s] k6: %s | k%s: %s' % (tag, ' ; '.join(x[i1:i2])[:150], EP, ' ; '.join(y[j1:j2])[:150]))
    return out
def one(b, a=None):
    a = a if a is not None else rev.get(b)
    if a is None:
        print('%05x %s: no match in K%s' % (b, k6names.get(b, ''), EP)); return
    out = compare(b, a)
    print('== K6 %05x %s <-> K%s %05x: %d differences' % (b, k6names.get(b, ''), EP, a, len(out)))
    print('\n'.join(out))
if sys.argv[1] == '-f':
    for m in re.finditer(r'/// ([A-Za-z_0-9]+)[^\n]*?\((0x[0-9a-f]{5})', open(sys.argv[2]).read()):
        one(int(m.group(2), 16))
else:
    one(int(sys.argv[1], 16), int(sys.argv[2], 16) if len(sys.argv) > 2 else None)
