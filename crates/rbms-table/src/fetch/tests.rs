//! The chain from a registered address to a table, against a site that lives in the test.
//!
//! The pages below have the structure of a table site as it is served — a page whose meta tag names
//! a relative header, a header naming a relative body, entries carrying the usual fields — with
//! contents made up for the test.

use std::collections::HashMap;

use super::*;

const PAGE_URL: &str = "https://tables.test/table/archive/sample1/";
const HEADER_URL: &str = "https://tables.test/table/archive/sample1/header.json";
const BODY_URL: &str = "https://tables.test/table/archive/sample1/data.json";

const PAGE: &str = "<!doctype html>\r\n<html>\r\n  <head>\r\n    <meta charset=\"utf-8\" />\r\n    <meta name=\"ROBOTS\" content=\"NOINDEX, NOFOLLOW\" />\r\n    \
                    <meta name=\"bmstable\" content=\"header.json\" />\r\n    <meta http-equiv=\"X-UA-Compatible\" content=\"IE=edge\" />\r\n    \
                    <title>Sample table</title>\r\n  </head>\r\n  <body><div id=\"app\"></div></body>\r\n</html>\r\n";

const HEADER: &str = "{\r\n  \"data_url\": \"data.json\",\r\n  \"last_update\": \"2001.02.03\",\r\n  \
                      \"level_order\": [\"1\", \"2\", \"3\", \"???\"],\r\n  \"name\": \"\u{30b5}\u{30f3}\u{30d7}\u{30eb}\u{96e3}\u{6613}\u{5ea6}\u{8868}\",\r\n  \
                      \"original_url\": \"http://elsewhere.test/sample.html\",\r\n  \"symbol\": \"\u{2605}\"\r\n}\r\n";

const BODY: &str = "[\r\n  {\"artist\": \"someone\", \"comment\": \"\", \"level\": \"2\", \"lr2_bmsid\": \"101\", \"md5\": \"0000000000000000000000000000000a\", \
                    \"name_diff\": \"\", \"title\": \"first [another]\", \"url\": \"http://elsewhere.test/a\", \"url_diff\": \"\"},\r\n  \
                    {\"artist\": \"someone else\", \"comment\": \"hard\", \"level\": \"1\", \"lr2_bmsid\": \"102\", \"md5\": \"0000000000000000000000000000000B\", \
                    \"name_diff\": \"x\", \"title\": \"second\", \"url\": \"\", \"url_diff\": \"http://elsewhere.test/b\"},\r\n  \
                    {\"artist\": \"a third\", \"comment\": \"\", \"level\": \"???\", \"lr2_bmsid\": \"103\", \"md5\": \"0000000000000000000000000000000c\", \
                    \"name_diff\": \"\", \"title\": \"third\", \"url\": \"\", \"url_diff\": \"\"}\r\n]\r\n";

/// A site: what each address answers with, and — for an address that redirects — where the answer
/// finally came from. Every address asked is recorded, in order.
#[derive(Default)]
struct Site {
    pages: HashMap<String, (Vec<u8>, String)>,
    asked: Vec<String>,
}

impl Site {
    fn serving(mut self, url: &str, body: &str) -> Site {
        self.pages.insert(url.to_string(), (body.as_bytes().to_vec(), url.to_string()));
        self
    }

    fn redirecting(mut self, from: &str, to: &str) -> Site {
        let (bytes, landed) = self.pages.get(to).cloned().expect("the redirect target is served");
        self.pages.insert(from.to_string(), (bytes, landed));
        self
    }

    fn sample() -> Site {
        Site::default().serving(PAGE_URL, PAGE).serving(HEADER_URL, HEADER).serving(BODY_URL, BODY)
    }

    fn fetch(&mut self, address: &str) -> Result<DifficultyTable, TableError> {
        let (pages, asked) = (&self.pages, &mut self.asked);
        fetch_with(address, &mut |url| {
            asked.push(url.to_string());
            match pages.get(url) {
                Some((bytes, landed)) => Ok(Fetched { bytes: bytes.clone(), url: landed.clone() }),
                None => Err(TableError::HttpStatus(reqwest::StatusCode::NOT_FOUND)),
            }
        })
    }

    fn failure(&mut self, address: &str) -> String {
        match self.fetch(address) {
            Ok(_) => panic!("{address} is not a table"),
            Err(error) => error.to_string(),
        }
    }
}

fn assert_is_the_sample(table: &DifficultyTable) {
    assert_eq!(table.name, "\u{30b5}\u{30f3}\u{30d7}\u{30eb}\u{96e3}\u{6613}\u{5ea6}\u{8868}");
    assert_eq!(table.symbol, "\u{2605}");
    assert_eq!(table.level_order, vec!["1", "2", "3", "???"]);
    assert_eq!(table.entries.len(), 3);
    assert_eq!(table.skipped, 0);
}

