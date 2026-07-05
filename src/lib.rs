#![doc = include_str!("../README.md")]
use mdbook_preprocessor::book::{Book, BookItem};
use mdbook_preprocessor::errors::Error;
use mdbook_preprocessor::{Preprocessor, PreprocessorContext};
use serde::Deserialize;

/// Preprocessor that rewrites fenced `nix repl` code blocks into
/// interactive HTML fragments for use in the rendered book.
pub struct NixRepl;

/// Configuration options for the `nix-repl` preprocessor parsed from `book.toml`.
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct NixReplConfig {
    /// Optional URL or path to a custom Nix REPL endpoint.
    endpoint: Option<String>,
}

impl Preprocessor for NixRepl {
    /// Returns the unique identifier for this preprocessor.
    fn name(&self) -> &'static str {
        "nix-repl"
    }

    /// Runs the preprocessor, scanning and modifying chapter content to evaluate Nix code blocks.
    fn run(&self, ctx: &PreprocessorContext, mut book: Book) -> Result<Book, Error> {
        // Read config from book.toml [preprocessor.nix-repl]
        let config: NixReplConfig = ctx
            .config
            .get("preprocessor.nix-repl")
            .ok()
            .flatten()
            .unwrap_or_default();

        // Log what we found
        match &config.endpoint {
            Some(endpoint) => {
                eprintln!("✅ [nix-repl] Using custom endpoint: {endpoint}");
            }
            None => {
                eprintln!("ℹ️  [nix-repl] No custom endpoint configured, using default");
            }
        }

        book.for_each_mut(|item| {
            if let BookItem::Chapter(ref mut ch) = *item {
                ch.content = rewrite_chapter(&ch.content);
            }
        });
        Ok(book)
    }

    /// Restricts this preprocessor to the HTML renderer.
    fn supports_renderer(&self, renderer: &str) -> Result<bool, Error> {
        Ok(renderer == "html")
    }
}

/// Apply all content transformations for a single chapter body.
fn rewrite_chapter(input: &str) -> String {
    rewrite_fenced_nix_repl_blocks(input)
}

#[allow(rustdoc::private_doc_tests)]
/// Scan the chapter for fenced `nix repl` code blocks and replace them
/// with the corresponding interactive HTML widget.
///
/// A block is detected by a line starting with:
/// ```nix repl
/// ```
///
/// and terminated by the next line starting with:
/// ```
/// ```
fn rewrite_fenced_nix_repl_blocks(input: &str) -> String {
    let mut out = String::new();
    let mut in_block = false;
    let mut buf = String::new();
    let mut fence_char = '`';
    let mut fence_count = 0;

    for line in input.lines() {
        if in_block {
            // Check for closing fence (same char, >= opening count)
            if is_closing_fence(line, fence_char, fence_count) {
                out.push_str(&render_nix_repl_html(&buf));
                in_block = false;
            } else {
                buf.push_str(line);
                buf.push('\n');
            }
        } else {
            // Check for opening fence (at least 3 backticks/tildes at start)
            if let Some(fence_info) = detect_fence_start(line)
                && fence_info.info_string.starts_with("nix repl")
            {
                in_block = true;
                fence_char = fence_info.char;
                fence_count = fence_info.count;
                buf.clear();
                continue;
            }
            out.push_str(line);
            out.push('\n');
        }
    }

    if in_block {
        out.push_str(&buf);
    }
    out
}

/// Represents the parsed metadata of an opening code block fence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FenceInfo {
    /// The character used to construct the fence.
    pub char: char,
    /// The number of consecutive fence characters detected.
    pub count: usize,
    /// The raw syntax or language identifier following the fence, stripped of leading/trailing whitespace.
    pub info_string: String,
}

/// Detects if a line is the start of a markdown-style code block fence and extracts its metadata.
///
/// A valid opening fence consists of at least 3 consecutive backticks (```` ` ````) or tildes (`~`),
/// optionally preceded by leading whitespace. Any text remaining on the line after the fence
/// characters is extracted as the info string (e.g., the language identifier).
///
/// # Returns
///
/// * `Some(FenceInfo)` containing the fence character, total count, and trimmed info string if successful.
/// * `None` if the line does not start with a valid fence sequence of at least 3 characters.
fn detect_fence_start(line: &str) -> Option<FenceInfo> {
    let trimmed = line.trim_start();
    if trimmed.len() < 3 {
        return None;
    }

    let first_char = trimmed.chars().next()?;
    if first_char != '`' && first_char != '~' {
        return None;
    }

    let count = trimmed.chars().take_while(|&c| c == first_char).count();
    if count < 3 {
        return None;
    }

    let info_string = trimmed[count..].trim().to_string();

    Some(FenceInfo {
        char: first_char,
        count,
        info_string,
    })
}

/// Checks if a line is a valid closing markdown-style code block fence.
///
/// A line is considered a valid closing fence if it consists entirely of a repeating
/// `fence_char` (with a count of at least `min_count`), allowing for optional leading
/// and trailing whitespace.
fn is_closing_fence(line: &str, fence_char: char, min_count: usize) -> bool {
    let trimmed = line.trim();
    if !trimmed.starts_with(fence_char) {
        return false;
    }

    let count = trimmed.chars().take_while(|&c| c == fence_char).count();
    count >= min_count && trimmed[count..].trim().is_empty()
}

/// Render the captured `nix repl` code as an interactive HTML widget.
///
/// Escapes the source code for safe embedding and wraps it in a
/// structure that can be hooked up to client‑side JS to actually run
/// the snippets.
fn render_nix_repl_html(code: &str) -> String {
    let escaped = html_escape::encode_text(code);

    let mut html = String::new();
    html.push_str("<div class=\"nix-repl-block\">\n");
    html.push_str("  <div class=\"nix-repl-editor\">\n");
    html.push_str("    <pre><code class=\"language-nix\">");
    html.push_str(&escaped);
    html.push_str("</code></pre>\n");
    html.push_str("  </div>\n");
    html.push_str("  <div class=\"nix-repl-controls\">\n");
    html.push_str("    <button class=\"nix-repl-run\">Run</button>\n");
    html.push_str("    <span class=\"nix-repl-status\"></span>\n");
    html.push_str("  </div>\n");
    html.push_str("  <pre class=\"nix-repl-output\"></pre>\n");
    html.push_str("</div>\n");
    html
}
