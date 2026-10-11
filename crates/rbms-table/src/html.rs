//! Finding a difficulty table's header from the web page that presents it.
//!
//! A table is published as an HTML page whose `<meta name="bmstable" content="…">` tag names the
//! header JSON, and that page's address is what every BMS tool is given. Only that one tag is read,
//! so this is a scan for `<meta …>` tags rather than an HTML parser: attribute order, quoting, case
//! and a self-closing slash all vary between sites, and a commented-out tag is not a tag.

/// The `name` a table page gives the meta tag that carries its header address.
const TABLE_META_NAME: &str = "bmstable";

const TAG_OPEN: char = '<';
const TAG_CLOSE: char = '>';
const TAG_NAME: &str = "meta";
const COMMENT_OPEN: &str = "<!--";
const COMMENT_CLOSE: &str = "-->";
const NAME_ATTRIBUTE: &str = "name";
const CONTENT_ATTRIBUTE: &str = "content";

/// The one character reference an address is written with inside an attribute, and what it stands for.
const ESCAPED_AMPERSAND: &str = "&amp;";
const AMPERSAND: &str = "&";

/// The header address a table page names, exactly as the page wrote it (usually relative to the
/// page). `None` when the page has no `bmstable` meta tag with an address in it.
pub(crate) fn table_header_address(html: &str) -> Option<String> {
    let mut rest = html;
    while let Some(at) = rest.find(TAG_OPEN) {
        rest = &rest[at..];
        if let Some(after_open) = rest.strip_prefix(COMMENT_OPEN) {
            rest = after_open.find(COMMENT_CLOSE).map_or("", |end| &after_open[end + COMMENT_CLOSE.len()..]);
            continue;
        }
        let after_bracket = &rest[TAG_OPEN.len_utf8()..];
        let Some(attributes) = meta_attributes(after_bracket) else {
            rest = after_bracket;
            continue;
        };
        let (found, after_tag) = read_meta(attributes);
        if found.is_some() {
            return found;
        }
        rest = after_tag;
    }
    None
}

/// What follows the tag name when `after_bracket` opens a `meta` tag, and `None` for any other tag.
fn meta_attributes(after_bracket: &str) -> Option<&str> {
    let name = after_bracket.get(..TAG_NAME.len())?;
    if !name.eq_ignore_ascii_case(TAG_NAME) {
        return None;
    }
    let attributes = &after_bracket[TAG_NAME.len()..];
    attributes.starts_with(|next: char| next.is_whitespace() || next == '/' || next == TAG_CLOSE).then_some(attributes)
}

/// Read one meta tag's attributes: the table address when the tag is the table's, and what follows the tag.
fn read_meta(mut rest: &str) -> (Option<String>, &str) {
    let mut name = None;
    let mut content = None;
    loop {
        rest = rest.trim_start_matches(|skipped: char| skipped.is_whitespace() || skipped == '/');
        if rest.is_empty() {
            break;
        }
        if let Some(after_tag) = rest.strip_prefix(TAG_CLOSE) {
            rest = after_tag;
            break;
        }
        let key_end = rest.find(|end: char| end.is_whitespace() || end == '=' || end == TAG_CLOSE || end == '/').unwrap_or(rest.len());
        let key = &rest[..key_end];
        rest = rest[key_end..].trim_start();
        let Some(after_equals) = rest.strip_prefix('=') else {
            continue;
        };
        let (value, after_value) = attribute_value(after_equals.trim_start());
        rest = after_value;
        if key.eq_ignore_ascii_case(NAME_ATTRIBUTE) {
            name = Some(value);
        } else if key.eq_ignore_ascii_case(CONTENT_ATTRIBUTE) {
            content = Some(value);
        }
    }
    let is_table = name.is_some_and(|name| name.trim().eq_ignore_ascii_case(TABLE_META_NAME));
    let address = content.map(|content| content.trim().replace(ESCAPED_AMPERSAND, AMPERSAND)).filter(|address| !address.is_empty());
    (if is_table { address } else { None }, rest)
}

