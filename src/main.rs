use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use mdbook_preprocessor::book::Book;
use mdbook_preprocessor::{Preprocessor, PreprocessorContext, parse_input};
use std::fs;
use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use mdbook_nix_repl::NixRepl;

const JS_CONTENT: &str = include_str!("../theme/nix_http.js");
const SERVER_RUST: &str = include_str!("../server/src/main.rs");
const SERVER_CARGO_TOML: &str = include_str!("../server/Cargo.toml.inc");
const DOCKERFILE: &str = include_str!("../server/Dockerfile");

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
    Supports {
        renderer: String,
    },
    Init {
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
        None => run_preprocessor(),
    }
}

fn run_preprocessor() -> Result<()> {
    let (ctx, book): (PreprocessorContext, Book) = parse_input(io::stdin())?;
    let pre = NixRepl;
    let processed = pre.run(&ctx, book)?;
    serde_json::to_writer(io::stdout(), &processed)?;
    Ok(())
}

fn handle_init(auto: bool) -> Result<()> {
    println!("📦 Initializing mdbook-nix-repl...");

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let token = format!("{:x}", nanos);

    let theme_dir = Path::new("theme");
    if !theme_dir.exists() {
        fs::create_dir(theme_dir).context("Failed to create theme directory")?;
    }

    // 1. Write Theme JS
    let js_path = theme_dir.join("nix_http.js");
    fs::write(&js_path, JS_CONTENT).context("Failed to write nix_http.js")?;
    println!("✅ Created theme/nix_http.js");

    // 2. Inject Configuration into index.hbs
    let index_path = theme_dir.join("index.hbs");
    if !index_path.exists() {
        println!("⚠️  theme/index.hbs not found. Run `mdbook theme` first.");
    } else {
        let content = fs::read_to_string(&index_path)?;
        let mut new_content = content.clone();
        let mut modified = false;

        if !new_content.contains("window.NIX_REPL_ENDPOINT") {
            let snippet = format!(
                r#"
<!-- mdbook-nix-repl config -->
<script>
  window.NIX_REPL_ENDPOINT = "http://127.0.0.1:8080/";
  window.NIX_REPL_TOKEN = "{}";
</script>
"#,
                token
            );
            new_content = new_content.replace("</body>", &format!("{}\n</body>", snippet));
            modified = true;
            println!("✅ Injected endpoint and auth token into theme/index.hbs");
        }

        if modified {
            fs::write(&index_path, new_content)?;
        }
    }

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
        detect_os_and_advise(&token);
    } else {
        println!("\n🚀 Setup complete. Token generated: {}", token);
    }

    Ok(())
}

fn detect_os_and_advise(token: &str) {
    let is_nixos = fs::read_to_string("/etc/os-release")
        .map(|c| c.to_lowercase().contains("id=nixos"))
        .unwrap_or(false);

    println!("\n🔍 System Detection:");
    println!("\n📋 Quick Start:");
    println!("   1. Build the Rust server:");
    println!("      $ cd nix-repl-backend && cargo build --release");
    println!("   2. Build the container:");
    println!("      $ podman build -t nix-repl-service .");
    println!("   3. Run the container:");
    // Added NIX_REPL_BIND=0.0.0.0 so it works inside container, while -p keeps it local-only on host
    println!("      $ podman run --rm -p 127.0.0.1:8080:8080 \\");
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
