//! Line reading with a size cap. Session files hold single lines of tens of
//! MB (embedded images, tool output); growing a fresh buffer for each one
//! fragments the allocator and keeps hundreds of MB resident. This reuses
//! one buffer and keeps at most `cap` bytes of a line.

use std::io::{self, BufRead};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Line {
    /// Bytes consumed from the reader, including the newline.
    pub consumed: usize,
    /// The line ended with `\n` (false: end of file mid-line).
    pub complete: bool,
    /// The line was longer than the cap; the buffer holds its first `cap` bytes.
    pub truncated: bool,
}

/// Read one line into `buf` (cleared first). `consumed == 0` means EOF.
pub fn read_line_capped<R: BufRead>(r: &mut R, buf: &mut Vec<u8>, cap: usize) -> io::Result<Line> {
    buf.clear();
    let mut consumed = 0;
    let mut truncated = false;
    loop {
        let avail = match r.fill_buf() {
            Ok(a) => a,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        if avail.is_empty() {
            return Ok(Line {
                consumed,
                complete: false,
                truncated,
            });
        }
        let (n, done) = match memchr::memchr(b'\n', avail) {
            Some(i) => (i + 1, true),
            None => (avail.len(), false),
        };
        let room = cap.saturating_sub(buf.len());
        if n <= room {
            buf.extend_from_slice(&avail[..n]);
        } else {
            buf.extend_from_slice(&avail[..room]);
            truncated = true;
        }
        r.consume(n);
        consumed += n;
        if done {
            return Ok(Line {
                consumed,
                complete: true,
                truncated,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caps_long_lines_and_reports_partial_tail() {
        let data = format!("short\n{}\nend", "x".repeat(100));
        let mut r = io::BufReader::with_capacity(7, data.as_bytes());
        let mut buf = Vec::new();
        let a = read_line_capped(&mut r, &mut buf, 10).unwrap();
        assert_eq!((a.consumed, a.complete, a.truncated), (6, true, false));
        assert_eq!(buf, b"short\n");
        let b = read_line_capped(&mut r, &mut buf, 10).unwrap();
        assert_eq!((b.consumed, b.complete, b.truncated), (101, true, true));
        assert_eq!(buf.len(), 10);
        let c = read_line_capped(&mut r, &mut buf, 10).unwrap();
        assert_eq!((c.consumed, c.complete), (3, false));
        let d = read_line_capped(&mut r, &mut buf, 10).unwrap();
        assert_eq!(d.consumed, 0);
    }
}
