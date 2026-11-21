"""Minimal MCP stdio client for the MJSFX helper."""
import json, os, subprocess
WS = os.environ.get("MJSFX_WORKSPACE", os.path.expanduser("~/Library/Containers/dev.mjs.app.mjsfx/Data/Documents/MJSFX"))
class MJ:
    def __init__(self):
        env = dict(os.environ, MJSFX_WORKSPACE=WS)
        self.p = subprocess.Popen(["/Applications/MJSFX.app/Contents/Helpers/mjsfx-mcp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, env=env, text=True)
        self.i = 0
        self.rpc("initialize", {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "sfx-script", "version": "1"}})
        self.send({"jsonrpc": "2.0", "method": "notifications/initialized"})
    def send(self, m):
        self.p.stdin.write(json.dumps(m) + "\n"); self.p.stdin.flush()
    def rpc(self, method, params):
        self.i += 1
        self.send({"jsonrpc": "2.0", "id": self.i, "method": method, "params": params})
        while True:
            r = json.loads(self.p.stdout.readline())
            if r.get("id") == self.i:
                if "error" in r: raise RuntimeError(r["error"])
                return r["result"]
    def call(self, _tool, **args):
        args = {k: (json.dumps(v) if isinstance(v, (dict, list)) else v) for k, v in args.items()}
        r = self.rpc("tools/call", {"name": _tool, "arguments": args})
        txt = "".join(c.get("text", "") for c in r.get("content", []))
        if r.get("isError"): raise RuntimeError(txt)
        try: return json.loads(txt)
        except Exception: return txt
    def resource(self, uri):
        r = self.rpc("resources/read", {"uri": uri})
        return "".join(c.get("text", "") for c in r["contents"])
    def resources(self):
        return self.rpc("resources/list", {})
