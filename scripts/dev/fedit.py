"""Whitespace-tolerant source edits (development helper)."""
import re, sys
def _rx(a):
    toks = re.findall(r'\w+|[^\w\s]', a)
    return r'\s*'.join(re.escape(t) for t in toks)
def edit(p, pairs, strict=True):
    s = open(p).read(); bad = []
    for a, b in pairs:
        if a in s:
            s = s.replace(a, b, 1); continue
        m = re.search(_rx(a), s)
        if m:
            s = s[:m.start()] + b + s[m.end():]
        else:
            bad.append(a[:80])
    open(p, 'w').write(s)
    for x in bad: print(f"NOT FOUND in {p}: {x!r}")
    if bad and strict: sys.exit(1)
