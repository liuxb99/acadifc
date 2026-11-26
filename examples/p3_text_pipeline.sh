#!/bin/sh
# Phase 6 P3: minimal reproducible text-fidelity pipeline (no renderer).
# Usage: p3_text_pipeline.sh <fixture.dwg> <outdir>
set -eu
FIXTURE=$1
OUT=$2
mkdir -p "$OUT"
BIN=target/debug/examples
PATH=/root/.cargo/bin:$PATH cargo build --example field_diff_roundtrip --example diag_block_transform \
  --example diag_attrib_owner --example check_poly_order >/dev/null

# 1) raw bytes -> decoder -> model -> encoder -> DWG (genuine no-op)
"$BIN/field_diff_roundtrip" "$FIXTURE" "$OUT/noop.dwg" > "$OUT/native_fields.tsv"

# 2) independent decoder before / after
python3 examples/diag_text_independent.py "$FIXTURE" > "$OUT/independent_A.tsv"
python3 examples/diag_text_independent.py "$OUT/noop.dwg" > "$OUT/independent_D.tsv"
python3 examples/diag_text_compare.py "$OUT/independent_A.tsv" "$OUT/independent_D.tsv" > "$OUT/text_diff.txt"

# 3) owner/block/transform and child order
"$BIN/diag_block_transform" "$FIXTURE" A > "$OUT/block_A.tsv"
"$BIN/diag_block_transform" "$OUT/noop.dwg" D > "$OUT/block_D.tsv"
"$BIN/diag_attrib_owner" "$FIXTURE" A > "$OUT/attrib_A.tsv"
"$BIN/diag_attrib_owner" "$OUT/noop.dwg" D > "$OUT/attrib_D.tsv"
python3 examples/check_poly_order_independent.py "$FIXTURE" "$OUT/noop.dwg" > "$OUT/poly_order.txt"

echo "native fields: $OUT/native_fields.tsv"
echo "independent text diff: $(tail -1 "$OUT/text_diff.txt")"
echo "polygon order: $(tail -1 "$OUT/poly_order.txt")"
echo "renderer visual comparison: NOT_TESTED (no AutoCAD 2006 / font renderer in this environment)"
