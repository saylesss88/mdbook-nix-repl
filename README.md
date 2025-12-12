# mdbook-nix-repl

Interactive Nix REPL–style code blocks for mdBook.

![mdbook-repl](https://raw.githubusercontent.com/saylesss88/mdbook-nix-repl/main/assets/mdbook-nix-repl1.png)

This preprocessor lets you write fenced blocks like:

````
```nix repl
1 + 1
```
````

In the rendered book you get a “Run” button that sends the code to a Nix
evaluation service and shows the result inline.

---

## Quick start (local)

1. `cargo install mdbook-nix-repl`

2. Configure `book.toml` and `index.hbs` as below

3. Run the container backend (optional but recommended)

4. `mdbook serve` and click “Run” on a ` ```nix-repl``` ` code block.

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

 {{!-- mdbook-nix-repl backend endpoint --}}
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

More of the file is shown above for context, to be clear, all you need to add to
`index.hbs` is:

```xml
{{!-- mdbook-nix-repl backend endpoint --}}
<script>
  window.NIX_REPL_ENDPOINT = "http://127.0.0.1:8080/";
</script>

```

---

## Containerized backend (Podman/Docker)

For a more isolated setup, the Nix eval server can run inside a container
instead of directly on the host. This works well on immutable hosts like Fedora
SecureBlue and Silverblue, and keeps Nix eval isolated inside the container.​

**Image contents**

The container:

- Is based on `debian:stable-slim`.

- Installs `curl`, `ca-certificates`, `python3`, and `xz-utils`.

- Creates a non-root user `nixuser` with a writable `/nix` directory.

- Installs Nix in single-user mode for `nixuser`.

- Enables `nix-command` and flakes via `~/.config/nix/nix.conf`.

- Copies `server.py` into `/app/server.py` and runs it on port 8080.

1. Clone [mdbook-nix-repl](https://github.com/saylesss88/mdbook-nix-repl) and
   move to it.

```bash
git clone https://github.com/saylesss88/mdbook-nix-repl.git

cd mdbook-nix-repl/server
```

2. Make `server.py` executable (see below to inspect the file):

```bash
chmod + x server.py
```

3. `podman build -t nix-repl-service .`

4. `podman run --rm -p 127.0.0.1:8080:8080 nix-repl-service`

Then configure `window.NIX_REPL_ENDPOINT = "http://127.0.0.1:8080/";` in your
`index.hbs` as shown above.

If the above server is running, `mdbook-nix-repl` will work on a machine without
nix or NixOS installed, either by running `mdbook serve` or `mdbook build`.

Just find the `nix-repl` code block and click `Run`.

See the following to inspect the `Dockerfile` and `server.py`:

<details>
<summary> ✔️ Dockerfile </summary>

```text
FROM debian:stable-slim

RUN apt-get update && \
    apt-get install -y curl ca-certificates python3 xz-utils && \
    rm -rf /var/lib/apt/lists/*

# Create the user and the /nix directory as root
RUN useradd -m nixuser
RUN mkdir -m 0755 /nix && chown nixuser /nix

USER nixuser
ENV USER=nixuser
ENV NIX_INSTALLER_NO_MODIFY_PROFILE=1

RUN curl -L https://nixos.org/nix/install -o /tmp/install-nix.sh \
 && sh /tmp/install-nix.sh --no-daemon \
 && rm /tmp/install-nix.sh

# Enable nix-command and flakes for this user
RUN mkdir -p /home/nixuser/.config/nix && \
    echo 'experimental-features = nix-command flakes' > /home/nixuser/.config/nix/nix.conf

ENV NIX_PATH=/home/nixuser/.nix-profile/etc/nix
ENV PATH=/home/nixuser/.nix-profile/bin:$PATH

WORKDIR /app
COPY server.py /app/server.py

EXPOSE 8080
CMD ["python3", "/app/server.py"]
```

</details>

<details>
<summary> ✔️ server.py </summary>

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

</details>

---

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

Point `window.NIX_REPL_ENDPOINT` (i.e., the small snippet you add to
`index.hbs`) at that machine’s HTTP endpoint.

> Security note: this server executes arbitrary Nix expressions. It is intended
> for local development on trusted machines. Do not expose it to untrusted
> networks without additional sandboxing, authentication, and resource limits.

Running the backend inside a rootless Podman or Docker container on localhost
improves isolation compared to a bare Python server, but it should still not be
exposed to untrusted networks without additional sandboxing, authentication, and
resource limits.

---

## Status

This is experimental. The UI and protocol may change. Contributions for:

- Safer backends (e.g. Nix sandboxes, systemd services).

- Containerized backends (Podman/Docker).

- Better frontend UX.

are very welcome.
