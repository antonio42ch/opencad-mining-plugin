# Reference data set

Analytical cases with closed-form answers (tolerance 1e-6 m):

| Hole | Geometry | Expected end of hole |
|---|---|---|
| VERT | vertical, no surveys, 100 m | (500000, 8500000, 3900) |
| INCL | straight, az 90, dip -60, 200 m | (500000+100+200·sin30, 8500000, 4000−200·cos30) |
| ARC | circular build-up vertical→horizontal north, R = 100 m, MD = πR/2 | (500200, 8500100, 3900) |

`collar.csv` is written like Excel on a Spanish Windows (`;`, decimal comma,
CRLF) to exercise the importer. When a commercial package is available, add
its desurvey output here as `*_expected.csv` and compare with the same
tolerance.
