pub mod comment;
pub mod item;
pub mod iterations;
pub mod link;
pub mod sprint;

/// Crude HTML → text: drop tags, decode common entities.
pub(crate) fn strip_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_html_removes_tags() {
        assert_eq!(strip_html("<p>hello <b>world</b></p>"), "hello world");
    }

    #[test]
    fn strip_html_decodes_entities() {
        assert_eq!(strip_html("a &amp; b &lt; c &gt; d"), "a & b < c > d");
    }

    #[test]
    fn strip_html_nbsp_and_quot() {
        assert_eq!(strip_html("&nbsp;&quot;hi&quot;"), " \"hi\"");
    }

    #[test]
    fn strip_html_plain_passthrough() {
        assert_eq!(strip_html("no tags"), "no tags");
    }

    #[test]
    fn strip_html_nested() {
        assert_eq!(strip_html("<div><p>text</p></div>"), "text");
    }
}
