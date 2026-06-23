use std::collections::HashMap;
use gpui::Hsla;
use crate::terminal::RenderCell;

trait HslaExt {
    fn into_rgba_like(self, r: u8, g: u8, b: u8) -> Self;
}

impl HslaExt for Hsla {
    fn into_rgba_like(self, r: u8, g: u8, b: u8) -> Self {
        let rf = r as f32 / 255.0;
        let gf = g as f32 / 255.0;
        let bf = b as f32 / 255.0;
        let max = rf.max(gf).max(bf);
        let min = rf.min(gf).min(bf);
        let l = (max + min) / 2.0;
        if max == min {
            return Hsla { h: 0.0, s: 0.0, l, a: 1.0 };
        }
        let d = max - min;
        let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
        let h = if max == rf {
            ((gf - bf) / d + if gf < bf { 6.0 } else { 0.0 }) / 6.0
        } else if max == gf {
            ((bf - rf) / d + 2.0) / 6.0
        } else {
            ((rf - gf) / d + 4.0) / 6.0
        };
        Hsla { h, s, l, a: 1.0 }
    }
}

#[derive(Debug, Clone)]
struct HighlightColors {
    error: Hsla,
    success: Hsla,
    warning: Hsla,
    info: Hsla,
    failure: Hsla,
    network: Hsla,
    url: Hsla,
    port: Hsla,
    debug: Hsla,
}

fn hsla(r: u8, g: u8, b: u8) -> Hsla {
    Hsla {
        h: 0.0,
        s: 0.0,
        l: 0.0,
        a: 1.0,
    }
    .into_rgba_like(r, g, b)
}

fn highlight_colors() -> HighlightColors {
    HighlightColors {
        error:   hsla(224,  96,  96), // #E06060 red
        success: hsla(126, 198, 153), // #7EC699 green
        warning: hsla(232, 201, 122), // #E8C97A yellow
        info:    hsla(108, 180, 238), // #6CB4EE blue
        failure: hsla(232, 168, 124), // #E8A87C orange
        network: hsla(199, 146, 234), // #C792EA purple
        url:     hsla( 86, 212, 199), // #56D4C7 teal
        port:    hsla(130, 170, 200), // #82AAC8 muted teal
        debug:   hsla(130, 140, 155), // #828C9B gray
    }
}

fn is_boundary(c: char) -> bool {
    !c.is_ascii_alphanumeric() && c != '_'
}

pub fn highlight_cells(
    cells: &[RenderCell],
    rows: usize,
) -> HashMap<(i32, i32), Hsla> {
    let colors = highlight_colors();

    // Pre-allocate the outer vector to the size of rows.
    let mut row_chars: Vec<Vec<(i32, char)>> = vec![Vec::with_capacity(128); rows];
    for rc in cells {
        if rc.row < 0 || (rc.row as usize) >= rows {
            continue;
        }
        row_chars[rc.row as usize].push((rc.col, rc.cell.c));
    }
    for row in row_chars.iter_mut() {
        row.sort_by_key(|&(col, _)| col);
    }

    let mut map = HashMap::new();

    // Reusable buffers to avoid allocation inside the loop
    let mut chars_buf = String::with_capacity(128);
    let mut byte_to_col: Vec<i32> = Vec::with_capacity(128);

    for (row_idx, row) in row_chars.iter().enumerate() {
        if row.is_empty() {
            continue;
        }
        let row_i32 = row_idx as i32;

        chars_buf.clear();
        byte_to_col.clear();

        for &(col, c) in row {
            chars_buf.push(c);
            while byte_to_col.len() < chars_buf.len() {
                byte_to_col.push(col);
            }
        }
        let text = chars_buf.as_str();

        // ── 1. Error keywords ──────────────────────────
        for kw in &["EMERGENCY", "CRITICAL", "FATAL", "PANIC", "ERROR", "ERR"] {
            for m in find_keyword(text, kw) {
                let start_col = byte_to_col[m];
                let end_col = byte_to_col[(m + kw.len()).min(byte_to_col.len() - 1)];
                for c in start_col..=end_col {
                    map.entry((row_i32, c)).or_insert(colors.error);
                }
            }
        }

        // ── 2. Success keywords ───────────────────────────
        for kw in &["SUCCESS", "SUCCEEDED", "PASSED", "PASS", "OK"] {
            for m in find_keyword(text, kw) {
                let start_col = byte_to_col[m];
                let end_col = byte_to_col[(m + kw.len()).min(byte_to_col.len() - 1)];
                for c in start_col..=end_col {
                    map.entry((row_i32, c)).or_insert(colors.success);
                }
            }
        }

        // ── 3. Failure keywords ───────────────────────────
        for kw in &["FAILED", "FAILURE", "DENIED", "REJECTED", "TIMEOUT", "FAIL"] {
            for m in find_keyword(text, kw) {
                let start_col = byte_to_col[m];
                let end_col = byte_to_col[(m + kw.len()).min(byte_to_col.len() - 1)];
                for c in start_col..=end_col {
                    map.entry((row_i32, c)).or_insert(colors.failure);
                }
            }
        }

        // ── 4. Warning keywords ───────────────────────────
        for kw in &["WARNING", "WARN"] {
            for m in find_keyword(text, kw) {
                let start_col = byte_to_col[m];
                let end_col = byte_to_col[(m + kw.len()).min(byte_to_col.len() - 1)];
                for c in start_col..=end_col {
                    map.entry((row_i32, c)).or_insert(colors.warning);
                }
            }
        }

        // ── 5. Info keywords ──────────────────────────────
        for kw in &["NOTICE", "INFO"] {
            for m in find_keyword(text, kw) {
                let start_col = byte_to_col[m];
                let end_col = byte_to_col[(m + kw.len()).min(byte_to_col.len() - 1)];
                for c in start_col..=end_col {
                    map.entry((row_i32, c)).or_insert(colors.info);
                }
            }
        }

        // ── 6. Debug keywords ─────────────────────────────
        for kw in &["DEBUG", "DBG", "TRACE"] {
            for m in find_keyword(text, kw) {
                let start_col = byte_to_col[m];
                let end_col = byte_to_col[(m + kw.len()).min(byte_to_col.len() - 1)];
                for c in start_col..=end_col {
                    map.entry((row_i32, c)).or_insert(colors.debug);
                }
            }
        }

        // ── 7. IP addresses ───────────────────────────────
        for m in find_ip_addresses(text) {
            let start_col = byte_to_col[m];
            let end_col = byte_to_col[(m + find_ip_len(&text[m..])).min(byte_to_col.len() - 1)];
            for c in start_col..=end_col {
                map.entry((row_i32, c)).or_insert(colors.network);
            }
        }

        // ── 8. URLs ───────────────────────────────────────
        for m in find_urls(text) {
            let url_len = find_url_len(&text[m..]);
            let start_col = byte_to_col[m];
            let end_col = byte_to_col[(m + url_len).min(byte_to_col.len() - 1)];
            for c in start_col..=end_col {
                map.entry((row_i32, c)).or_insert(colors.url);
            }
        }

        // ── 9. Port numbers ─────────────────────────────────────
        for m in find_ports(text) {
            let port_len = find_port_len(&text[m..]);
            let start_col = byte_to_col[m];
            let end_col = byte_to_col[(m + port_len).min(byte_to_col.len() - 1)];
            for c in start_col..=end_col {
                map.entry((row_i32, c)).or_insert(colors.port);
            }
        }
    }

    map
}

