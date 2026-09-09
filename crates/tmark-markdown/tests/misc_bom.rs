use pretty_assertions::assert_eq;
use tmark_markdown::to_html;

#[test]
fn bom() {
    assert_eq!(to_html("\u{FEFF}"), "", "should ignore just a bom");

    assert_eq!(
        to_html("\u{FEFF}# hea\u{FEFF}ding"),
        "<h1>hea\u{FEFF}ding</h1>",
        "should ignore a bom"
    );
}
