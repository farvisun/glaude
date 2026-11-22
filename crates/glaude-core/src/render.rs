//! Markdown to sanitized HTML blocks.
//!
//! The source is split into top-level blocks; each block is rendered on its
//! own and gets its own text direction. While a message streams in, only
//! the last (unfinished) block is re-rendered.

use std::ops::Range;
use std::sync::OnceLock;

use pulldown_cmark::{html, CodeBlockKind, CowStr, Event, Options, Parser, Tag, TagEnd};
use syntect::html::{ClassStyle, ClassedHTMLGenerator};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

use crate::bidi::{self, Dir};
use crate::model::Block;

/// Code above this size is shown without highlighting.
const HIGHLIGHT_LIMIT: usize = 200_000;
/// While streaming, an unfinished code block above this size is not
/// re-highlighted on every tick.
const STREAM_HIGHLIGHT_LIMIT: usize = 16_000;

fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES
}

fn syntaxes() -> &'static SyntaxSet {
    static SS: OnceLock<SyntaxSet> = OnceLock::new();
    SS.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn sanitizer() -> &'static ammonia::Builder<'static> {
    static B: OnceLock<ammonia::Builder<'static>> = OnceLock::new();
    B.get_or_init(|| {
        let mut b = ammonia::Builder::default();
        b.add_tags(["bdi", "input", "span", "del", "details", "summary"])
            .add_generic_attributes(["class", "dir"])
            .add_tag_attributes("input", ["type", "checked", "disabled"])
            .add_tag_attributes("ol", ["start"])
            .add_tag_attributes("td", ["align"])
            .add_tag_attributes("th", ["align"])
            .link_rel(None);
        b
    })
}

pub fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Byte ranges of the top-level blocks of `src`.
pub fn top_level_ranges(src: &str) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (ev, range) in Parser::new_ext(src, options()).into_offset_iter() {
        match ev {
            Event::Start(_) => {
                if depth == 0 {
                    start = range.start;
                }
                depth += 1;
            }
            Event::End(_) => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    ranges.push(start..range.end);
                }
            }
            _ if depth == 0 => ranges.push(range),
            _ => {}
        }
    }
    ranges
}

fn highlight(code: &str, lang: &str) -> Option<String> {
    if code.len() > HIGHLIGHT_LIMIT || lang.is_empty() {
        return None;
    }
    let ss = syntaxes();
    let token = lang.split([' ', ',', '{']).next().unwrap_or(lang);
    let syntax = ss
        .find_syntax_by_token(token)
        .or_else(|| ss.find_syntax_by_extension(token))?;
    let mut gen = ClassedHTMLGenerator::new_with_class_style(
        syntax,
        ss,
        ClassStyle::SpacedPrefixed { prefix: "hl-" },
    );
    for line in LinesWithEndings::from(code) {
        gen.parse_html_for_line_which_includes_newline(line).ok()?;
    }
    Some(gen.finalize())
}

fn is_ltr_run_char(c: char) -> bool {
    (c.is_alphanumeric() && !bidi::is_rtl_char(c) && !('\u{06F0}'..='\u{06F9}').contains(&c))
        || matches!(
            c,
            '.' | '-' | '_' | '/' | ':' | '@' | '#' | '+' | '=' | '%' | '~' | '&' | '?'
        )
}

