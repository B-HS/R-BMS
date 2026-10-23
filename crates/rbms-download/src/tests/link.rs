use crate::{LinkKind, MAX_BROWSABLE_URL_CHARS, browsable_url, classify_link};

#[test]
fn stella_uploads_and_zip_paths_are_direct_archives() {
    for link in [
        "https://stellabms.xyz/upload/3f2a9c1d",
        "http://stellabms.xyz/upload/song.zip",
        "https://example.test/files/Song.ZIP",
        "https://example.test/dl/pack.zip?token=1",
        "  https://example.test/a/b.zip  ",
    ] {
        assert_eq!(classify_link(link), LinkKind::DirectZip, "{link}");
    }
}

#[test]
fn only_a_stella_upload_with_something_after_it_is_an_archive_without_a_zip_name() {
    for link in [
        "https://stellabms.xyz/upload/",
        "https://stellabms.xyz/other/3f2a9c1d",
        "https://www.stellabms.xyz/upload/3f2a9c1d",
        "https://not-stellabms.xyz/upload/3f2a9c1d",
    ] {
        assert_eq!(classify_link(link), LinkKind::Browser, "{link}");
    }
}

#[test]
fn rar_7z_lzh_single_charts_pages_and_cloud_links_go_to_the_browser() {
    for link in [
        "https://example.test/pack.rar",
        "https://example.test/pack.7z",
        "https://example.test/pack.lzh",
        "https://example.test/pack.lha",
        "https://example.test/chart.bms",
        "https://example.test/chart.bme",
        "https://example.test/page.html",
        "https://example.test/",
        "https://example.test",
        "https://drive.google.com/file/d/1AbC/view?usp=sharing",
        "https://www.dropbox.com/s/abc/song?dl=0",
    ] {
        assert_eq!(classify_link(link), LinkKind::Browser, "{link}");
    }
}

#[test]
fn a_query_does_not_make_a_page_a_zip() {
    for link in ["https://example.test/page.html?f=a.zip", "https://example.test/page.html#a.zip", "https://example.test/dl?file=a.zip"] {
        assert_eq!(classify_link(link), LinkKind::Browser, "{link}");
    }
}

#[test]
fn a_link_that_is_not_http_is_unusable() {
    for link in [
        "",
        "   ",
        "ftp://example.test/a.zip",
        "file:///tmp/a.zip",
        "javascript:alert(1)",
        "data:text/plain,hi",
        "mailto:someone@example.test",
        "example.test/a.zip",
        "//example.test/a.zip",
        "http://",
    ] {
        assert_eq!(classify_link(link), LinkKind::Unusable, "{link:?}");
    }
}

#[test]
fn only_http_and_https_with_a_host_are_browsable() {
    assert_eq!(browsable_url("https://example.test/a").as_deref(), Some("https://example.test/a"));
    assert_eq!(browsable_url("http://Example.TEST").as_deref(), Some("http://example.test/"));
    assert_eq!(browsable_url("https://example.test:8080/p?q=1&r=%E3%81%82#f").as_deref(), Some("https://example.test:8080/p?q=1&r=%E3%81%82#f"));
    assert_eq!(browsable_url("  https://example.test/a  ").as_deref(), Some("https://example.test/a"));
    for refused in ["ftp://example.test/", "//example.test/a", "example.test", "http://", "https://", ""] {
        assert_eq!(browsable_url(refused), None, "{refused:?}");
    }
}

#[test]
fn control_characters_white_space_and_overlong_urls_are_refused() {
    for refused in [
        "https://example.test/a b",
        "https://example.test/a\tb",
        "https://example.test/a\nb",
        "https://example.test/a\u{7}b",
        "https://example.test/a\u{0}b",
        "https://example.test/a\u{3000}b",
        "https://example.test/a\u{85}b",
    ] {
        assert_eq!(browsable_url(refused), None, "{refused:?}");
    }
    let prefix = "https://example.test/";
    let at_the_limit = format!("{prefix}{}", "a".repeat(MAX_BROWSABLE_URL_CHARS - prefix.len()));
    assert_eq!(browsable_url(&at_the_limit), Some(at_the_limit.clone()));
    assert_eq!(browsable_url(&format!("{at_the_limit}a")), None);
}

#[test]
fn an_address_that_grows_past_the_limit_once_normalised_is_refused() {
    let short_but_wide = format!("https://example.test/{}", "\u{3042}".repeat(MAX_BROWSABLE_URL_CHARS / 4));
    assert!(short_but_wide.chars().count() < MAX_BROWSABLE_URL_CHARS);
    assert_eq!(browsable_url(&short_but_wide), None);
}

#[test]
fn javascript_file_and_data_urls_are_refused() {
    for refused in [
        "javascript:alert(1)",
        "JAVASCRIPT:alert(1)",
        "file:///etc/passwd",
        "data:text/html,<script>alert(1)</script>",
        "vbscript:msgbox(1)",
        "about:blank",
        "blob:https://example.test/6a1c",
    ] {
        assert_eq!(browsable_url(refused), None, "{refused:?}");
    }
}

#[test]
fn a_browsable_url_never_starts_with_a_dash() {
    for raw in ["-h", "--version", "-oProxyCommand=touch x", "-https://example.test/", " --help", "https://example.test/-x", "http://-a.test/"] {
        if let Some(url) = browsable_url(raw) {
            assert!(url.starts_with("http"), "{raw:?} became {url:?}");
        }
    }
    assert_eq!(browsable_url("--version"), None);
    assert_eq!(browsable_url("-https://example.test/"), None);
}
