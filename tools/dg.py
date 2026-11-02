import struct,sys
exec(open(__import__("os").path.join(__import__("os").path.dirname(__import__("os").path.abspath(__file__)), "explore.py")).read())
D=lin(DGLIN,0xffff)
off=int(sys.argv[1],16); n=int(sys.argv[2]); kind=sys.argv[3] if len(sys.argv)>3 else 'w'
if kind=='w': print(list(struct.unpack_from('<%dh'%n,D,off)))
elif kind=='u': print(list(struct.unpack_from('<%dH'%n,D,off)))
elif kind=='b': print(list(D[off:off+n]))
elif kind=='s': print(repr(D[off:off+n]))
