use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use mdbook_preprocessor::book::Book;
use mdbook_preprocessor::{Preprocessor, PreprocessorContext, parse_input};
use std::fs;
use std::io;
use std::path::Path;

use mdbook_nix_repl::NixRepl;

const JS_CONTENT: &str = include_str!("../theme/nix_http.js");
const SERVER_RUST: &str = include_str!("../server/src/main.rs");
const SERVER_CARGO_TOML: &str = include_str!("../server/Cargo.toml.inc");
const DOCKERFILE: &str = include_str!("../server/Dockerfile");

/// Command-line interface for the `mdbook-nix-repl` helper.
///
/// This is intended to be used both by `mdbook` (as a preprocessor)
/// and directly by users for initial setup.
#[derive(Parser)]
#[command(name = "mdbook-nix-repl")]
#[command(about = "A mdbook preprocessor for interactive Nix REPL blocks")]
#[command(version, propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Report whether the preprocessor supports a given renderer.
    Supports { renderer: String },
    /// Set up the theme JS and backend skeleton in the current project.
    Init {
        /// If set, try to auto-detect OS and print extra hints.
        #[arg(long)]
        auto: bool,
    },
}

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Init { auto }) => handle_init(auto),
        Some(Commands::Supports { renderer }) => {
            let supported = NixRepl.supports_renderer(&renderer).unwrap_or(false);
            if supported {
                println!("true");
                std::process::exit(0);
            } else {
                std::process::exit(1);
            }
        }
        // Default mode: act as an mdBook preprocessor, reading JSON on stdin.
        std::prelude::v1::None => run_preprocessor(),
    }
}

/// Run the preprocessor in the standard mdBook pipeline:
/// read context + book from stdin, process, and write the result to stdout.
fn run_preprocessor() -> Result<()> {
    let (ctx, book): (PreprocessorContext, Book) = parse_input(io::stdin())?;
    // Version check
    let book_version = semver::Version::parse(&ctx.mdbook_version)?;
    let version_req = semver::VersionReq::parse(mdbook_preprocessor::MDBOOK_VERSION)?;

    if !version_req.matches(&book_version) {
        eprintln!(
            "Warning: mdbook-nix-repl was built for mdBook {}, but invoked from {}",
            mdbook_preprocessor::MDBOOK_VERSION,
            ctx.mdbook_version
        );
    }
    let pre = NixRepl;
    let processed = pre.run(&ctx, book)?;
    serde_json::to_writer(io::stdout(), &processed)?;
    Ok(())
}

fn generate_secure_token() -> String {
    use rand::TryRngCore;
    use rand::rngs::OsRng;

    let mut bytes = [0u8; 24]; // paranoid setup use 32 bytes = 256 bits
    OsRng
        .try_fill_bytes(&mut bytes)
        .expect("OS RNG unavailable");
    hex::encode(bytes)
}

fn read_endpoint_from_book_toml() -> Option<String> {
    let s = std::fs::read_to_string("book.toml").ok()?;
    let t = s.parse::<toml::Table>().ok()?; // parsing via Table is the standard approach [web:203]

    let endpoint = t
        .get("preprocessor")?
        .get("nix-repl")?
        .get("endpoint")?
        .as_str()?
        .trim();

    if endpoint.is_empty() {
        None
    } else {
        Some(endpoint.to_string())
    }
}

fn endpoint_port(endpoint: &str) -> Option<u16> {
    let u = url::Url::parse(endpoint).ok()?;
    u.port_or_known_default()
}

