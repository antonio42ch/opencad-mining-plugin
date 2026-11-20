#!/usr/bin/env python3
"""End-to-end smoke test of Open Mining inside a real Open CAD Studio.

    python3 scripts/smoke_serve.py /path/to/OpenCADStudio-or-AppRun [--plugins DIR] [--install]

Starts `OpenCADStudio --serve` (headless, JSON lines over stdin/stdout) with an
isolated configuration and plugins folder, then drives every module through
the same commands and panel fields a user would:

  plugin loaded + panels registered -> geology trace + projection along dip ->
  axis solid + sections -> drillholes load/draw -> composites -> variogram
  model -> block model estimate -> block view -> geotechnics (load, classify,
  Hoek-Brown, block model columns, view)

Each check is reported; on failure the host's command-line history is printed.
Exit code 0 = all required checks passed.
"""
import argparse
import itertools
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DATA = ROOT / "crates/drillhole_engine/tests/data"
GT_DATA = ROOT / "crates/geotech_engine/tests/data"
RESULT = "OPENMINING_RESULT "
PANELS = ["min_drillholes", "min_workings", "min_geomap", "min_axes", "min_sections", "min_rs_comp", "min_rs_vario", "min_rs_bm", "min_gt_data", "min_gt_hb", "min_gt_model"]


