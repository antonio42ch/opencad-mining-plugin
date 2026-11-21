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
PANELS = ["min_drillholes", "min_workings", "min_geomap", "min_axes", "min_sections", "min_rs_comp", "min_rs_vario", "min_rs_bm", "min_gt_data", "min_gt_hb", "min_gt_model", "min_rm_struct", "min_rm_design", "min_channels", "min_vein", "min_vein_model"]


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
    for f in ("ch_points.csv", "ch_sheet.csv"):
        shutil.copy(DATA / f, data / f)
    for f in ("logging.csv", "lab.csv", "joints.csv"):
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

        # 1a. The operations describe themselves (no guide needed).
        run("MIN_HELP")
        r = ok("help", "help lists the operations", lambda d: len(d.get("operations", [])) > 50 and "geotechnics" in d.get("groups", []))
        run("MIN_HELP op=gt_view")
        ok("help", "help op=gt_view gives its parameters and options", lambda d: any(p["key"] == "mode" and "cubes" in (p["options"] or []) for p in d.get("parameters", [])))
        run("MIN_HELP op=nada")
        c.need(result("help").get("error", {}).get("code") == "not_found", "help of an unknown operation is not_found")

        # 1b. Status of an empty project points at loading drillholes.
        run("MIN_STATUS")
        ok("status", "status of an empty project suggests dh_load", lambda d: (d.get("next") or [{}])[0].get("op") == "dh_load")

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
                run("MIN_ENTITIES class=UG_SOLID")
                ok("entities", "entities lists the solid with its volume", lambda d: d.get("total", 0) >= 1 and "volume" in d["entities"][0]["attributes"])
            else:
                c.need(False, "the solid is on MIN_UG_SOLIDO")
        else:
            c.need(False, "3DPOLY created a polyline")

        # 4. Drillholes (Spanish Excel collar: ';' + decimal comma), paths as arguments.
        run(f'MIN_DH_LOAD collar="{data / "collar.csv"}" survey="{data / "survey.csv"}" assays="{data / "assays.csv"}"')
        ok("dh_load", "dh_load reads the 3 reference holes", lambda d: d.get("holes") == 3 and "cu%" in d.get("assay_attributes", []))
        run("MIN_DH_DRAW assay_attr=CU% scheme=rainbow")
        ok("dh_draw", "dh_draw draws them coloured by CU%", lambda d: d.get("entities", 0) > 0)
        c.need(count(layer("MIN_DH_TRAZA")) >= 3, "3 traces on MIN_DH_TRAZA")
        run("MIN_DH_LIST")
        ok("dh_list", "dh_list lists the 3 holes", lambda d: d.get("total") == 3)
        run("MIN_DH_LIST hole=VERT")
        ok("dh_list", "dh_list hole=VERT returns its assay intervals", lambda d: len(d.get("assays", [])) >= 3)

        # 5. Resources: composites -> model saved for the domain -> estimate -> view.
        run("MIN_RS_COMPOSITE comp_length=10")
        ok("rs_composite", "rs_composite makes composites of CU%", lambda d: d.get("composites", 0) > 0)
        run("MIN_RS_MODELSAVE")
        ok("rs_modelsave", "rs_modelsave stores the model of the domain")
        run("MIN_RS_ESTIMATE size_x=10 size_y=10 size_z=10 radius1=300 radius2=300 radius3=300")
        ok("rs_estimate", "rs_estimate estimates blocks", lambda d: d.get("estimated", 0) > 0)
        run("MIN_RS_BMVIEW mode=levels attribute=grade_ok")
        ok("rs_bmview", "rs_bmview draws block polygons", lambda d: d.get("polygons", 0) > 0)
        run("MIN_RS_STATS")
        ok("rs_stats", "rs_stats reports the composites by domain", lambda d: d["domains"][0]["count"] > 0)
        run("MIN_BM_INFO")
        ok("bm_info", "bm_info reports blocks and estimated columns", lambda d: d.get("blocks", 0) > 0 and any(k.endswith("_ok") or k.endswith("_nn") for k in d.get("columns", {})))
        run("MIN_BM_SAMPLE point=500000,8500000,3985")
        ok("bm_sample", "bm_sample finds the block under VERT", lambda d: d["samples"][0]["block"] is not None)
        run("MIN_BM_RESOURCES cutoffs=0,1")
        ok("bm_resources", "bm_resources reports two cut-offs", lambda d: len(d.get("table", [])) == 2)
        run("MIN_STATUS")
        ok("status", "status after estimating knows the block model", lambda d: d.get("block_model") is not None and d["drillholes"]["holes"] == 3)
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
        run("MIN_GT_VIEW mode=cubes attribute=rmr limit=1000")
        r = ok("gt_view", "gt_view mode=cubes draws one cube mesh per colour", lambda d: d.get("mode") == "cubes" and d.get("cubes") and d["cubes"][0] == d["cubes"][1] > 0)
        c.need(count(layer("MIN_RS_VISTA")) >= 1, "the cube meshes are on MIN_RS_VISTA")
        run("MIN_BM_INFO")
        ok("bm_info", "the model has the confidence columns gt_dist and gt_n", lambda d: "gt_dist" in d.get("columns", {}) and "gt_n" in d.get("columns", {}))
        run("MIN_GT_VIEW mode=cubes attribute=dist")
        ok("gt_view", "gt_view attribute=dist colours by the distance to the records", lambda d: d.get("column") == "gt_dist" and d.get("cubes", [0, 0])[0] > 0)
        run("MIN_GT_VIEW mode=cubes attribute=rmr show=above threshold=1000")
        r = result("gt_view")
        c.need(r.get("ok") is False and r["error"]["code"] == "no_data", "a filter that leaves no block is reported as no_data")
        run("MIN_GT_VIEW mode=cubes attribute=rmr show=all cut=true cut_dip=0 cut_z=-100000 cut_side=front")
        ok("gt_view", "a horizontal cut below the model keeps every block", lambda d: d.get("cubes", [0, 0])[0] > 0)
        run("MIN_GT_VIEW mode=cubes attribute=rmr cut=true cut_dip=0 cut_z=100000 cut_side=front")
        r = result("gt_view")
        c.need(r.get("ok") is False and r["error"]["code"] == "no_data", "a cut above the model leaves nothing")
        run("MIN_GT_VIEW mode=levels attribute=class show=all cut=false")
        ok("gt_view", "gt_view mode=levels still draws plans", lambda d: d.get("mode") == "levels" and d.get("polygons", 0) > 0)
        run("MIN_GT_SUMMARY")
        ok("gt_summary", "gt_summary repeats the classification", lambda d: d.get("with_rmr") == 5)

        # 7. Structures: discontinuities -> stereonet, sets, kinematics, wedges.
        run(f'MIN_GT_LOAD joints="{data / "gt_joints.csv"}"')
        ok("gt_load", "gt_load reads 135 discontinuities", lambda d: (d["tables"]["joints"] or {}).get("records") == 135)
        run("MIN_ST_STEREONET contours=true")
        ok("st_stereonet", "st_stereonet plots the 135 poles of the table", lambda d: d.get("from_table") == 135 and d.get("max_density", 0) > 5)
        run("MIN_ST_SETS n_sets=0 min_angle=20 max_angle=30")
        ok("st_sets", "st_sets finds the 3 sets", lambda d: len(d.get("sets", [])) == 3)
        run("MIN_ST_KINEMATIC face_dip=70 face_dipdir=40 friction=30 lateral=20")
        ok("st_kinematic", "st_kinematic reports planar, wedge and toppling", lambda d: "planar" in d and "wedge" in d and "flexural" in d)
        run("MIN_ST_WEDGES set_a=1 set_b=2 set_c=3 axis_trend=0 axis_plunge=0 width=4 height=4 at=0,200,0")
        ok("st_wedges", "st_wedges finds and draws wedges with FS", lambda d: len(d.get("wedges", [])) >= 1 and d.get("drawn", 0) >= 1)
        c.need(count(layer("MIN_ST_CUNAS")) >= 1, "the wedges are on MIN_ST_CUNAS")
        run("MIN_ST_DRAW at=0,-300,0 radius=50")
        ok("st_draw", "st_draw draws the stereonet in the DWG", lambda d: d.get("entities", 0) > 100)

        # 8. Stresses and design.
        run("MIN_SM_STRESS depth=500 gamma=27 k_h=1.5 k_hmin=1 az_h=90 axis_trend=0")
        ok("sm_stress", "sm_stress: sigma_v 13.5, sigma_H 20.25 across a N-S working", lambda d: abs(d["sigma_v"] - 13.5) < 1e-6 and abs(d["section"]["sigma_1"] - 20.25) < 1e-6)
        run("MIN_SM_KIRSCH opening=circle radius=2.5")
        ok("sm_kirsch", "sm_kirsch: boundary stress 3 x 20.25 - 13.5 = 47.25", lambda d: abs(d["sigma_t_max"] - 47.25) < 1e-6)
        run("MIN_SM_CONVERGENCE shotcrete=true sc_thickness=0.1 install=2")
        ok("sm_convergence", "sm_convergence gives the ground curve and the support", lambda d: d.get("u_max", 0) > 0 and d.get("support_p_max") is not None)
        run("MIN_SM_PILLAR shape=square width=8 height=5 room=6 formula=lunder_pakalnis")
        ok("sm_pillar", "sm_pillar reports load, strength and FS", lambda d: d.get("fs", 0) > 0 and len(d.get("all", {})) == 4)
        run("MIN_SM_SPAN span=10 strike_length=100 thickness=20 gravity=2.7 dip=70 critical=5")
        ok("sm_span", "sm_span computes Carter's scaled span", lambda d: d.get("scaled_span", 0) > 0 and d.get("ratio") is not None)
        run("MIN_SM_DRAW chart=convergence at=200,0,0 size=100")
        ok("sm_draw", "sm_draw draws the convergence chart", lambda d: d.get("entities", 0) >= 3)
        # 9. Channel samples converted into drillholes and appended to them.
        run(f'MIN_CH_CONVERT points="{data / "ch_points.csv"}" sheet="{data / "ch_sheet.csv"}"')
        ok("ch_convert", "ch_convert turns 3 channels (5 samples) into holes added to the 3 drillholes", lambda d: d.get("channels") == 3 and d.get("samples") == 5 and d.get("holes") == 6 and d.get("issues") == [])
        run("MIN_DH_LIST hole=CH-C1")
        ok("dh_list", "the channel CH-C1 is a hole with its 2 samples, potencia and grades", lambda d: len(d.get("assays", [])) == 2 and d["assays"][1]["values"].get("au") == 6.2 and d["assays"][1]["values"].get("potencia") == 0.6)
        run("MIN_DH_DRAW assay_attr=au")
        ok("dh_draw", "dh_draw draws the 6 holes (drillholes + channels)", lambda d: d.get("holes") == 6)
        run("MIN_CH_ORIENT channel=C2 p1=0,0,0 p2=0,10,0")
        ok("ch_orient", "ch_orient sets channel C2 to azimuth 0 and converts again", lambda d: abs(d.get("azimuth", -1)) < 1e-9 and d["converted"]["channels"] == 3)
        # 10. Vein surfaces: hanging wall and footwall contacts mapped as 3D polylines
        #     (a vertical vein striking north, 1.2 m thick, around the channels).
        org = (result("dh_load").get("data") or {}).get("origin") or [0, 0, 0]
        def poly(pts):
            before = set(handles(s.send({"op": "query", "type": "Polyline3D"})))
            run("3DPOLY " + " ".join(f"{x - org[0]},{y - org[1]},{z - org[2]}" for x, y, z in pts))
            after = handles(s.send({"op": "query", "type": "Polyline3D"}))
            return [h for h in after if h not in before]
        hw, fw = [], []
        for z in (3945, 3955):
            hw += poly([(500000.6, 8500040, z), (500000.6, 8500060, z), (500000.6, 8500080, z)])
            fw += poly([(499999.4, 8500040, z), (499999.4, 8500060, z), (499999.4, 8500080, z)])
        c.need(len(hw) == 2 and len(fw) == 2, f"4 contact polylines drawn ({hw} {fw})")
        run(f"MIN_VN_SURFACES vein=V1 hw={','.join(hw)} fw={','.join(fw)} hw_layer= fw_layer= intercepts=false step=1 radius=30 densify=2 method=idw")
        ok("vn_surfaces", "vn_surfaces: vertical plane, ~1.2 m thick, no crossings", lambda d: abs(d["plane"]["dip"] - 90) < 5 and abs(d["diagnostics"]["thickness_mean"] - 1.2) < 0.3 and d["diagnostics"]["crossings"] == 0 and d["contacts"]["from_handles"] > 0)
        c.need(count(layer("MIN_VN_TECHO")) >= 1 and count(layer("MIN_VN_PISO")) >= 1, "both vein surfaces are drawn")
        run("MIN_VN_SURFACES vein=V1 hw_layer=NO_EXISTE fw_layer=NO_EXISTE intercepts=false")
        c.need(result("vn_surfaces").get("error", {}).get("code") == "no_data", "no contacts is reported as no_data")

        # 11. Vein prism model from the three channels (vein intercepts tipo=VETA, au).
        run("MIN_VN_VARIOGRAM vein=V1 grade=au thickness_col=potencia lag=5 nlags=4")
        ok("vn_variogram", "vn_variogram composites the 3 channel intercepts and fits a model", lambda d: d["composites"] == 3 and d["model"]["range_strike"] > 0)
        run("MIN_VN_ESTIMATE vein=V1 nugget=0 sill=1 range_strike=30 range_dip=30 block_u=5 block_v=5 max_distance=0 search_radius=60 min_samples=2 draw_prisms=true")
        ok("vn_estimate", "vn_estimate builds prisms with grades inside the channel range", lambda d: d["summary"]["prisms"] > 0 and 3.0 <= d["summary"]["grade"] <= 9.5 and d["validation"]["n"] == 3)
        prisms = result("vn_estimate").get("data", {}).get("summary", {}).get("prisms", 0)
        c.need(count(layer("MIN_VN_PRISMAS")) == prisms, f"the {prisms} prisms are drawn")
        run("MIN_VN_LONG vein=V1 at=1000,1000,0")
        ok("vn_long", "vn_long draws the long section at the base point", lambda d: d["at"][0] == 1000 and d["width"] > 0 and d["height"] > 0)
        lg = result("vn_long").get("data", {})
        c.need(count(layer("MIN_VN_LONG")) > prisms, "long section entities drawn")
        w2, ht = lg.get("width", 0) / 2, lg.get("height", 0)
        before = set(handles(s.send({"op": "query", "type": "Polyline3D"})))
        run(f"3DPOLY 999,999,0 {1000 + w2},999,0 {1000 + w2},{1001 + ht},0 999,{1001 + ht},0 999,999,0")
        poly_h = [h for h in handles(s.send({"op": "query", "type": "Polyline3D"})) if h not in before]
        run(f"MIN_VN_EDIT action=exclude polygon={poly_h[0] if poly_h else '0'}")
        ok("vn_edit", "vn_edit excludes the left half of the long section and estimates again", lambda d: len(d["edits"]) == 1 and d["estimate"]["summary"]["out_edits"] > 0 and d["estimate"]["summary"]["prisms"] < prisms)
        run("MIN_VN_EDIT action=clear")
        ok("vn_edit", "vn_edit clear restores the model", lambda d: len(d["edits"]) == 0 and d["estimate"]["summary"]["prisms"] == prisms)
        run("MIN_VN_ESTIMATE vein=NOPE")
        c.need(result("vn_estimate").get("error", {}).get("code") == "no_data", "a vein without surfaces is reported as no_data")
        run("MIN_VN_ESTIMATE vein=V1")

        run("MIN_CH_CONVERT sheet=nothing.csv")
        c.need(result("ch_convert").get("error", {}).get("code") == "not_found", "a missing sheet is reported as not_found")

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