/// In an RTL block, isolate Latin runs ("Claude Code 2.1", "src/main.rs")
/// so neighbouring punctuation and digits stay where they belong.
fn isolate_ltr_runs(text: &str) -> String {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut out = String::with_capacity(text.len() + 16);
    let mut i = 0;
    while i < chars.len() {
        let (pos, c) = chars[i];
        let starts_run = c.is_ascii_alphabetic() || (c.is_alphabetic() && !bidi::is_rtl_char(c));
        if !starts_run {
            out.push_str(&escape_html(&text[pos..pos + c.len_utf8()]));
            i += 1;
            continue;
        }
        // Extend over LTR chars and single spaces between LTR words.
        let mut j = i;
        let mut last_good = i;
        while j < chars.len() {
            let ch = chars[j].1;
            if is_ltr_run_char(ch) {
                if ch.is_alphanumeric() {
                    last_good = j;
                }
                j += 1;
            } else if ch == ' ' && j + 1 < chars.len() && chars[j + 1].1.is_ascii_alphanumeric() {
                j += 1;
            } else {
                break;
            }
        }
        let end = chars
            .get(last_good + 1)
            .map(|(p, _)| *p)
            .unwrap_or(text.len());
        out.push_str("<bdi>");
        out.push_str(&escape_html(&text[pos..end]));
        out.push_str("</bdi>");
        i = last_good + 1;
    }
    out
}

