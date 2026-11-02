import struct,sys
exec(open(__import__("os").path.join(__import__("os").path.dirname(__import__("os").path.abspath(__file__)), "explore.py")).read())
D=lin(0x3f900,59952)
def rec(i):
    return struct.unpack_from('<8h7H',D,i)
def fp(off,seg):
    if off==0 and seg==0: return None
    return (seg+0x1000)*16+off
def plaus(i):
    l,r,prog,skip,push,tic,xm,ym,t0,t1,c0,c1,r0,r1,nx=rec(i)
    if not(0<=prog<=4 and skip in(0,1) and push in(0,1) and 0<=tic<2000 and -1<=l<600 and -1<=r<600): return False
    for o,s in ((t0,t1),(c0,c1),(r0,r1)):
        if (o,s)!=(0,0) and not (0x374<=s<0x1d88): return False
    if nx!=0 and not (0x700<=nx<0x3400): return False
    return True
cand=[i for i in range(0x700,0x3400,2) if plaus(i)]
# keep only those referenced as nextstate by another plausible record or forming chains
refs=set()
for i in cand:
    nx=rec(i)[14]
    if nx: refs.add(nx)
good=set()
for i in cand:
    if i in refs or (rec(i)[14] in cand): good.add(i)
# remove overlapping: prefer those in refs
out=[]
last=-100
for i in sorted(good):
    if i < last+30 and i not in refs: continue
    out.append(i); last=i
for i in out:
    l,r,prog,skip,push,tic,xm,ym,t0,t1,c0,c1,r0,r1,nx=rec(i)
    f=lambda o,s: ('%05x'%fp(o,s)) if fp(o,s) else '-'
    print('%04x L%3d R%3d p%d s%d f%d t%3d xm%4d ym%4d th %s co %s re %s nx %04x %s'%(i,l,r,prog,skip,push,tic,xm,ym,f(t0,t1),f(c0,c1),f(r0,r1),nx,'' if i in refs else '(unref)'))
