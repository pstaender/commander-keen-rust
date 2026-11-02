import re,glob,os,ast
strs={}
for l in open('re/dgroup_strings.txt'):
    a,s=l.split(' ',1); strs[int(a,16)]=ast.literal_eval(s.strip())
keys=sorted(strs)
import bisect
def lookup(off):
    i=bisect.bisect_right(keys,off)-1
    if i<0: return None
    k=keys[i]; s=strs[k]
    if off<=k+len(s): return s
    return None
pat=re.compile(r's_[A-Za-z0-9_]*?_3f90_([0-9a-f]{4})(?:\s*\+\s*(0x[0-9a-f]+|\d+))?')
pat2=re.compile(r'\(char \*\)0x([0-9a-f]{3,4})\b')
for f in sorted(glob.glob('re/decomp/*.c')):
    t=open(f).read()
    found=[]
    for m in pat.finditer(t):
        off=int(m.group(1),16)+(int(m.group(2),0) if m.group(2) else 0)
        s=lookup(off)
        if s and s not in found: found.append(s)
    name=os.path.basename(f)[:-2]
    first=t.split('\n')[0]
    print(name, '|', ' || '.join(x.replace('\n','\\n')[:40] for x in found[:6]))
