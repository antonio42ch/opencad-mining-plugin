# From the legacy C# tools (AutoCAD .NET) to Open Mining

| Legacy | Open Mining |
|---|---|
| `PST_SONDAJES` palette (WPF/MVVM) | `MIN_DH_PANEL` panel + `MIN_DH_*` commands |
| `PSTSOLIDMINE` palette | `MIN_UG_PANEL` panel + `MIN_UG_*` commands |
| `pstgeomapper` palette / `InsertarMapeoCmd` | `MIN_GM_PANEL` panel + `MIN_GM_TRACE` (coordinates) / `MIN_GM_TRACE_PICK` (click). The text annotations of the legacy form were dropped on purpose; the data goes to XDATA |
| Layers `PST_Trace`, `PST_Collar`, `&&PST_<field>` | `MIN_DH_TRAZA`, `MIN_DH_COLLAR`, `MIN_DH_LEY_<ATTR>` |
| Layers `PST_SOLIDO`, `PST_SECCION` | `MIN_UG_SOLIDO`, `MIN_UG_SECCION` |
| XData `PST_SONDAJES` / `GEO_ADDIN_DATA` (free text) | XDATA `OPENMINING` schema v1 (typed keys, class identity) |
| `Solid3d.CreateLoftedSolid` | `ClosedMesh` (watertight mesh, volume checked) |

## Behaviour changes (fixes)

1. **Dogleg**: the legacy code used the from-horizontal formula with
   inclinations from vertical (up to +27 % in ΔZ for near-vertical holes with
   noisy azimuth). Now computed from tangent vectors. Results differ from
   legacy-processed data in those cases.
2. Interpolation inside a survey interval uses minimum curvature (was linear);
   interval centroids lie on the trace at (from+to)/2 (was the chord midpoint).
3. R05 rejects non-finite From (was silently kept). R06 compares against the
   furthest accepted To. R07 also applies to geology. R03 no longer mutates
   the input collars.
4. Survey FROM-TO files keep the TO station (was discarded).
5. Boundary methods report when a polyline does not cross a section (was silent)
   and accept a 3D tangent for steep ramps.
6. Bearing parsing validates the 0–90° quadrant angle and reports errors
   (was 0); accepts `O` (oeste) and decimal comma.
7. Horseshoe crowns always have an apex vertex (odd arc point count).
8. Per-hole drawing errors name the hole (were only counted).
9. Grade colour scale can be global (new default) or per hole (legacy).
