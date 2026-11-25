#!/usr/bin/env python3
"""Compare two diag_text_independent.py TSV dumps by handle.

Usage: diag_text_compare.py before.tsv after.tsv
"""
import sys

NUM = {
    4: "insert", 5: "align", 6: "height", 7: "width", 8: "rotation",
    9: "oblique", 10: "normal", 11: "halign", 12: "valign",
    13: "thickness", 14: "gen", 15: "attach", 16: "drawdir",
    17: "spacing", 18: "spacing_style", 19: "text_direction", 20: "style",
}

def load(path):
    out = {}
    for line in open(path, encoding="utf-8"):
        if line.startswith("#") or not line.strip():
            continue
        row = line.rstrip("\n").split("\t")
        out[row[1]] = row
    return out

def close(a, b):
    if a == b:
        return True
    try:
        return abs(float(a) - float(b)) < 1e-5
    except Exception:
        return False

before = load(sys.argv[1])
after = load(sys.argv[2])
print("BEFORE=%d AFTER=%d MISSING=%d NEW=%d" % (
    len(before), len(after), len(set(before) - set(after)), len(set(after) - set(before))))
count = 0
for h in sorted(before):
    a = before[h]
    b = after.get(h)
    if b is None:
        continue
    diffs = []
    for i, name in NUM.items():
        if i < len(a) and i < len(b) and not close(a[i], b[i]):
            diffs.append("%s:%s->%s" % (name, a[i], b[i]))
    if len(a) > 21 and len(b) > 21 and a[-1] != b[-1]:
        diffs.append("text:%s->%s" % (a[-1][:60], b[-1][:60]))
    if diffs:
        count += 1
        if count <= 40:
            print(h, a[0], a[3], " | ".join(x[:120] for x in diffs))
print("TOTAL_DIFF=%d" % count)
