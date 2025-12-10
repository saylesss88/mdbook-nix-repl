# mdbook-nix-repl

Interactive Nix REPL–style code blocks for mdBook.

This preprocessor lets you write fenced blocks like:

````
```nix repl
1 + 1
```
````

In the rendered book you get a “Run” button that sends the code to a Nix
evaluation service and shows the result inline.

---

## Installation

Add `mdbook-nix-repl` to your toolchain, for example with Cargo:

```bash
cargo install mdbook-nix-repl
```

Then enable it in your `book.toml`:

```toml
[preprocessor.nix-repl]
command = "mdbook-nix-repl"
```

---

## Usage in Markdown

Use fenced blocks tagged as `nix repl`:

```nix repl
2 + 2
```

4

```nix repl
"goodbye ${ { d = "world";}.d}"
```

"goodbye world"

The preprocessor rewrites these into interactive blocks with a “Run” button and
an output area.

---

## Frontend integration

The preprocessor only generates HTML; it does not talk to Nix directly. A small
JS helper sends the code to an HTTP endpoint and displays the result.

In your `book.toml`:

```toml
[output.html]
theme = "theme"
additional-js = ["theme/nix_http.js"]
```

Example `theme/nix_http.js`:

```js
document.addEventListener("DOMContentLoaded", () => {
  const endpoint = window.NIX_REPL_ENDPOINT || "https://your-nix-eval/eval";

  document.querySelectorAll(".nix-repl-block").forEach((block) => {
    const btn = block.querySelector(".nix-repl-run");
    const codeEl = block.querySelector("code");
    const out = block.querySelector(".nix-repl-output");
    const status = block.querySelector(".nix-repl-status");

    if (!btn || !codeEl || !out) return;

    btn.addEventListener("click", async () => {
      const code = codeEl.textContent;
      block.classList.add("running");
      status.textContent = "Running…";
      out.textContent = "";

      try {
        const res = await fetch(endpoint, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ code }),
        });

        console.log("nix-repl response status", res.status);
        const raw = await res.text();
        console.log("nix-repl raw response", raw);

        let data;
        try {
          data = JSON.parse(raw);
        } catch (e) {
          block.classList.add("error");
          out.textContent = "Non-JSON response: " + raw;
          status.textContent = "Error";
          return;
        }

        if (data.error) {
          block.classList.add("error");
          out.textContent = data.error;
          status.textContent = "Error";
        } else {
          block.classList.remove("error");
          out.textContent = data.stdout || "";
          status.textContent = "Done";
        }
      } catch (e) {
        block.classList.add("error");
        out.textContent = String(e);
        status.textContent = "Network error";
      } finally {
        block.classList.remove("running");
      }
    });
  });
});
```

In your `theme/index.hbs`, inject the endpoint and include the extra JS:

```xml
    {{!-- ...rest of template... --}}

    <script>
      window.NIX_REPL_ENDPOINT = "http://127.0.0.1:8080/";
    </script>

    {{#if js}}
      {{#each js}}
        <script src="{{ ../path_to_root }}{{ this }}"></script>
      {{/each}}
    {{/if}}
  </body>
</html>
```

---

## Example backend: local Nix eval server (Python)

This project does not ship a mandatory backend; you can plug in any HTTP service
that matches the simple JSON protocol:

- Request: POST NIX_REPL_ENDPOINT with JSON body:

```json
{ "code": "1 + 1" }
```

- Response on success:

```json
{ "stdout": "2\n" }
```

- Response on error:

```json
{ "error": "some error message" }
```

For local development, you can run a tiny Python server on a NixOS machine (or
any system with nix and Python 3 installed):

`server.py`:

```py
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
```

Run the example backend on a machine with nix and Python 3 installed (NixOS or
any system with the Nix package manager).

I personally tested in a NixOS VM, clone your book inside the VM, run the server
in the VM, run `mdbook serve` in the VM, go to `https://localhost:3000` find
your `nix repl` code block, and click `Run`.

Point `window.NIX_REPL_ENDPOINT` (i.e., the small snippet you add to
`index.hbs`) at that machine’s HTTP endpoint.

Run it:

```bash
chmod +x server.py
python3 server.py
```

Then in another terminal:

```bash
mdbook serve
```

Visit your book, press "Run" on a `nix repl`

> Security note: this server executes arbitrary Nix expressions. It is intended
> for local development on trusted machines. Do not expose it to untrusted
> networks without additional sandboxing, authentication, and resource limits.

---

## Status

This is experimental. The UI and protocol may change. Contributions for:

- Safer backends (e.g. Nix sandboxes, systemd services).

- Containerized backends (Podman/Docker).

- Better frontend UX.

are very welcome.
