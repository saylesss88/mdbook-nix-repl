use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use mdbook_preprocessor::book::Book;
use mdbook_preprocessor::{Preprocessor, PreprocessorContext, parse_input};
use std::fs;
use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

// Embed the assets directly into the binary at compile time
use mdbook_nix_repl::NixRepl;

const JS_CONTENT: &str = include_str!("../theme/nix_http.js");
const SERVER_PY: &str = include_str!("../server/server.py");
const DOCKERFILE: &str = include_str!("../server/Dockerfile");

#[derive(Parser)]
#[command(name = "mdbook-nix-repl")]
#[command(about = "A mdbook preprocessor for interactive Nix REPL blocks")]
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

    // Generate a simple local token (timestamp based) to avoid external rand deps
    // In production, use the `uuid` or `rand` crate for cryptographically secure tokens.
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

    // 2. Inject Configuration & Footer into index.hbs
    let index_path = theme_dir.join("index.hbs");
    if !index_path.exists() {
        println!("⚠️  theme/index.hbs not found. Run `mdbook theme` first.");
    } else {
        let content = fs::read_to_string(&index_path)?;
        let mut new_content = content.clone();
        let mut modified = false;

        // Inject Config
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

        // Inject Footer
        if !new_content.contains("mdbook-kanagawa-theme") {
            // Inserts footer before the content closing div usually found in default theme
            // Fallback to inserting before body end if specific div not found
            let footer_html = r#"
            <footer style="text-align: center; margin-top: 50px; font-size: 0.8em; opacity: 0.7;">
                <p>Made with <a href="https://github.com/yourusername/mdbook-kanagawa-theme">mdbook-kanagawa-theme</a></p>
            </footer>
            "#;

            // Try to place it inside the page-wrapper for better styling
            if new_content.contains("</div>\n    <!-- Unnamed -->") {
                new_content = new_content.replace(
                    "</div>\n    <!-- Unnamed -->",
                    &format!("{}\n</div>\n    <!-- Unnamed -->", footer_html),
                );
            } else {
                new_content = new_content.replace("</body>", &format!("{}\n</body>", footer_html));
            }
            modified = true;
            println!("✅ Injected footer into theme/index.hbs");
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
    fs::write(backend_dir.join("server.py"), SERVER_PY)?;
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
    if is_nixos {
        println!("   🎉 NixOS detected! Run native backend:");
        println!("   $ export NIX_REPL_TOKEN={}", token);
        println!("   $ cd nix-repl-backend && python3 server.py");
    } else {
        println!("   ☁️  Non-NixOS system. Recommended secure Docker command:");
        println!("   $ cd nix-repl-backend");
        println!("   $ podman build -t nix-repl-service .");
        println!("   $ podman run --rm -p 127.0.0.1:8080:8080 \\");
        println!("       -e NIX_REPL_TOKEN={} \\", token);
        println!("       --cap-drop=ALL --security-opt=no-new-privileges \\");
        println!("       nix-repl-service");
    }
}
