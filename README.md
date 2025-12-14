# mdbook-nix-repl

Interactive Nix REPL–style code blocks for mdBook.

![mdbook-repl](https://raw.githubusercontent.com/saylesss88/mdbook-nix-repl/main/assets/mdbook-nix-repl1.png)

This preprocessor lets you write fenced blocks like:

````
```nix repl
1 + 1
```
````

In the rendered book you get a “Run” button that sends the code to a secure
local Nix evaluation service and shows the result inline.

---

## Quick Start

1.  **Install the tool:**

    ```bash
    cargo install mdbook-nix-repl
    ```

2.  **Initialize your book:** Go to your mdBook directory and run:

    ```bash
    # Run this once if you don't already have a `theme/index.hbs` (required for injection)
    mdbook theme

    # Initialize the plugin files and backend
    mdbook-nix-repl init --auto
    ```

    _This command automatically creates the `theme/` files, generates a unique
    authentication token, injects the necessary scripts into `index.hbs`, and
    creates the backend server files._

3.  **Enable the plugin:** Add this to your `book.toml`:

    ```toml
    [preprocessor.nix-repl]
    command = "mdbook-nix-repl"

    [output.html]
    additional-js = ["theme/nix_http.js"]
    ```

4.  **Run the backend:** The `init` command output provided the token you need.
    Open a separate terminal and run the server using that token:

    ```bash
    # 1. Get your token from theme/index.hbs if you lost it
    # Look for window.NIX_REPL_TOKEN = "..."

    # 2. Export it
    export NIX_REPL_TOKEN=your_token_here

    # 3. Run the service (Container recommended)
    cd nix-repl-backend
    podman build -t nix-repl-service .
    podman run --rm \
      -p 127.0.0.1:8080:8080 \
      -e NIX_REPL_BIND=0.0.0.0 \
      -e NIX_REPL_TOKEN=$NIX_REPL_TOKEN \
      --cap-drop=ALL --security-opt=no-new-privileges \
      localhost/nix-repl-service
    ```

5.  **Serve your book:**
    ```bash
    mdbook serve
    ```

---

## Usage in Markdown

Use fenced blocks tagged as `nix repl`:

```nix repl
2 + 2
```

**Output:** `4`

```nix repl
"goodbye ${ { d = "world";}.d}"
```

**Output:** `"goodbye world"`

The preprocessor rewrites these into interactive blocks with a “Run” button and
an output area.

---

## Security Model

This tool is designed for **local development**. To ensure safety, it implements
several security layers:

1.  **Authentication:** The browser and server share a randomly generated secret
    token (`NIX_REPL_TOKEN`). Requests without this token are rejected (403
    Forbidden).

2.  **Localhost Only:** The server binds strictly to `127.0.0.1` on the host,
    preventing access from the local network or internet.

3.  **CORS & Origin Locking:** The server rejects requests from non-local
    origins to prevent CSRF attacks from malicious websites.

4.  **Container Hardening:** The recommended Podman/Docker setup drops all root
    capabilities (`--cap-drop=ALL`) and prevents privilege escalation
    (`no-new-privileges`).

---

## How it Works

The preprocessor only generates HTML; it does not talk to Nix directly. A small
JS helper (`theme/nix_http.js`) sends the code to an HTTP endpoint and displays
the result.

The `init` command sets up the following integration for you:

1.  **Frontend Script:** Creates `theme/nix_http.js` to handle the UI logic.
2.  **Theme Injection:** Injects the endpoint and auth token into
    `theme/index.hbs`:

    ```html
    <script>
      window.NIX_REPL_ENDPOINT = "http://127.0.0.1:8080/";
      window.NIX_REPL_TOKEN = "a1b2c3d4...";
    </script>
    ```

3.  **Backend Generation:** Creates a `nix-repl-backend/` directory containing a
    Python server and Dockerfile.

---

## Backend Setup

### Option A: Containerized (Recommended)

For a secure, isolated setup, the Nix eval server runs inside a hardened
container.

**1. Build the image:**

```bash
cd nix-repl-backend
podman build -t nix-repl-service .
```

**2. Run securely:**

```bash
export NIX_REPL_TOKEN=... # From your index.hbs
podman run --rm \
  -p 127.0.0.1:8080:8080 \
  -e NIX_REPL_BIND=0.0.0.0 \
  -e NIX_REPL_TOKEN=$NIX_REPL_TOKEN \
  --cap-drop=ALL --security-opt=no-new-privileges \
  localhost/nix-repl-service
```

### Option B: Native (NixOS Users)

If you are on NixOS, you can run the server directly.

```bash
export NIX_REPL_TOKEN=... # From your index.hbs
cd nix-repl-backend
python3 server.py
```

> ⚠️ Security Warning: Running natively is less secure than the container
> method. While nix eval is sandboxed, running the server directly on your host
> lacks the resource limits (CPU/RAM) and filesystem isolation provided by the
> container. A "while true" loop in Nix could freeze your whole system, or a
> vulnerability in the server could expose user files. Use the container
> whenever possible.

---

## Protocol

- **Request:** POST `NIX_REPL_ENDPOINT`
  - Headers: `X-Nix-Repl-Token: <token>`
  - Body: `{ "code": "1 + 1" }`

- **Response:** `{ "stdout": "2\n" }` or `{ "error": "..." }`
