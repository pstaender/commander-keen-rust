import struct,re,glob
exec(open(__import__("os").path.join(__import__("os").path.dirname(__import__("os").path.abspath(__file__)), "explore.py")).read())
D=lin(0x3f900,59952)
def rec(i): return struct.unpack_from('<8h7H',D,i)
def loose(i):
    l,r,prog,skip,push,tic,xm,ym,t0,t1,c0,c1,r0,r1,nx=rec(i)
    if not(0<=prog<=4 and skip in(0,1) and push in(0,1) and 0<=tic<3000): return False
    for o,s in ((t0,t1),(c0,c1),(r0,r1)):
        if (o,s)!=(0,0) and not (0x374<=s<0x1d88): return False
    return nx==0 or 0x700<=nx<0x3500
seeds=set()
for f in glob.glob('re/decomp/*.c'):
    for m in re.finditer(r'0x([0-9a-f]{3,4})\b',open(f).read()):
        v=int(m.group(1),16)
        if 0x700<=v<0x3500 and v%2==0 and loose(v): seeds.add(v)
for l in open('re/states_raw.txt'): seeds.add(int(l.split()[0],16))
# closure
todo=list(seeds); allst=set()
while todo:
    s=todo.pop()
    if s in allst or not loose(s): continue
    allst.add(s); nx=rec(s)[14]
    if nx: todo.append(nx)
# keep consistent: drop states overlapping another accepted state by <30 bytes unless referenced by nextstate
refs={rec(s)[14] for s in allst}
st=sorted(allst)
final=[]
for s in st:
    if final and s-final[-1]<30:
        # overlapping; prefer one in refs
        if s in refs and final[-1] not in refs: final[-1]=s
        continue
    final.append(s)
def fp(o,s): return None if (o,s)==(0,0) else (s+0x1000)*16+o
for s in final:
    l,r,prog,skip,push,tic,xm,ym,t0,t1,c0,c1,r0,r1,nx=rec(s)
    f=lambda o,sg: ('%05x'%fp(o,sg)) if fp(o,sg) else '-'
    print('%04x L%3d R%3d p%d s%d f%d t%3d xm%4d ym%4d th %s co %s re %s nx %04x'%(s,l,r,prog,skip,push,tic,xm,ym,f(t0,t1),f(c0,c1),f(r0,r1),nx))