fn extract_existing_token(hbs: &str) -> Option<String> {
    let needle = "window.NIX_REPL_TOKEN = \"";
    let start = hbs.find(needle)? + needle.len();
    let rest = &hbs[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Initialize the assets needed for interactive `nix repl` blocks.
///
/// - Writes the theme JS into `theme/`
/// - Injects a small config snippet into `theme/index.hbs`
/// - Creates a `nix-repl-backend` directory with a Rust server skeleton
fn handle_init(auto: bool) -> Result<()> {
    println!("📦 Initializing mdbook-nix-repl...");

    let theme_dir = Path::new("theme");
    if !theme_dir.exists() {
        fs::create_dir(theme_dir).context("Failed to create theme directory")?;
    }

    // 1. Write Theme JS
    let js_path = theme_dir.join("nix_http.js");
    fs::write(&js_path, JS_CONTENT).context("Failed to write nix_http.js")?;
    println!("✅ Created theme/nix_http.js");

    // 2. Inject/Update configuration in index.hbs
    let index_path = theme_dir.join("index.hbs");
    let (endpoint, token) = if !index_path.exists() {
        println!("⚠️  theme/index.hbs not found. Run `mdbook theme` first.");
        (
            "http://127.0.0.1:8080/".to_string(),
            generate_secure_token(),
        )
    } else {
        let content = fs::read_to_string(&index_path)?;

        let mut endpoint =
            read_endpoint_from_book_toml().unwrap_or_else(|| "http://127.0.0.1:8080/".to_string());
        if !endpoint.ends_with('/') {
            endpoint.push('/');
        }

        let token = extract_existing_token(&content).unwrap_or_else(generate_secure_token);

        let snippet = format!(
            r#"
<!-- mdbook-nix-repl config -->
<script>
  window.NIX_REPL_ENDPOINT = "{endpoint}";
  window.NIX_REPL_TOKEN = "{token}";
</script>
"#
        );

        let new_content = if let Some(start) = content.find("<!-- mdbook-nix-repl config -->") {
            let end = content[start..]
                .find("</script>")
                .map(|i| start + i + "</script>".len())
                .unwrap_or(content.len());

            let mut s = content;
            s.replace_range(start..end, snippet.trim_matches('\n'));
            s
        } else {
            content.replace("</body>", &format!("{snippet}\n</body>"))
        };

        fs::write(&index_path, new_content)?;
        println!("✅ Updated mdbook-nix-repl config in theme/index.hbs");

        (endpoint, token)
    };

    // 3. Write Backend Files
    let backend_dir = Path::new("nix-repl-backend");
    if !backend_dir.exists() {
        fs::create_dir(backend_dir)?;
    }

    let server_src_dir = backend_dir.join("src");
    fs::create_dir_all(&server_src_dir)?;

    fs::write(server_src_dir.join("main.rs"), SERVER_RUST)?;
    fs::write(backend_dir.join("Cargo.toml"), SERVER_CARGO_TOML)?;
    fs::write(backend_dir.join("Dockerfile"), DOCKERFILE)?;
    println!("✅ Created backend files in ./nix-repl-backend/");

    // 4. Advise
    if auto {
        detect_os_and_advise(&token, &endpoint);
    } else {
        println!("\n🚀 Setup complete. Endpoint: {endpoint} Token generated: {token}");
    }

    Ok(())
}

/// Try to detect the host OS and print a minimal run guide for the backend.
///
/// The container recipe is meant to work everywhere; on NixOS a native run is
/// also suggested for convenience.
fn detect_os_and_advise(token: &str, endpoint: &str) {
    let _ = endpoint;
    let is_nixos = fs::read_to_string("/etc/os-release")
        .map(|c| c.to_lowercase().contains("id=nixos"))
        .unwrap_or(false);
    let port = endpoint_port(endpoint).unwrap_or(8080);

    println!("\n🔍 System Detection:");
    println!("\n📋 Quick Start:");
    println!("   1. Build the Rust server:");
    println!("      $ cd nix-repl-backend && cargo build --release");
    println!("   2. Build the container:");
    println!("      $ podman build -t nix-repl-service .");
    println!("   3. Run the container:");
    println!("      $ podman run --rm -p 127.0.0.1:{port}:8080 \\");
    println!("         -e NIX_REPL_BIND=0.0.0.0 \\");
    println!("         -e NIX_REPL_TOKEN={} \\", token);
    println!("         --cap-drop=ALL --security-opt=no-new-privileges \\");
    println!("         nix-repl-service");

    if is_nixos {
        println!("\n   🎉 NixOS detected! You can also run natively:");
        println!("      $ export NIX_REPL_TOKEN={}", token);
        println!("      $ cd nix-repl-backend");
        // Native run uses the default 127.0.0.1 bind (secure by default)
        println!("      $ cargo run --release");
    } else {
        println!("\n   ℹ️  Non-NixOS: Container recommended for Nix isolation.");
    }

    println!("\n🔒 Security: Token saved to theme/index.hbs");
    println!("   Keep NIX_REPL_TOKEN={} private!", token);
}
