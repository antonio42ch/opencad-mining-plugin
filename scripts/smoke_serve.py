#!/usr/bin/env python3
"""Smoke test of Open Mining inside a real Open CAD Studio, headless.

    python3 scripts/smoke_serve.py /path/to/OpenCADStudio [--plugins DIR]

Starts `OpenCADStudio --serve` with an isolated config folder, installs nothing
itself (run scripts/dev-install.sh with OCS_PLUGINS_DIR=DIR first, or let this
script do it with --install), then drives the plugin's commands over the JSON
line protocol and checks that entities appear on the plugin layers.
"""
import argparse, json, os, subprocess, sys, tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


class Server:
    def __init__(self, exe, env):
        self.p = subprocess.Popen([exe, "--serve"], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                  stderr=sys.stderr, text=True, env=env)
        self.greeting = json.loads(self.p.stdout.readline())

    def send(self, payload):
        self.p.stdin.write(json.dumps(payload) + "\n")
        self.p.stdin.flush()
        line = self.p.stdout.readline()
        if not line:
            raise RuntimeError("server closed the connection")
        reply = json.loads(line)
        print(">>", json.dumps(payload))
        print("<<", json.dumps(reply)[:600])
        return reply

    def close(self):
        self.p.stdin.close()
        self.p.wait(timeout=30)


def count(reply):
    for k in ("count", "total"):
        if isinstance(reply.get(k), int):
            return reply[k]
    for k in ("entities", "results", "items"):
        if isinstance(reply.get(k), list):
            return len(reply[k])
    return -1


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("exe")
    ap.add_argument("--plugins", help="plugins folder (default: a temp folder, filled with --install)")
    ap.add_argument("--install", action="store_true", help="run scripts/dev-install.sh into the plugins folder")
    a = ap.parse_args()

    work = Path(tempfile.mkdtemp(prefix="openmining-smoke-"))
    plugins = Path(a.plugins) if a.plugins else work / "plugins"
    env = dict(os.environ, OCS_PLUGINS_DIR=str(plugins), XDG_CONFIG_HOME=str(work / "config"), OPENMINING_LANG="es")
    if a.install or not a.plugins:
        subprocess.run([str(ROOT / "scripts/dev-install.sh")], check=True, env=env)

    s = Server(a.exe, env)
    print("greeting:", s.greeting)
    failures = []
    try:
        s.send({"op": "new"})

        # Geology: interactive point fed as a token -> follow-up MIN_GM_RUN.
        r = s.send({"op": "run", "cmd": "MIN_GM_TRACE 10,20,0"})
        q = s.send({"op": "query", "layer": "MIN_GM_*"})
        if count(q) < 1:
            failures.append("MIN_GM_TRACE created nothing on MIN_GM_* layers")

        # Workings: draw an axis, then pick it by handle.
        s.send({"op": "run", "cmd": "3DPOLY 0,0,0 0,50,0 0,80,10"})
        polys = s.send({"op": "query", "type": "Polyline3D"})
        handle = next((e.get("handle") for e in polys.get("entities", []) if isinstance(e, dict)), None)
        if handle:
            s.send({"op": "run", "cmd": f"MIN_UG_AXIS {handle}"})
            q = s.send({"op": "query", "layer": "MIN_UG_*"})
            if count(q) < 1:
                failures.append("MIN_UG_AXIS created no solid")
        else:
            failures.append("could not read the axis handle (query reply format?)")

        # Persistence: the settings xrecord is written by the commands above.
        s.send({"op": "records"})
    finally:
        s.close()

    print("\nRESULT:", "OK" if not failures else "FAILED")
    for f in failures:
        print(" -", f)
    print("Drillholes need file paths in the panel: test them in the GUI (see docs/TESTING.md).")
    sys.exit(1 if failures else 0)


if __name__ == "__main__":
    main()