/// The address a user is given for a table is its web page, ending in a slash: the page's meta tag
/// names the header beside it, and the header names the body beside itself.
#[test]
fn a_table_page_address_leads_through_its_meta_tag_to_the_header_and_the_body() {
    let mut site = Site::sample();
    let table = site.fetch(PAGE_URL).expect("a table page is a table");
    assert_is_the_sample(&table);
    assert_eq!(site.asked, vec![PAGE_URL, HEADER_URL, BODY_URL], "one request each for the page, the header and the body");

    let owned = table.match_levels(["0000000000000000000000000000000b", "ffffffffffffffffffffffffffffffff", "0000000000000000000000000000000A"].into_iter());
    assert_eq!(
        owned,
        vec![("1".to_string(), vec![0]), ("2".to_string(), vec![2]), ("???".to_string(), Vec::new())],
        "levels in the header's order; the level the table has no chart in is not a folder, the level nothing is owned in is an empty one"
    );
}

#[test]
fn a_header_address_and_a_body_address_are_tables_too() {
    let mut site = Site::sample();
    assert_is_the_sample(&site.fetch(HEADER_URL).expect("a header is a table"));
    assert_eq!(site.asked, vec![HEADER_URL, BODY_URL]);

    let mut site = Site::sample();
    assert_is_the_sample(&site.fetch(BODY_URL).expect("a body is a table"));
    assert_eq!(site.asked, vec![BODY_URL, HEADER_URL], "the header beside a body is asked for, for the name and the level order");

    let mut alone = Site::default().serving(BODY_URL, BODY);
    let table = alone.fetch(BODY_URL).expect("a body with no header beside it is still a table");
    assert_eq!(table.name, "");
    assert_eq!(table.level_order, vec!["1", "2", "???"], "with no header the order comes from the entries");
}

/// A page reached through a redirect names its header relative to where it ended up, which is
/// what an address typed without its closing slash depends on.
#[test]
fn relative_addresses_are_relative_to_where_the_answer_came_from() {
    let bare = "https://tables.test/table/archive/sample1";
    let mut site = Site::sample().redirecting(bare, PAGE_URL);
    assert_is_the_sample(&site.fetch(bare).expect("the redirect is followed"));
    assert_eq!(site.asked, vec![bare, HEADER_URL, BODY_URL]);

    let moved_header = "https://cdn.test/v2/meta/header.json";
    let mut site = Site::default()
        .serving(PAGE_URL, &PAGE.replace("content=\"header.json\"", &format!("content=\"{moved_header}\"")))
        .serving(moved_header, &HEADER.replace("\"data.json\"", "\"../body/data.json\""))
        .serving("https://cdn.test/v2/body/data.json", BODY);
    assert_is_the_sample(&site.fetch(PAGE_URL).expect("the body is found from the header, not from the page"));
    assert_eq!(site.asked, vec![PAGE_URL, moved_header, "https://cdn.test/v2/body/data.json"]);

    let mut site = Site::default()
        .serving(PAGE_URL, &PAGE.replace("content=\"header.json\"", "content=\"/shared/header.json\""))
        .serving("https://tables.test/shared/header.json", HEADER);
    let failure = site.failure(PAGE_URL);
    assert_eq!(failure, "body https://tables.test/shared/data.json: HTTP 404 Not Found", "a root-relative header, and the body looked for beside it");
}

