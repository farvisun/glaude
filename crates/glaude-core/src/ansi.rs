//! ANSI SGR → HTML spans, for statusline scripts. Output is escaped text in
//! `<span>`s; 16-colour codes use theme classes (`ansi-fg-1`…), 256-colour
//! and truecolour use inline styles.

use crate::render::escape_html;

#[derive(Clone, Default, PartialEq)]
struct Style {
    fg: Option<String>,
    bg: Option<String>,
    fg_class: Option<u8>,
    bg_class: Option<u8>,
    bold: bool,
    dim: bool,
    italic: bool,
    underline: bool,
}

impl Style {
    fn open(&self) -> Option<String> {
        if *self == Style::default() {
            return None;
        }
        let mut classes = Vec::new();
        let mut css = Vec::new();
        if let Some(c) = self.fg_class {
            classes.push(format!("ansi-fg-{c}"));
        }
        if let Some(c) = self.bg_class {
            classes.push(format!("ansi-bg-{c}"));
        }
        if let Some(c) = &self.fg {
            css.push(format!("color:{c}"));
        }
        if let Some(c) = &self.bg {
            css.push(format!("background:{c}"));
        }
        if self.bold {
            css.push("font-weight:600".into());
        }
        if self.dim {
            css.push("opacity:.65".into());
        }
        if self.italic {
            css.push("font-style:italic".into());
        }
        if self.underline {
            css.push("text-decoration:underline".into());
        }
        let mut tag = String::from("<span");
        if !classes.is_empty() {
            tag.push_str(&format!(" class=\"{}\"", classes.join(" ")));
        }
        if !css.is_empty() {
            tag.push_str(&format!(" style=\"{}\"", css.join(";")));
        }
        tag.push('>');
        Some(tag)
    }
}

fn xterm256(n: u8) -> String {
    const BASE: [&str; 16] = [
        "#000000", "#cd3131", "#0dbc79", "#e5e510", "#2472c8", "#bc3fbc", "#11a8cd", "#e5e5e5",
        "#666666", "#f14c4c", "#23d18b", "#f5f543", "#3b8eea", "#d670d6", "#29b8db", "#ffffff",
    ];
    match n {
        0..=15 => BASE[n as usize].into(),
        16..=231 => {
            let i = n - 16;
            let c = |v: u8| if v == 0 { 0 } else { 55 + v as u32 * 40 };
            format!("#{:02x}{:02x}{:02x}", c(i / 36), c((i / 6) % 6), c(i % 6))
        }
        _ => {
            let v = 8 + (n as u32 - 232) * 10;
            format!("#{v:02x}{v:02x}{v:02x}")
        }
    }
}

fn apply(style: &mut Style, params: &[u32]) {
    let mut i = 0;
    let params = if params.is_empty() { &[0][..] } else { params };
    while i < params.len() {
        let p = params[i];
        match p {
            0 => *style = Style::default(),
            1 => style.bold = true,
            2 => style.dim = true,
            3 => style.italic = true,
            4 => style.underline = true,
            22 => {
                style.bold = false;
                style.dim = false;
            }
            23 => style.italic = false,
            24 => style.underline = false,
            30..=37 => {
                style.fg_class = Some((p - 30) as u8);
                style.fg = None;
            }
            90..=97 => {
                style.fg_class = Some((p - 90 + 8) as u8);
                style.fg = None;
            }
            40..=47 => {
                style.bg_class = Some((p - 40) as u8);
                style.bg = None;
            }
            100..=107 => {
                style.bg_class = Some((p - 100 + 8) as u8);
                style.bg = None;
            }
            39 => {
                style.fg = None;
                style.fg_class = None;
            }
            49 => {
                style.bg = None;
                style.bg_class = None;
            }
            38 | 48 => {
                let color = match params.get(i + 1) {
                    Some(5) => {
                        let c = params.get(i + 2).map(|n| xterm256((*n).min(255) as u8));
                        i += 2;
                        c
                    }
                    Some(2) => {
                        let rgb = (params.get(i + 2), params.get(i + 3), params.get(i + 4));
                        i += 4;
                        match rgb {
                            (Some(r), Some(g), Some(b)) => Some(format!(
                                "#{:02x}{:02x}{:02x}",
                                (*r).min(255),
                                (*g).min(255),
                                (*b).min(255)
                            )),
                            _ => None,
                        }
                    }
                    _ => None,
                };
                if p == 38 {
                    style.fg = color;
                    style.fg_class = None;
                } else {
                    style.bg = color;
                    style.bg_class = None;
                }
            }
            _ => {}
        }
        i += 1;
    }
}

/// Convert text with ANSI escapes to HTML. Non-SGR escapes are dropped.
pub fn to_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 2);
    let mut style = Style::default();
    let mut text = String::new();
    let flush = |out: &mut String, text: &mut String, style: &Style| {
        if text.is_empty() {
            return;
        }
        if let Some(tag) = style.open() {
            out.push_str(&tag);
            out.push_str(&escape_html(text));
            out.push_str("</span>");
        } else {
            out.push_str(&escape_html(text));
        }
        text.clear();
    };
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\x1b' {
            if c != '\r' {
                text.push(c);
            }
            continue;
        }
        match chars.peek() {
            Some('[') => {
                chars.next();
                let mut seq = String::new();
                let mut fin = None;
                for d in chars.by_ref() {
                    if ('@'..='~').contains(&d) {
                        fin = Some(d);
                        break;
                    }
                    seq.push(d);
                }
                if fin == Some('m') {
                    flush(&mut out, &mut text, &style);
                    let params: Vec<u32> = seq
                        .split([';', ':'])
                        .filter(|s| !s.is_empty())
                        .filter_map(|s| s.parse().ok())
                        .collect();
                    apply(&mut style, &params);
                }
            }
            Some(']') => {
                // OSC (e.g. hyperlinks): skip to BEL or ST.
                chars.next();
                while let Some(d) = chars.next() {
                    if d == '\x07' {
                        break;
                    }
                    if d == '\x1b' && chars.peek() == Some(&'\\') {
                        chars.next();
                        break;
                    }
                }
            }
            _ => {
                chars.next();
            }
        }
    }
    flush(&mut out, &mut text, &style);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_and_escaping() {
        let html = to_html("\x1b[1;38;2;255;0;0mred <b>\x1b[0m plain \x1b[32mgreen\x1b[39m");
        assert_eq!(
            html,
            "<span style=\"color:#ff0000;font-weight:600\">red &lt;b&gt;</span> plain <span class=\"ansi-fg-2\">green</span>"
        );
    }

    #[test]
    fn drops_osc_and_other_csi() {
        let html = to_html("\x1b]8;;https://x\x07link\x1b]8;;\x07\x1b[2K done");
        assert_eq!(html, "link done");
    }

    #[test]
    fn palette_256() {
        assert_eq!(xterm256(196), "#ff0000");
        assert_eq!(xterm256(244), "#808080");
    }
}
