#!/usr/bin/env python3
"""Independently compare POLYLINE_2D child order using ezdwg 0.19.2.

Usage: check_poly_order_independent.py before.dwg after.dwg [hex-handle]
Prints one line per changed polyline; exit 0 means every polyline preserved
its VERTEX handle order, SEQEND handle and point sequence.
"""
import sys
import ezdwg

def load(path):
    doc = ezdwg.read(path)
    return {
        e.handle: e.dxf
        for e in doc.entities().iter_entities()
        if e.dxftype == "POLYLINE_2D"
    }

before = load(sys.argv[1])
after = load(sys.argv[2])
only = int(sys.argv[3], 16) if len(sys.argv) > 3 else None
diffs = 0
for handle, a in before.items():
    if only is not None and handle != only:
        continue
    b = after.get(handle)
    if b is None:
        print(f"MISSING {handle:X}")
        diffs += 1
        continue
    fields = ("vertex_handles", "seqend_handle", "points")
    if any(a.get(k) != b.get(k) for k in fields):
        diffs += 1
        print(f"DIFF {handle:X}")
        for k in fields:
            if a.get(k) != b.get(k):
                print(f"  {k}: {a.get(k)} -> {b.get(k)}")
print(f"POLYLINES={len(before)} AFTER={len(after)} DIFFS={diffs}")
sys.exit(1 if diffs else 0)
