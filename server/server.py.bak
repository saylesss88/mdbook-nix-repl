#!/usr/bin/env python3
import json
import subprocess
from http.server import BaseHTTPRequestHandler, HTTPServer

HOST = "0.0.0.0"
PORT = 8080

class Handler(BaseHTTPRequestHandler):
    def _set_headers(self, status=200, length=0):
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "POST, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type")
        self.send_header("Content-Length", str(length))
        self.end_headers()


    def do_OPTIONS(self):
        self.send_response(204)
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "POST, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type")
        self.end_headers()

    def do_POST(self):
        length = int(self.headers.get("Content-Length", "0"))
        body = self.rfile.read(length).decode("utf-8") if length > 0 else ""
        try:
            data = json.loads(body or "{}")
            code = data.get("code", "")
        except Exception as e:
            resp = {"error": f"Invalid JSON: {e}"}
            raw = json.dumps(resp).encode("utf-8")
            self._set_headers(400, len(raw))
            self.wfile.write(raw)
            return

        if not code:
            resp = {"error": "No code provided"}
            raw = json.dumps(resp).encode("utf-8")
            self._set_headers(400, len(raw))
            self.wfile.write(raw)
            return

        try:
            proc = subprocess.run(
                ["nix", "eval", "--json", "--expr", code],
                capture_output=True,
                text=True,
            )
            if proc.returncode == 0:
                resp = {"stdout": proc.stdout}
            else:
                resp = {"error": proc.stderr or f"exit {proc.returncode}"}
        except Exception as e:
            resp = {"error": f"Failed to run nix: {e}"}

        raw = json.dumps(resp).encode("utf-8")
        self._set_headers(200, len(raw))
        self.wfile.write(raw)

def main():
    server = HTTPServer((HOST, PORT), Handler)
    print(f"nix-repl HTTP server on {HOST}:{PORT}")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass

if __name__ == "__main__":
    main()

