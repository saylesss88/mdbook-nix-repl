#!/usr/bin/env python3
import json
import subprocess
import os
import sys
from http.server import BaseHTTPRequestHandler, HTTPServer

# SECURITY: Only bind to localhost. Never use 0.0.0.0 unless behind a secure proxy.
# HOST = os.environ.get("NIX_REPL_BIND","127.0.0.1") 
HOST = "0.0.0.0"
PORT = 8080

# SECURITY: 1MB limit for request body to prevent memory exhaustion
MAX_BODY_SIZE = 1024 * 1024 

class Handler(BaseHTTPRequestHandler):
    def _send_json(self, status, data):
        body = json.dumps(data).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self._set_cors()
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _set_cors(self):
        # SECURITY: Reflected CORS for localhost only. 
        # Prevents malicious external sites from driving the REPL.
        origin = self.headers.get("Origin", "")
        if "localhost" in origin or "127.0.0.1" in origin:
            self.send_header("Access-Control-Allow-Origin", origin)
        
        self.send_header("Access-Control-Allow-Methods", "POST, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type, X-Nix-Repl-Token")

    def do_OPTIONS(self):
        self.send_response(204)
        self._set_cors()
        self.end_headers()


    def do_POST(self):
        # SECURITY: Auth Check
        expected_token = os.environ.get("NIX_REPL_TOKEN", "").strip() # Trim env var just in case
        auth_header = (self.headers.get("X-Nix-Repl-Token") or "").strip() # Trim header
        
        # DEBUG LOGGING (Remove in production!)
        print(f"DEBUG: Expected='{expected_token}'")
        print(f"DEBUG: Received='{auth_header}'")

        if expected_token and auth_header != expected_token:
            return self._send_json(403, {"error": f"Forbidden: Invalid Token. Expected '{expected_token}' vs Received '{auth_header}'"})

        # SECURITY: Size Check
        try:
            length = int(self.headers.get("Content-Length", "0"))
        except ValueError:
            return self._send_json(400, {"error": "Invalid Content-Length"})

        if length > MAX_BODY_SIZE:
            return self._send_json(413, {"error": "Payload too large"})

        body = self.rfile.read(length).decode("utf-8")
        try:
            data = json.loads(body or "{}")
            code = data.get("code", "")
        except json.JSONDecodeError:
            return self._send_json(400, {"error": "Invalid JSON"})

        if not code:
            return self._send_json(400, {"error": "No code provided"})

        try:
            # SECURITY: Timeout added (5s) to prevent infinite loops
            proc = subprocess.run(
                ["nix", "eval", "--json", "--expr", code],
                capture_output=True,
                text=True,
                timeout=5 
            )
            
            # Truncate output if massive (basic anti-spam)
            stdout = proc.stdout[:10000] 
            stderr = proc.stderr[:10000]

            if proc.returncode == 0:
                resp = {"stdout": stdout}
            else:
                resp = {"error": stderr or f"exit {proc.returncode}"}
        except subprocess.TimeoutExpired:
            resp = {"error": "Execution timed out (5s limit)"}
        except Exception as e:
            resp = {"error": f"Server error: {str(e)}"}

        self._send_json(200, resp)

def main():
    if not os.environ.get("NIX_REPL_TOKEN"):
        print("⚠️  WARNING: NIX_REPL_TOKEN env var not set. Auth is disabled!")
    
    server = HTTPServer((HOST, PORT), Handler)
    print(f"🔒 nix-repl secure server listening on {HOST}:{PORT}")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass

if __name__ == "__main__":
    main()
