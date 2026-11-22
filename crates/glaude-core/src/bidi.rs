//! Paragraph direction detection.
//!
//! The browser's `dir="auto"` picks the direction from the first strong
//! character, so a Persian sentence that starts with `Claude` renders LTR.
//! We instead look at the share of strong RTL letters in the whole block.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Dir {
    Ltr,
    Rtl,
}

impl Dir {
    pub fn as_str(self) -> &'static str {
        match self {
            Dir::Ltr => "ltr",
            Dir::Rtl => "rtl",
        }
    }
}

/// Share of strong RTL letters at or above which a block is laid out RTL.
const RTL_SHARE: f32 = 0.3;

pub fn is_rtl_char(c: char) -> bool {
    matches!(c as u32,
        0x0590..=0x08FF      // Hebrew, Arabic, Syriac, Thaana, NKo, Samaritan, Arabic ext
        | 0xFB1D..=0xFDFF    // Hebrew/Arabic presentation forms A
        | 0xFE70..=0xFEFF    // Arabic presentation forms B
        | 0x10800..=0x10FFF  // historic RTL scripts
        | 0x1E800..=0x1EFFF)
        && c.is_alphabetic()
}

fn is_ltr_strong(c: char) -> bool {
    c.is_alphabetic() && !is_rtl_char(c)
}

/// Detect direction of a block of plain text.
pub fn detect(text: &str) -> Dir {
    let (mut rtl, mut ltr) = (0u32, 0u32);
    for c in text.chars() {
        if is_rtl_char(c) {
            rtl += 1;
        } else if is_ltr_strong(c) {
            ltr += 1;
        }
    }
    if rtl == 0 {
        return Dir::Ltr;
    }
    if rtl as f32 / (rtl + ltr) as f32 >= RTL_SHARE {
        Dir::Rtl
    } else {
        Dir::Ltr
    }
}

/// Detect direction of markdown source, ignoring code spans, fenced code
/// and URLs, which are always LTR and would skew the ratio.
pub fn detect_markdown(src: &str) -> Dir {
    let mut plain = String::with_capacity(src.len());
    let mut in_fence = false;
    for line in src.lines() {
        let t = line.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let mut in_code = false;
        for word in line.split_inclusive(' ') {
            if word.contains("://") {
                continue;
            }
            for c in word.chars() {
                if c == '`' {
                    in_code = !in_code;
                } else if !in_code {
                    plain.push(c);
                }
            }
        }
        plain.push('\n');
    }
    detect(&plain)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_english_is_ltr() {
        assert_eq!(detect("Hello world"), Dir::Ltr);
    }

    #[test]
    fn persian_is_rtl() {
        assert_eq!(detect("سلام دنیا"), Dir::Rtl);
    }

    #[test]
    fn persian_starting_with_latin_is_rtl() {
        assert_eq!(
            detect("Claude یک دستیار هوش مصنوعی است که کد می‌نویسد"),
            Dir::Rtl
        );
    }

    #[test]
    fn english_with_one_persian_word_is_ltr() {
        assert_eq!(
            detect("The word سلام means hello in Persian and is used everywhere"),
            Dir::Ltr
        );
    }

    #[test]
    fn code_spans_do_not_count() {
        assert_eq!(
            detect_markdown("فایل `src/main.rs` و `Cargo.toml` را ببین"),
            Dir::Rtl
        );
    }

    #[test]
    fn fenced_code_is_ignored() {
        let md = "این کد:\n```rust\nfn main() { println!(\"hello world\"); }\n```\n";
        assert_eq!(detect_markdown(md), Dir::Rtl);
    }

    #[test]
    fn digits_and_punctuation_only_is_ltr() {
        assert_eq!(detect("123 - 456 !"), Dir::Ltr);
    }
}
