use mdbook_preprocessor::book::{Book, BookItem};
use mdbook_preprocessor::errors::Error;
use mdbook_preprocessor::{Preprocessor, PreprocessorContext};

/// Preprocessor that rewrites fenced `nix repl` code blocks into
/// interactive HTML fragments for use in the rendered book.
pub struct NixRepl;

impl Preprocessor for NixRepl {
    /// Name used to enable this preprocessor in `book.toml`.
    fn name(&self) -> &str {
        "nix-repl"
    }

    /// Walk the book and rewrite chapter content, transforming any
    /// ` ```
    fn run(&self, _ctx: &PreprocessorContext, mut book: Book) -> Result<Book, Error> {
        book.for_each_mut(|item| {
            if let BookItem::Chapter(ref mut ch) = *item {
                ch.content = rewrite_chapter(&ch.content);
            }
        });
        Ok(book)
    }

    /// Only enable this preprocessor for the HTML renderer, since it
    /// emits raw HTML markup.
    fn supports_renderer(&self, renderer: &str) -> Result<bool, Error> {
        Ok(renderer == "html")
    }
}

/// Apply all content transformations for a single chapter body.
fn rewrite_chapter(input: &str) -> String {
    rewrite_fenced_nix_repl_blocks(input)
}

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
    const START: &str = "```nix repl";
    const END: &str = "```";

    let mut out = String::new();
    let mut in_block = false;
    let mut buf = String::new();

    for line in input.lines() {
        let trimmed = line.trim_start();

        if !in_block {
            if trimmed.starts_with(START) {
                in_block = true;
                buf.clear();
            } else {
                out.push_str(line);
                out.push('\n');
            }
        } else if trimmed.starts_with(END) {
            out.push_str(&render_nix_repl_html(&buf));
            in_block = false;
        } else {
            buf.push_str(line);
            buf.push('\n');
        }
    }

    // If the input ends while still inside a fenced block, just emit the
    // raw contents rather than dropping them.
    if in_block {
        out.push_str(&buf);
    }

    out
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