/// Render one block of markdown. `final_` is false while the block is
/// still streaming.
pub fn render_block(src: &str, final_: bool) -> Block {
    let dir = bidi::detect_markdown(src);
    let mut events: Vec<Event> = Vec::new();
    let mut code: Option<(String, String)> = None;
    for ev in Parser::new_ext(src, options()) {
        match ev {
            Event::Start(Tag::CodeBlock(kind)) => {
                let lang = match kind {
                    CodeBlockKind::Fenced(l) => l.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                code = Some((lang, String::new()));
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some((lang, body)) = code.take() {
                    let can = final_ || body.len() <= STREAM_HIGHLIGHT_LIMIT;
                    let inner = can
                        .then(|| highlight(&body, &lang))
                        .flatten()
                        .unwrap_or_else(|| escape_html(&body));
                    let class = if lang.is_empty() {
                        String::new()
                    } else {
                        format!(
                            " class=\"language-{}\"",
                            escape_html(lang.split(' ').next().unwrap_or(""))
                        )
                    };
                    events.push(Event::Html(CowStr::from(format!(
                        "<pre dir=\"ltr\"><code{class}>{inner}</code></pre>"
                    ))));
                }
            }
            Event::Text(t) if code.is_some() => {
                if let Some((_, body)) = code.as_mut() {
                    body.push_str(&t);
                }
            }
            Event::Text(t) if dir == Dir::Rtl => {
                events.push(Event::InlineHtml(CowStr::from(isolate_ltr_runs(&t))));
            }
            other => events.push(other),
        }
    }
    // A fence still open while streaming: show what we have.
    if let Some((_, body)) = code.take() {
        events.push(Event::Html(CowStr::from(format!(
            "<pre dir=\"ltr\"><code>{}</code></pre>",
            escape_html(&body)
        ))));
    }
    let mut raw = String::with_capacity(src.len() * 2);
    html::push_html(&mut raw, events.into_iter());
    Block {
        html: sanitizer().clean(&raw).to_string(),
        dir,
    }
}

/// Render a whole markdown document.
pub fn render(src: &str) -> Vec<Block> {
    top_level_ranges(src)
        .into_iter()
        .map(|r| render_block(&src[r], true))
        .collect()
}

/// Plain text (no markdown), e.g. a user prompt: one block per paragraph,
/// line breaks kept.
pub fn render_plain(text: &str) -> Vec<Block> {
    text.split("\n\n")
        .filter(|p| !p.trim().is_empty())
        .map(|p| {
            let dir = bidi::detect(p);
            let body = if dir == Dir::Rtl {
                isolate_ltr_runs(p)
            } else {
                escape_html(p)
            };
            Block {
                html: format!("<p>{}</p>", body.replace('\n', "<br>")),
                dir,
            }
        })
        .collect()
}

/// Incremental renderer for a streaming message.
#[derive(Default)]
pub struct StreamRenderer {
    src: String,
    done: Vec<Block>,
    /// Byte offset where the first unfinished block starts.
    done_end: usize,
}

impl StreamRenderer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn text(&self) -> &str {
        &self.src
    }

    /// Append a delta. Returns the index of the first changed block and
    /// the blocks from there on.
    pub fn push(&mut self, delta: &str) -> (usize, Vec<Block>) {
        self.src.push_str(delta);
        self.refresh()
    }

    fn refresh(&mut self) -> (usize, Vec<Block>) {
        let from = self.done.len();
        let tail = &self.src[self.done_end..];
        let ranges = top_level_ranges(tail);
        if ranges.len() > 1 {
            for r in &ranges[..ranges.len() - 1] {
                self.done.push(render_block(&tail[r.clone()], true));
            }
            self.done_end += ranges[ranges.len() - 1].start;
        }
        let mut out: Vec<Block> = self.done[from..].to_vec();
        let rest = &self.src[self.done_end..];
        if !rest.trim().is_empty() {
            out.push(render_block(rest, false));
        }
        (from, out)
    }

    /// Final render of the whole text.
    pub fn finish(&self) -> Vec<Block> {
        render(&self.src)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_top_level_blocks() {
        let md = "# Title\n\npara one\n\n- a\n- b\n\n```rs\nfn x() {}\n```\n";
        let r = top_level_ranges(md);
        assert_eq!(r.len(), 4);
        assert_eq!(&md[r[0].clone()].trim_end(), &"# Title");
    }

    #[test]
    fn rtl_block_gets_rtl_dir_and_bdi() {
        let b = render_block("نسخه‌ی Claude Code 2.1 منتشر شد.", true);
        assert_eq!(b.dir, Dir::Rtl);
        assert!(b.html.contains("<bdi>Claude Code 2.1</bdi>"), "{}", b.html);
        assert!(b.html.ends_with("منتشر شد.</p>\n"), "{}", b.html);
    }

    #[test]
    fn code_blocks_are_ltr_and_highlighted() {
        let b = render_block("```rust\nfn main() {}\n```", true);
        assert!(b
            .html
            .starts_with("<pre dir=\"ltr\"><code class=\"language-rust\">"));
        assert!(b.html.contains("hl-"), "{}", b.html);
    }

    #[test]
    fn strips_scripts() {
        let b = render_block(
            "hello <script>alert(1)</script> <img src=x onerror=alert(1)>",
            true,
        );
        assert!(!b.html.contains("<script"));
        assert!(!b.html.contains("onerror"));
    }

    #[test]
    fn task_lists_survive_sanitizing() {
        let b = render_block("- [x] done\n- [ ] todo", true);
        assert!(b.html.contains("type=\"checkbox\""), "{}", b.html);
    }

    #[test]
    fn streaming_only_rerenders_tail() {
        let mut s = StreamRenderer::new();
        let (from, blocks) = s.push("first para");
        assert_eq!((from, blocks.len()), (0, 1));
        let (from, blocks) = s.push("\n\nsecond");
        assert_eq!(from, 0);
        assert_eq!(blocks.len(), 2);
        let (from, blocks) = s.push(" para continues");
        assert_eq!(from, 1, "first block is finished and not resent");
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].html.contains("second para continues"));
        assert_eq!(s.finish().len(), 2);
    }

    #[test]
    fn unfinished_fence_while_streaming() {
        let mut s = StreamRenderer::new();
        let (_, blocks) = s.push("```py\nprint(1)\n");
        assert!(
            blocks[0].html.starts_with("<pre dir=\"ltr\">"),
            "{}",
            blocks[0].html
        );
        assert!(blocks[0].html.contains("print"), "{}", blocks[0].html);
    }

    #[test]
    fn plain_text_keeps_lines() {
        let b = render_plain("سلام\nخوبی؟\n\nsecond <b>");
        assert_eq!(b.len(), 2);
        assert_eq!(b[0].dir, Dir::Rtl);
        assert!(b[0].html.contains("<br>"));
        assert!(b[1].html.contains("&lt;b&gt;"));
    }
}