class Server:
    def __init__(self, exe, env):
        self.p = subprocess.Popen([exe, "--serve"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=sys.stderr, text=True, env=env)
        self.ids = itertools.count(1)
        line = self.p.stdout.readline()
        if not line:
            raise RuntimeError("the host exited before greeting (see stderr above)")
        self.greeting = json.loads(line)

    def send(self, payload, quiet=False):
        self.p.stdin.write(json.dumps(payload) + "\n")
        self.p.stdin.flush()
        line = self.p.stdout.readline()
        if not line:
            raise RuntimeError("the host closed the connection")
        reply = json.loads(line)
        if not quiet:
            print(">>", json.dumps(payload)[:300])
            print("<<", json.dumps(reply)[:600])
        return reply

    def control(self, op, quiet=False, **kw):
        """Request of the control protocol (envelope + unique request id)."""
        return self.send(dict(protocol=1, op=op, request_id=f"smoke-{next(self.ids)}", **kw), quiet=quiet)

    def close(self):
        try:
            self.p.stdin.close()
            self.p.wait(timeout=30)
        except Exception:
            self.p.kill()


def entities_of(reply):
    for k in ("entities", "results", "items", "matches"):
        if isinstance(reply.get(k), list):
            return reply[k]
    if isinstance(reply.get("result"), dict):
        return entities_of(reply["result"])
    return []


def count(reply):
    for k in ("count", "total", "matched"):
        if isinstance(reply.get(k), int):
            return reply[k]
    return len(entities_of(reply))


def handles(reply):
    return [e["handle"] for e in entities_of(reply) if isinstance(e, dict) and e.get("handle")]


class Checks:
    def __init__(self):
        self.failed, self.warned, self.passed = [], [], []

    def need(self, ok, what):
        (self.passed if ok else self.failed).append(what)
        print(("  PASS " if ok else "  FAIL ") + what)
        return ok

    def soft(self, ok, what):
        (self.passed if ok else self.warned).append(what)
        print(("  PASS " if ok else "  WARN ") + what)
        return ok


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("exe")
    ap.add_argument("--plugins", help="plugins folder (default: a temporary one)")
    ap.add_argument("--install", action="store_true", help="build and install the plugin into the plugins folder first")
    a = ap.parse_args()

    work = Path(tempfile.mkdtemp(prefix="openmining-smoke-"))
    plugins = Path(a.plugins) if a.plugins else work / "plugins"
    env = dict(os.environ, OCS_PLUGINS_DIR=str(plugins), XDG_CONFIG_HOME=str(work / "config"), OPENMINING_LANG="es")
    if a.install or not a.plugins:
        subprocess.run([str(ROOT / "scripts/dev-install.sh")], check=True, env=env)
    data = work / "data"
    data.mkdir()
    for f in ("collar.csv", "survey.csv", "assays.csv"):
        shutil.copy(DATA / f, data / f)
    for f in ("logging.csv", "lab.csv"):
        shutil.copy(GT_DATA / f, data / ("gt_" + f))

    c = Checks()
    s = Server(a.exe, env)
    print("greeting:", s.greeting)
    try:
        # 1. Plugin loaded and panels registered.
        st = s.control("state")
        plugins_loaded = json.dumps(st.get("plugins", st.get("result", {}).get("plugins", [])))
        c.need("opencad.mining" in plugins_loaded, "plugin opencad.mining is loaded")
        panels_json = json.dumps(st.get("plugin_panels", st.get("result", {}).get("plugin_panels", [])))
        for p in PANELS:
            c.need(p in panels_json, f"panel {p} registered")
        s.send({"op": "new"})
        # Panel requests of the control protocol are addressed to a document.
        doc = s.send({"op": "query", "layer": "0"}).get("document_id")
        c.need(doc is not None, "document id readable after NEW")

        def field(panel, control, value):
            kw = {"value": value} if isinstance(value, (int, float)) else {"text": str(value)}
            r = s.control("panel_input", panel_id=panel, control_id=control, document_id=doc, **kw)
            return c.need(r.get("ok", True) is not False, f"panel {panel}: {control} = {value}")

        def choose(panel, control, index):
            r = s.control("panel_action", panel_id=panel, control_id=control, action="change", value={"index": index}, document_id=doc)
            return c.need(r.get("ok", True) is not False, f"panel {panel}: {control} -> option {index}")

        def run(cmd):
            return s.send({"op": "run", "cmd": cmd})

        def layer(name):
            return s.send({"op": "query", "layer": name})

        def result(op):
            hist = s.control("history", quiet=True)
            entries = hist.get("entries", hist.get("result", {}).get("entries", []))
            for e in reversed(entries):
                t = e.get("text", "")
                i = t.find(RESULT)
                if i >= 0:
                    r = json.loads(t[i + len(RESULT):])
                    if r.get("op") == op:
                        return r
            return {}

        def ok(op, what, cond=lambda d: True):
            r = result(op)
            good = bool(r.get("ok")) and cond(r.get("data") or {})
            c.need(good, f"{what} ({json.dumps(r.get('data') if r.get('ok') else r.get('error'))[:240]})")
            return r

        # 2. Geology through arguments: trace, then projection along the dip.
        run("MIN_GM_TRACE e=1000 n=2000 z=300 bearing=N90E length=20 anchor=start")
        r = ok("gm_trace", "gm_trace draws at E1000 N2000 Z300, azimuth 90", lambda d: abs(d.get("azimuth", 0) - 90) < 1e-9)
        th = r.get("created", [])
        if th:
            run(f"MIN_GM_PROJECT dip=45 dip_dir=right proj_mode=height proj_value=290 traces={th[0]}")
            ok("gm_project", "gm_project projects 1 trace 10 m down at 45 degrees", lambda d: d.get("projected") == 1)
            c.need(count(layer("MIN_GM_PROYECCION")) >= 1, "the projection is on MIN_GM_PROYECCION")
        else:
            c.need(False, "gm_trace reports the created handle")
        # A value typed in a generated panel reaches the next run without arguments.
        field("min_geomap", "gm.length", 25)
        run("MIN_GM_TRACE")
        ok("gm_trace", "a panel value (gm.length = 25) is used by the next run", lambda d: abs(d.get("length", 0) - 25) < 1e-9)

        # 3. Workings and sections with handles instead of clicks.
        run("3DPOLY 0,0,0 0,50,0 0,80,10")
        polys = s.send({"op": "query", "type": "Polyline3D", "layer": "0"})
        ph = handles(polys) or handles(s.send({"op": "query", "type": "Polyline3D"}))
        if ph:
            run(f"MIN_UG_AXIS axis={ph[-1]} profile=rectangular width=4 height=3.5 interval=2")
            r = ok("ug_axis", "ug_axis builds a solid on the given axis", lambda d: d.get("volume", 0) > 0)
            sh = [h for h in r.get("created", [])]
            solid = handles(layer("MIN_UG_SOLIDO"))
            if solid:
                run(f"MIN_SC_RUN mode=levels z_from=1 z_to=9 z_step=4 draw_sheets=false solids={solid[0]}")
                ok("sc_run", "sc_run cuts the solid with 3 level planes", lambda d: d.get("planes") == 3 and d.get("cut", 0) >= 1)
            else:
                c.need(False, "the solid is on MIN_UG_SOLIDO")
        else:
            c.need(False, "3DPOLY created a polyline")

        # 4. Drillholes (Spanish Excel collar: ';' + decimal comma), paths as arguments.
        run(f'MIN_DH_LOAD collar="{data / "collar.csv"}" survey="{data / "survey.csv"}" assays="{data / "assays.csv"}"')
        ok("dh_load", "dh_load reads the 3 reference holes", lambda d: d.get("holes") == 3 and "CU%" in d.get("assay_attributes", []))
        run("MIN_DH_DRAW assay_attr=CU% scheme=rainbow")
        ok("dh_draw", "dh_draw draws them coloured by CU%", lambda d: d.get("entities", 0) > 0)
        c.need(count(layer("MIN_DH_TRAZA")) >= 3, "3 traces on MIN_DH_TRAZA")

        # 5. Resources: composites -> model saved for the domain -> estimate -> view.
        run("MIN_RS_COMPOSITE comp_length=10")
        ok("rs_composite", "rs_composite makes composites of CU%", lambda d: d.get("composites", 0) > 0)
        run("MIN_RS_MODELSAVE")
        ok("rs_modelsave", "rs_modelsave stores the model of the domain")
        run("MIN_RS_ESTIMATE size_x=10 size_y=10 size_z=10 radius1=300 radius2=300 radius3=300")
        ok("rs_estimate", "rs_estimate estimates blocks", lambda d: d.get("estimated", 0) > 0)
        run("MIN_RS_BMVIEW mode=levels attribute=grade_ok")
        ok("rs_bmview", "rs_bmview draws block polygons", lambda d: d.get("polygons", 0) > 0)
        run("MIN_RS_VARIO domain=NoExiste")
        c.need(result("rs_vario").get("error", {}).get("code") == "not_found", "an unknown domain name is refused with not_found")

        # 6. Geotechnics through the operations format an AI client uses:
        #    key=value arguments and the OPENMINING_RESULT line of each command.
        run(f'MIN_GT_LOAD logging="{data / "gt_logging.csv"}" lab="{data / "gt_lab.csv"}"')
        r = result("gt_load")
        c.need(r.get("ok") and r["data"]["tables"]["logging"]["records"] == 5 and r["data"]["lab_lithologies"] == 2,
               f"gt_load result: 5 logged intervals, 2 lithologies ({r.get('data') or r.get('error')})")
        run("MIN_GT_CLASSIFY color_by=q_class")
        r = result("gt_classify")
        d = r.get("data", {})
        c.need(r.get("ok") and d.get("with_rmr") == 5 and d.get("classes") == [1, 2, 0, 2, 0] and d.get("drawn") == 5,
               f"gt_classify result: RMR for 5, classes I1 II2 IV2, 5 drawn ({d or r.get('error')})")
        c.need(len(r.get("created", [])) == 5, "gt_classify reports the 5 created handles")
        c.need(count(layer("MIN_GT_SONDAJE")) >= 5, "the 5 intervals are on MIN_GT_SONDAJE")
        run("MIN_GT_HB sigci=100 gsi=50 mi=10 d=0")
        r = result("gt_hb")
        c.need(r.get("ok") and abs(r["data"]["mb"] - 1.6767) < 1e-3 and abs(r["data"]["a"] - 0.5057) < 1e-3,
               f"gt_hb result: mb 1.677, a 0.506 ({r.get('data') or r.get('error')})")
        run("MIN_GT_HB gsi=150")
        r = result("gt_hb")
        c.need(r.get("ok") is False and r["error"]["code"] == "out_of_range" and r["error"]["param"] == "gsi",
               "an out-of-range argument is refused with code out_of_range")
        run("MIN_GT_HB nope=1")
        c.need(result("gt_hb").get("error", {}).get("code") == "unknown_argument", "an unknown argument is refused")
        run("MIN_GT_ESTIMATE method=idw radius1=80 radius2=80 radius3=40")
        r = result("gt_estimate")
        c.need(r.get("ok") and r["data"]["estimated"] > 0 and "gt_rmr" in r["data"]["columns"],
               f"gt_estimate result fills gt_rmr ({r.get('data') or r.get('error')})")
        run("MIN_GT_VIEW attribute=class")
        c.need(result("gt_view").get("data", {}).get("column") == "gt_class" and count(layer("MIN_RS_VISTA")) >= 1,
               "gt_view draws the model coloured by rock mass class")
    finally:
        hist = s.control("history", quiet=True)
        entries = hist.get("entries", hist.get("result", {}).get("entries", []))
        print("\n--- host command line ---")
        for e in entries[-200:]:
            print(f"  [{e.get('kind')}] {e.get('text')}")
        s.close()

    print(f"\nRESULT: {'OK' if not c.failed else 'FAILED'} ({len(c.passed)} passed, {len(c.warned)} warnings, {len(c.failed)} failed)")
    for f in c.failed:
        print("  FAIL", f)
    for w in c.warned:
        print("  WARN", w)
    sys.exit(1 if c.failed else 0)


if __name__ == "__main__":
    main()
