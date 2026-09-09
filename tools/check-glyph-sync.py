#!/usr/bin/env python3
"""index.html and workbench.html are each standalone single files, so drawGlyph()
is necessarily duplicated. This makes the duplication safe: the two copies must
be byte-identical or the suite fails."""
import sys, pathlib, hashlib
root = pathlib.Path(__file__).resolve().parent.parent
def grab(name):
    src = (root / name).read_text()
    i = src.index("function drawGlyph(")
    j = src.index("\n}", src.index("g.restore();", i)) + 2
    return src[i:j]
try:
    a, b = grab("index.html"), grab("workbench.html")
except FileNotFoundError as e:
    print("  skipped —", e.filename, "not present yet"); sys.exit(0)
ha, hb = hashlib.sha256(a.encode()).hexdigest()[:12], hashlib.sha256(b.encode()).hexdigest()[:12]
if a != b:
    print("  FAIL drawGlyph differs: index.html %s vs workbench.html %s" % (ha, hb)); sys.exit(1)
print("  OK drawGlyph identical in both (%s, %d chars)" % (ha, len(a)))
