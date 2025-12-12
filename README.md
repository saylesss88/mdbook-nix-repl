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
    📦 Initializing mdbook-nix-repl...
    ✅ Created theme/nix_http.js
    ✅ Injected configuration into theme/index.hbs
    ✅ Created backend files in ./nix-repl-backend/

    🔍 System Detection:
    ☁️  Non-NixOS system detected.
    It is recommended to use the Docker backend:
    $ cd nix-repl-backend
    $ podman build -t nix-repl-service .
    $ podman run --rm -p 8080:8080 nix-repl-service
    ```

    _This command automatically creates the `theme/` files, injects the
    necessary script into `index.hbs`, and generates the backend server files._

3.  **Enable the plugin:** Add this to your `book.toml`:

    ```toml
    [preprocessor.nix-repl]
    command = "mdbook-nix-repl"

    [output.html]
    additional-js = ["theme/nix_http.js"]
    ```

4.  **Run the backend:** Open a separate terminal and run the server generated
    in step 2:

    ```bash
    # For Docker/Podman users (Recommended for non-NixOS):
    cd nix-repl-backend
    podman build -t nix-repl-service .
    podman run --rm -p 8080:8080 nix-repl-service

    # OR for native NixOS users:
    cd nix-repl-backend
    python3 server.py
    ```

5.  **Serve your book:**
    ```bash
    mdbook serve
    ```

> NOTE: As long as the backend is running, the `Run` command will work with both
> `mdbook serve` and `mdbook build`

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

## How it Works

The preprocessor only generates HTML; it does not talk to Nix directly. A small
JS helper (`theme/nix_http.js`) sends the code to an HTTP endpoint and displays
the result.

The `init` command sets up the following integration for you:

1.  **Frontend Script:** Creates `theme/nix_http.js` to handle the UI logic.

2.  **Theme Injection:** Injects the backend endpoint configuration into
    `theme/index.hbs`:

    ```html
    <script>
      window.NIX_REPL_ENDPOINT = "http://127.0.0.1:8080/";
    </script>
    ```

3.  **Backend Generation:** Creates a `nix-repl-backend/` directory containing a
    Python server and Dockerfile.

---

## Backend Setup

The `init --auto` command will detect your OS and suggest the best way to run
the backend.

### Option A: Containerized (Recommended for non-NixOS)

For a more isolated setup, the Nix eval server runs inside a container. This
works well on immutable hosts like Fedora SecureBlue or Silverblue.

The generated `Dockerfile`:

- Uses `debian:stable-slim`
- Installs Nix in single-user mode
- Enables `nix-command` and `flakes`
- Exposes port 8080

**To run:**

```bash
cd nix-repl-backend
podman build -t nix-repl-service .
podman run --rm -p 8080:8080 nix-repl-service
```

### Option B: Native (NixOS Users)

If you are already on NixOS, you can run the server directly without a
container.

**To run:**

```bash
cd nix-repl-backend
python3 server.py
```

---

## JSON Protocol

This project does not ship a mandatory backend; you can plug in any HTTP service
that matches this simple JSON protocol:

- **Request:** POST `NIX_REPL_ENDPOINT`

  ```json
  { "code": "1 + 1" }
  ```

- **Response (Success):**

  ```json
  { "stdout": "2\n" }
  ```

- **Response (Error):**
  ```json
  { "error": "some error message" }
  ```

> **Security Note:** This server executes arbitrary Nix expressions. It is
> intended for local development on trusted machines. Do not expose it to
> untrusted networks without additional sandboxing, authentication, and resource
> limits.

---

## Status

This is experimental. The UI and protocol may change. Contributions for:

- Safer backends (e.g. Nix sandboxes, systemd services).
- Better frontend UX.

are very welcome.
