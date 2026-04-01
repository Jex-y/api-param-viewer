/// Strip HTML tags from a string.
///
/// If `inline` is true, tags are simply removed (useful for table cells).
/// If false, block tags like `</p>` and `<br/>` insert newlines.
pub fn strip_html(s: &str, inline: bool) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    let mut tag_buf = String::new();

    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '<' {
            in_tag = true;
            tag_buf.clear();
        } else if c == '>' && in_tag {
            in_tag = false;
            if !inline {
                let tag_lower = tag_buf.to_lowercase();
                let tag_name = tag_lower.split_whitespace().next().unwrap_or("");
                match tag_name {
                    "br" | "br/" | "br /" => out.push('\n'),
                    "/p" => out.push_str("\n\n"),
                    "hr" | "hr/" => out.push_str("\n---\n"),
                    "/li" => out.push('\n'),
                    _ => {}
                }
            }
        } else if in_tag {
            tag_buf.push(c);
        } else if c == '&' {
            let mut entity = String::new();
            for ec in chars.by_ref() {
                if ec == ';' {
                    break;
                }
                entity.push(ec);
                if entity.len() > 10 {
                    break;
                }
            }
            match entity.as_str() {
                "amp" => out.push('&'),
                "lt" => out.push('<'),
                "gt" => out.push('>'),
                "quot" => out.push('"'),
                "apos" => out.push('\''),
                "nbsp" => out.push(' '),
                _ => {
                    out.push('&');
                    out.push_str(&entity);
                    out.push(';');
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}
