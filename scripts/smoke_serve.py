#!/usr/bin/env python3
"""End-to-end smoke test of Open Mining inside a real Open CAD Studio.

    python3 scripts/smoke_serve.py /path/to/OpenCADStudio-or-AppRun [--plugins DIR] [--install]

Starts `OpenCADStudio --serve` (headless, JSON lines over stdin/stdout) with an
isolated configuration and plugins folder, then drives every module through
the same commands and panel fields a user would:

  plugin loaded + panels registered -> geology trace + projection along dip ->
  axis solid + sections -> drillholes load/draw -> composites -> variogram
  model -> block model estimate -> block view

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
PANELS = ["min_drillholes", "min_workings", "min_geomap", "min_axes", "min_sections", "min_rs_comp", "min_rs_vario", "min_rs_bm"]


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

        # 2. Geology: trace from coordinates, then projection along the dip.
        for ctl, v in (("gm_e", 1000), ("gm_n", 2000), ("gm_z", 300), ("gm_bearing", "N90E"), ("gm_length", 20)):
            field("min_geomap", ctl, v)
        run("MIN_GM_TRACE")
        trace = layer("MIN_GM_TRAZA")
        c.need(count(trace) >= 1, "MIN_GM_TRACE draws a trace")
        th = handles(trace)
        if th:
            field("min_geomap", "gm_dip", 45)
            field("min_geomap", "gm_pvalue", 290)
            s.send({"op": "select", "handles": th[:1]})
            run("MIN_GM_PROJECT")
            c.need(count(layer("MIN_GM_PROYECCION")) >= 1, "MIN_GM_PROJECT projects the trace to another height")
        else:
            c.need(False, "trace handle readable from query (reply format?)")

        # 3. Workings: solid along a 3D polyline, then level sections.
        run("3DPOLY 0,0,0 0,50,0 0,80,10")
        polys = s.send({"op": "query", "type": "Polyline3D", "layer": "0"})
        ph = handles(polys) or handles(s.send({"op": "query", "type": "Polyline3D"}))
        if ph:
            s.send({"op": "select", "handles": ph[-1:]})
            run("MIN_UG_AXIS")
            solid = layer("MIN_UG_SOLIDO")
            c.need(count(solid) >= 1, "MIN_UG_AXIS builds a solid on the selected axis")
            sh = handles(solid)
            if sh:
                choose("min_sections", "sc_mode", 2)
                s.send({"op": "select", "handles": sh[:1]})
                run("MIN_SC_RUN")
                c.soft(count(layer("MIN_SC_CORTE")) >= 1, "MIN_SC_RUN cuts the solid with level planes")
        else:
            c.need(False, "3DPOLY created a polyline")

        # 4. Drillholes from CSV (Spanish Excel collar: ';' + decimal comma).
        for ctl, f in (("dh_collar", "collar.csv"), ("dh_survey", "survey.csv"), ("dh_assays", "assays.csv")):
            field("min_drillholes", ctl, str(data / f))
        run("MIN_DH_LOAD")
        run("MIN_DH_DRAW")
        c.need(count(layer("MIN_DH_TRAZA")) >= 3, "MIN_DH_LOAD + MIN_DH_DRAW draw the 3 reference holes")

        # 5. Resources: composites -> model saved for the domain -> estimate -> view.
        run("MIN_RS_COMPOSITE")
        run("MIN_RS_MODELSAVE")
        run("MIN_RS_ESTIMATE")
        run("MIN_RS_BMVIEW")
        c.need(count(layer("MIN_RS_VISTA")) >= 1, "composite + estimate + view draw block polygons")
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