fn find_keyword(text: &str, keyword: &str) -> Vec<usize> {
    let mut positions = Vec::new();
    let mut start = 0;
    while let Some(pos) = text[start..].find(keyword) {
        let abs = start + pos;
        let before_ok = abs == 0
            || text.as_bytes()[abs - 1] == b' '
            || is_boundary(text.as_bytes()[abs - 1] as char);
        let after_pos = abs + keyword.len();
        let after_ok = after_pos >= text.len()
            || text.as_bytes()[after_pos] == b' '
            || is_boundary(text.as_bytes()[after_pos] as char);
        if before_ok && after_ok {
            positions.push(abs);
        }
        start = abs + keyword.len();
    }
    positions
}

fn find_ip_len(text: &str) -> usize {
    let bytes = text.as_bytes();
    let mut dots = 0u8;
    let mut digits = 0u8;
    let mut len = 0usize;

    for &b in bytes {
        match b {
            b'0'..=b'9' => {
                digits += 1;
                if digits > 3 {
                    return 0;
                }
            }
            b'.' => {
                if digits == 0 {
                    return 0;
                }
                dots += 1;
                if dots > 3 {
                    return 0;
                }
                digits = 0;
            }
            _ => break,
        }
        len += 1;
    }

    if dots == 3 && digits > 0 {
        len
    } else {
        0
    }
}

fn find_ip_addresses(text: &str) -> Vec<usize> {
    let mut positions = Vec::new();
    let bytes = text.as_bytes();
    let len = bytes.len();

    for i in 0..len {
        if bytes[i].is_ascii_digit()
            && (i == 0 || is_boundary(bytes[i - 1] as char))
        {
            let remaining = &text[i..];
            let ip_len = find_ip_len(remaining);
            if ip_len > 0 {
                let ip_str = &remaining[..ip_len];
                let valid = ip_str
                    .split('.')
                    .all(|octet| octet.parse::<u8>().is_ok());
                if valid {
                    positions.push(i);
                }
            }
        }
    }
    positions
}

fn find_urls(text: &str) -> Vec<usize> {
    let mut positions = Vec::new();
    let mut start = 0;
    while let Some(pos) = text[start..].find("http") {
        let abs = start + pos;
        let remaining = &text[abs..];
        if remaining.starts_with("https://") || remaining.starts_with("http://") {
            if abs == 0 || is_boundary(text.as_bytes()[abs - 1] as char) {
                positions.push(abs);
            }
        }
        start = abs + 4;
    }
    positions
}

fn find_url_len(text: &str) -> usize {
    text.find(|c: char| c.is_ascii_whitespace())
        .unwrap_or(text.len())
}

fn find_ports(text: &str) -> Vec<usize> {
    let mut positions = Vec::new();
    let bytes = text.as_bytes();
    let len = bytes.len();

    for i in 0..len {
        if bytes[i] == b':'
            && i + 1 < len
            && bytes[i + 1].is_ascii_digit()
            && (i == 0 || is_boundary(bytes[i - 1] as char) || bytes[i - 1] == b' ')
        {
            let mut j = i + 1;
            while j < len && bytes[j].is_ascii_digit() {
                j += 1;
            }
            let port_str = &text[i + 1..j];
            if let Ok(port) = port_str.parse::<u16>() {
                if port > 0 {
                    let after_ok = j >= len || is_boundary(bytes[j] as char);
                    if after_ok {
                        positions.push(i);
                    }
                }
            }
        }
    }
    positions
}

fn find_port_len(text: &str) -> usize {
    if !text.starts_with(':') {
        return 0;
    }
    let mut len = 1;
    for b in text.as_bytes()[1..].iter() {
        if b.is_ascii_digit() {
            len += 1;
        } else {
            break;
        }
    }
    len
}
