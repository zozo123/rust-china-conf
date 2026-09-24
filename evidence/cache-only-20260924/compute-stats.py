#!/usr/bin/env python3
"""Recompute every median, range and ratio quoted for this run, from the two
retained sample logs only. No number in the write-up is entered by hand."""
import json, statistics as st
p1=[json.loads(l) for l in open("cache-only-20260924T122940Z/build-proof/samples.jsonl")]
p2=[json.loads(l) for l in open("cache-extras-20260924T123437Z/cache-extras/samples.jsonl")]
def agg(rows):
    out={}
    for m in dict.fromkeys(r["mode"] for r in rows):
        g=[r for r in rows if r["mode"]==m]; w=sorted(r["wall_ms"] for r in g)
        hs=sorted({f'{r.get("cache_hits")}/{(r.get("cache_hits") or 0)+(r.get("cache_misses") or 0)}'
                   for r in g if r.get("cache_hits") is not None}) or ["n/a"]
        rt=sorted({str(r.get("remote_tasks")) for r in g if r.get("remote_tasks") is not None}) or ["n/a"]
        out[m]=(len(g),st.median(w),w[0],w[-1],",".join(hs),",".join(rt))
    return out
for label,rows in (("PHASE 1 (gated by swf-cli build-proof --distribution excluded, 5/mode, rotating)",p1),
                   ("PHASE 2 (side measurements, NOT gated by build-proof, 5/mode, rotating)",p2)):
    print(label)
    for m,(n,md,lo,hi,h,rt) in agg(rows).items():
        print(f"  {m:<20} n={n} median={md:>8.0f}ms  range={lo}-{hi}  hits/total={h:<7} remote_tasks={rt}")
med=lambda rows,m: st.median([r["wall_ms"] for r in rows if r["mode"]==m])
nat1,cold1,warm1=med(p1,"native"),med(p1,"ib-cold"),med(p1,"ib-parent-warm")
nat2,d0,lld,f=med(p2,"native"),med(p2,"native-debug0"),med(p2,"native-lld"),med(p2,"ib-warm-identical")
print("\nANSWERS (medians only; ratios are median/median)")
print(f"  native anchor phase1={nat1:.0f}ms phase2={nat2:.0f}ms (drift {100*(nat2-nat1)/nat1:+.1f}%)")
print(f"  full-reuse vs native       : {nat2/f:.2f}x faster, saves {nat2-f:.0f}ms ({f:.0f} vs {nat2:.0f})")
print(f"  one-file-change vs native  : {nat1/warm1:.2f}x faster, saves {nat1-warm1:.0f}ms ({warm1:.0f} vs {nat1:.0f})")
print(f"  cold cache vs native       : {nat1/cold1:.2f}x ({cold1:.0f} vs {nat1:.0f}) = {cold1-nat1:.0f}ms SLOWER")
print(f"  full-reuse vs ib-cold      : {cold1/f:.2f}x faster, saves {cold1-f:.0f}ms")
print(f"  one-file vs ib-cold        : {cold1/warm1:.2f}x faster, saves {cold1-warm1:.0f}ms")
print(f"  debug=0 vs native          : {nat2/d0:.2f}x faster, saves {nat2-d0:.0f}ms ({d0:.0f} vs {nat2:.0f})")
print(f"  explicit lld vs native     : {nat2/lld:.3f}x ({lld:.0f} vs {nat2:.0f}); lld is ALREADY the default,")
print( "                               see cache-extras-*/cache-extras/linker-and-debuginfo-lever-check.txt")
print(f"  full-reuse vs debug=0      : {d0/f:.2f}x faster")
print(f"  one-file vs debug=0        : {d0/warm1:.2f}x faster")