/// One attribute value — quoted either way, or bare — and what follows it.
fn attribute_value(rest: &str) -> (&str, &str) {
    for quote in ['"', '\''] {
        if let Some(quoted) = rest.strip_prefix(quote) {
            return match quoted.find(quote) {
                Some(end) => (&quoted[..end], &quoted[end + quote.len_utf8()..]),
                None => ("", ""),
            };
        }
    }
    let end = rest.find(|end: char| end.is_whitespace() || end == TAG_CLOSE).unwrap_or(rest.len());
    (&rest[..end], &rest[end..])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn address(html: &str) -> Option<String> {
        table_header_address(html)
    }

    #[test]
    fn the_usual_tag_names_the_header() {
        let page = "<!doctype html>\r\n<html>\r\n  <head>\r\n    <meta charset=\"utf-8\" />\r\n    <meta name=\"ROBOTS\" content=\"NOINDEX, NOFOLLOW\" />\r\n    \
                    <meta name=\"bmstable\" content=\"header.json\" />\r\n  </head><body>table</body></html>";
        assert_eq!(address(page).as_deref(), Some("header.json"));
    }

    #[test]
    fn attribute_order_case_quoting_and_the_closing_slash_do_not_matter() {
        assert_eq!(address(r#"<META CONTENT="h.json" NAME="BmsTable">"#).as_deref(), Some("h.json"), "content first, upper case, no slash");
        assert_eq!(address("<meta name='bmstable' content='./h.json'/>").as_deref(), Some("./h.json"), "single quotes, slash without a space");
        assert_eq!(address("<meta name=bmstable content=h.json>").as_deref(), Some("h.json"), "bare values");
        assert_eq!(address("<meta\n  name = \"bmstable\"\n  content = \"h.json\"\n>").as_deref(), Some("h.json"), "spread over lines, spaced equals");
    }

    #[test]
    fn an_address_is_taken_as_written_with_its_query_unescaped() {
        assert_eq!(
            address(r#"<meta name="bmstable" content=" https://x.test/t/header.json?a=1&amp;b=2 ">"#).as_deref(),
            Some("https://x.test/t/header.json?a=1&b=2")
        );
        assert_eq!(address(r#"<meta name="bmstable" content="a>b.json">"#).as_deref(), Some("a>b.json"), "a quoted value may hold the closing bracket");
    }

    #[test]
    fn other_meta_tags_and_other_tags_are_passed_over() {
        let page = r#"<metadata name="bmstable" content="no.json"></metadata><meta name="bmstable-alt" content="alt.json"><meta property="og:title" content="x"><meta name="bmstable" content="yes.json">"#;
        assert_eq!(address(page).as_deref(), Some("yes.json"));
    }

    #[test]
    fn a_commented_out_tag_is_not_the_tag() {
        let page = r#"<!-- <meta name="bmstable" content="old.json"> --><meta name="bmstable" content="new.json">"#;
        assert_eq!(address(page).as_deref(), Some("new.json"));
        assert_eq!(address(r#"<!-- <meta name="bmstable" content="old.json">"#), None, "an unclosed comment hides the rest of the page");
    }

    #[test]
    fn a_page_without_the_tag_or_without_an_address_in_it_names_nothing() {
        assert_eq!(address(""), None);
        assert_eq!(address("<html><head><title>not a table</title></head></html>"), None);
        assert_eq!(address(r#"<meta name="bmstable">"#), None, "no content");
        assert_eq!(address(r#"<meta name="bmstable" content="  ">"#), None, "blank content");
        assert_eq!(address(r#"<meta name="bmstable" content="h.json"#), None, "an unterminated value is not an address");
    }

    #[test]
    fn text_in_another_encoding_around_the_tag_does_not_hide_it() {
        let bytes = [b"<title>\x94\xad\x8b\xb6</title>".as_slice(), br#"<meta name="bmstable" content="header.json">"#.as_slice()].concat();
        assert_eq!(address(&String::from_utf8_lossy(&bytes)).as_deref(), Some("header.json"));
    }
}
