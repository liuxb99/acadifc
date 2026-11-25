#!/usr/bin/env python3
"""Independent-reader (ezdwg 0.19.2) dump of TEXT/MTEXT/ATTRIB geometry.

Usage: diag_text_independent.py <file.dwg> > out.tsv
"""
import sys
import ezdwg

path = sys.argv[1]
doc = ezdwg.read(path)
layers = {layer.handle: layer.name for layer in doc.layers}
styles = {}
try:
    for st in doc.styles:
        styles[getattr(st, "handle", None)] = getattr(st, "name", "?")
except Exception:
    pass

def name_of(table, h):
    return table.get(h, str(h)) if h is not None else "-"

def f(v):
    return "-" if v is None else f"{float(v):.12f}"

def v3(v):
    if v is None:
        return "-"
    return ",".join(f"{float(x):.12f}" for x in v)

print("#source\tEZDWG\tversion\t%s" % doc.version)
for e in doc.entities().iter_entities():
    d = e.dxf
    t = e.dxftype
    if t not in ("TEXT", "MTEXT", "ATTRIB", "ATTDEF"):
        continue
    print("\t".join([
        t, str(e.handle), str(d.get("owner_handle")), name_of(layers, d.get("layer_handle")),
        v3(d.get("insert") or d.get("insertion_point")),
        v3(d.get("align_point")),
        f(d.get("height") if t != "MTEXT" else d.get("char_height")),
        f(d.get("width") if t != "MTEXT" else d.get("rect_width")),
        f(d.get("rotation")), f(d.get("oblique")),
        v3(d.get("extrusion") or d.get("normal")),
        str(d.get("halign")), str(d.get("valign")),
        f(d.get("thickness")), str(d.get("text_generation_flag")),
        str(d.get("attachment_point") or "-"),
        str(d.get("drawing_direction") or "-"),
        f(d.get("line_spacing_factor")), str(d.get("line_spacing_style")),
        v3(d.get("text_direction")),
        name_of(styles, d.get("style_handle")),
        str(d.get("text") or d.get("value") or "").replace("\t", " ").replace("\n", "\\P"),
    ]))