#[test]
fn a_table_split_over_several_bodies_is_the_sum_of_them() {
    let mut site = Site::default()
        .serving(HEADER_URL, r#"{"name":"split","symbol":"s","data_url":["a.json","b.json"]}"#)
        .serving("https://tables.test/table/archive/sample1/a.json", r#"[{"md5":"aa","level":1},{"level":2}]"#)
        .serving("https://tables.test/table/archive/sample1/b.json", r#"[{"md5":"bb","level":"2"},"note"]"#);
    let table = site.fetch(HEADER_URL).expect("both bodies are read");
    assert_eq!(table.entries.iter().map(|e| e.md5.as_str()).collect::<Vec<_>>(), vec!["aa", "bb"]);
    assert_eq!(table.level_order, vec!["1", "2"], "a level written as a number is the level written as text");
    assert_eq!(table.skipped, 2, "the items naming no chart are counted across the bodies");
}

#[test]
fn a_byte_order_mark_in_front_of_the_json_does_not_turn_it_into_a_page() {
    let marked = |text: &str| format!("\u{feff}{text}");
    let mut site = Site::default().serving(PAGE_URL, PAGE).serving(HEADER_URL, &marked(HEADER)).serving(BODY_URL, &marked(BODY));
    assert_is_the_sample(&site.fetch(PAGE_URL).expect("marked JSON is JSON"));
    assert_is_the_sample(&site.fetch(HEADER_URL).expect("a marked header is recognised as a header"));
    assert_is_the_sample(&site.fetch(BODY_URL).expect("a marked body is recognised as a body"));
}

/// Each failure says which step failed and at which address, since "the table did not load" leaves
/// the user with nothing to act on.
#[test]
fn a_failure_names_the_step_the_address_and_the_reason() {
    assert_eq!(Site::default().failure(PAGE_URL), format!("address {PAGE_URL}: HTTP 404 Not Found"));

    let mut no_tag = Site::default().serving(PAGE_URL, "<html><head><title>not a table</title></head></html>");
    assert_eq!(no_tag.failure(PAGE_URL), format!("address {PAGE_URL}: the page has no <meta name=\"bmstable\"> tag and is not table JSON"));
    assert_eq!(no_tag.asked, vec![PAGE_URL], "nothing is guessed at once the page names no header");

    assert_eq!(Site::default().serving(PAGE_URL, PAGE).failure(PAGE_URL), format!("header {HEADER_URL}: HTTP 404 Not Found"));

    let mut page_for_header = Site::default().serving(PAGE_URL, PAGE).serving(HEADER_URL, "{\"name\": ");
    assert!(page_for_header.failure(PAGE_URL).starts_with(&format!("header {HEADER_URL}: header parse: ")), "a header that is not JSON");

    let mut no_body_named = Site::default().serving(HEADER_URL, r#"{"name":"n","symbol":"s"}"#);
    assert_eq!(no_body_named.failure(HEADER_URL), format!("header {HEADER_URL}: header.json has no data_url"));

    assert_eq!(Site::default().serving(PAGE_URL, PAGE).serving(HEADER_URL, HEADER).failure(PAGE_URL), format!("body {BODY_URL}: HTTP 404 Not Found"));

    let mut bad_body = Site::default().serving(HEADER_URL, HEADER).serving(BODY_URL, "[{\"md5\": ");
    assert!(bad_body.failure(HEADER_URL).starts_with(&format!("body {BODY_URL}: body parse: ")), "a body that is not JSON");

    let mut object_for_body = Site::default().serving(HEADER_URL, HEADER).serving(BODY_URL, r#"{"md5":"aa"}"#);
    assert!(object_for_body.failure(HEADER_URL).starts_with(&format!("body {BODY_URL}: body parse: ")), "a body that is not a list");
}

#[test]
fn what_an_answer_is_comes_from_its_first_meaningful_byte() {
    assert!(matches!(shape_of(b"  \r\n[{}]"), Shape::Body));
    assert!(matches!(shape_of(b"\xEF\xBB\xBF{\"name\":1}"), Shape::Header));
    assert!(matches!(shape_of(b"<!doctype html>"), Shape::Page));
    assert!(matches!(shape_of(b""), Shape::Page));
}

/// An answer that arrives a byte at a time and never ends, which is what keeps every single read
/// inside the time a server may stay silent for.
struct Trickle;

impl Read for Trickle {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        match buffer.first_mut() {
            Some(first) => {
                *first = b' ';
                Ok(1)
            }
            None => Ok(0),
        }
    }
}

/// The time a table load has is spent across its reads, however promptly each of them answers: an
/// answer that keeps trickling in is given up on when the time is, not when it has grown as large as
/// an answer may be.
#[test]
fn an_answer_that_trickles_in_without_end_is_given_up_on_when_the_time_is_up() {
    const READS_THE_TIME_LASTS: u32 = 40;
    let mut reads_left = READS_THE_TIME_LASTS;
    let mut asked = 0;
    let outcome = read_within(Trickle, &mut || {
        asked += 1;
        reads_left = reads_left.saturating_sub(1);
        Duration::from_secs(u64::from(reads_left))
    });
    match outcome {
        Err(TableError::TimedOut { limit }) => assert_eq!(limit, LOAD_TIMEOUT),
        Err(other) => panic!("the trickle ended as something other than the time running out: {other}"),
        Ok(bytes) => panic!("a trickle without end was read to an end of {} bytes", bytes.len()),
    }
    assert_eq!(asked, READS_THE_TIME_LASTS, "the time left is asked for before each read and the reading stops at the first that has none");
}

#[test]
fn an_answer_that_ends_in_time_is_read_whole_and_one_too_large_is_refused() {
    let whole = read_within(&b"[1, 2, 3]"[..], &mut || LOAD_TIMEOUT).expect("an answer that ends is an answer");
    assert_eq!(whole, b"[1, 2, 3]");
    let too_large = read_within(std::io::repeat(b' '), &mut || LOAD_TIMEOUT);
    assert!(
        matches!(too_large, Err(TableError::TooLarge { limit: MAX_RESPONSE_BYTES })),
        "an answer that never ends but arrives at once is refused for its size"
    );
}
