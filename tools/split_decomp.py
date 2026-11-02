"""Split REA batch_decompile results into one file per function.
Usage: split_decomp.py OUTDIR result.json..."""
import json, sys, os
out = sys.argv[1]
os.makedirs(out, exist_ok=True)
for f in sys.argv[2:]:
    j = json.load(open(f))
    for it in j['normalized_result']['items']:
        a = it['address']; pc = it.get('pseudocode') or ''
        open(f'{out}/{int(a,16):05x}.c', 'w').write(f'// {a} {it["procedure"].get("name")}\n' + pc)
print(len(os.listdir(out)))
