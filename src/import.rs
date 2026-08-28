use std::path::Path;

use chrono::DateTime;

use crate::frontmatter::{self, Frontmatter};
use crate::slug::slug;
use crate::store::{Store, StoreError};

pub struct ParsedBookmark {
    pub url: String,
    pub title: String,
    pub tags: Vec<String>,
    pub added: Option<String>,
}

pub struct ImportSummary {
    pub imported: usize,
}

#[derive(Debug)]
pub enum ImportError {
    Read(std::io::Error),
    Store(StoreError),
    Frontmatter(frontmatter::FrontmatterError),
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportError::Read(e) => write!(f, "reading export: {e}"),
            ImportError::Store(e) => write!(f, "{e}"),
            ImportError::Frontmatter(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ImportError {}

pub fn import(store: &Store, file: &Path) -> Result<ImportSummary, ImportError> {
    let html = std::fs::read_to_string(file).map_err(ImportError::Read)?;
    let parsed = parse_netscape(&html);

    for bookmark in &parsed {
        let fm = Frontmatter::imported(
            bookmark.url.clone(),
            bookmark.title.clone(),
            bookmark.tags.clone(),
            bookmark.added.clone(),
        );
        let content = frontmatter::serialize(&fm, "").map_err(ImportError::Frontmatter)?;
        store
            .write_bookmark(&slug(&bookmark.title), &content)
            .map_err(ImportError::Store)?;
    }

    Ok(ImportSummary {
        imported: parsed.len(),
    })
}

enum Token {
    OpenDl,
    CloseDl,
    Heading,
    Anchor,
}

pub fn parse_netscape(html: &str) -> Vec<ParsedBookmark> {
    let lower = html.to_ascii_lowercase();
    let mut bookmarks = Vec::new();
    let mut folders: Vec<Option<String>> = Vec::new();
    let mut pending: Option<String> = None;
    let mut cursor = 0;

    while cursor < lower.len() {
        let rest = &lower[cursor..];
        let next = [
            rest.find("</dl").map(|i| (i, Token::CloseDl)),
            rest.find("<dl").map(|i| (i, Token::OpenDl)),
            rest.find("<h3").map(|i| (i, Token::Heading)),
            rest.find("<a ").map(|i| (i, Token::Anchor)),
        ]
        .into_iter()
        .flatten()
        .min_by_key(|(i, _)| *i);

        let Some((rel, token)) = next else { break };
        let pos = cursor + rel;

        match token {
            Token::OpenDl => {
                folders.push(pending.take());
                cursor = pos + "<dl".len();
            }
            Token::CloseDl => {
                folders.pop();
                cursor = pos + "</dl".len();
            }
            Token::Heading => {
                let Some((text, end)) = tag_content(html, &lower, pos, "</h3>") else {
                    break;
                };
                pending = Some(text.trim().to_string());
                cursor = end;
            }
            Token::Anchor => {
                let Some(close_rel) = lower[pos..].find('>') else {
                    break;
                };
                let tag_end = pos + close_rel;
                let open_tag = &html[pos..tag_end];

                let Some((text, end)) = tag_content(html, &lower, tag_end, "</a>") else {
                    break;
                };

                if let Some(url) = href_value(open_tag) {
                    let title = resolve_title(text, &url);
                    bookmarks.push(ParsedBookmark {
                        url,
                        title,
                        tags: folder_tags(&folders),
                        added: added_value(open_tag),
                    });
                }
                cursor = end;
            }
        }
    }

    bookmarks
}

fn tag_content<'a>(
    html: &'a str,
    lower: &str,
    open_tag_start: usize,
    close: &str,
) -> Option<(&'a str, usize)> {
    let content_rel = lower[open_tag_start..].find('>')?;
    let content_start = open_tag_start + content_rel + 1;
    let close_rel = lower[content_start..].find(close)?;
    let content_end = content_start + close_rel;
    Some((&html[content_start..content_end], content_end + close.len()))
}

fn folder_tags(folders: &[Option<String>]) -> Vec<String> {
    let mut tags: Vec<String> = Vec::new();
    for name in folders.iter().flatten() {
        let trimmed = name.trim();
        if trimmed.is_empty() || tags.iter().any(|t| t == trimmed) {
            continue;
        }
        tags.push(trimmed.to_string());
    }
    tags
}

fn href_value(open_tag: &str) -> Option<String> {
    attr_value(open_tag, "href=")
}

fn added_value(open_tag: &str) -> Option<String> {
    let secs: i64 = attr_value(open_tag, "add_date=")?.trim().parse().ok()?;
    let dt = DateTime::from_timestamp(secs, 0)?;
    Some(dt.to_rfc3339())
}

fn attr_value(open_tag: &str, attr: &str) -> Option<String> {
    let lower = open_tag.to_ascii_lowercase();
    let at = lower.find(attr)?;
    let after = at + attr.len();
    let value = open_tag[after..].strip_prefix('"')?;
    let end = value.find('"')?;
    Some(value[..end].to_string())
}

fn resolve_title(raw: &str, url: &str) -> String {
    let decoded = decode_entities(raw);
    let trimmed = decoded.trim();
    if trimmed.is_empty() {
        url.to_string()
    } else {
        trimmed.to_string()
    }
}

fn decode_entities(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let after = &rest[amp..];
        match decode_one_entity(after) {
            Some((ch, len)) => {
                out.push(ch);
                rest = &after[len..];
            }
            None => {
                out.push('&');
                rest = &after[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn decode_one_entity(s: &str) -> Option<(char, usize)> {
    let semi = s.find(';').filter(|&i| i <= 12)?;
    let body = &s[1..semi];
    let ch = match body.strip_prefix('#') {
        Some(num) => {
            let code = match num.strip_prefix(['x', 'X']) {
                Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                None => num.parse().ok()?,
            };
            char::from_u32(code)?
        }
        None => match body {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" => '\'',
            "nbsp" => '\u{00A0}',
            _ => return None,
        },
    };
    Some((ch, semi + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../tests/fixtures/flat-export.html");
    const NESTED: &str = include_str!("../tests/fixtures/nested-export.html");

    fn find<'a>(bookmarks: &'a [ParsedBookmark], url: &str) -> &'a ParsedBookmark {
        bookmarks
            .iter()
            .find(|b| b.url == url)
            .unwrap_or_else(|| panic!("no bookmark for {url}"))
    }

    #[test]
    fn parses_one_bookmark_per_link() {
        let bookmarks = parse_netscape(FIXTURE);
        assert_eq!(bookmarks.len(), 3);
    }

    #[test]
    fn keeps_href_verbatim_with_tracking_params() {
        let bookmarks = parse_netscape(FIXTURE);
        assert_eq!(
            bookmarks[0].url,
            "https://example.com/a?utm_source=news&id=7"
        );
    }

    #[test]
    fn title_is_the_link_text() {
        let bookmarks = parse_netscape(FIXTURE);
        assert_eq!(bookmarks[0].title, "Example Page");
        assert_eq!(bookmarks[1].title, "The Rust Programming Language");
    }

    #[test]
    fn add_date_maps_to_rfc3339_added() {
        let bookmarks = parse_netscape(FIXTURE);
        assert_eq!(
            bookmarks[0].added.as_deref(),
            Some("2020-09-13T12:26:40+00:00")
        );
    }

    #[test]
    fn missing_add_date_leaves_added_unset() {
        let html = r#"<DL><p>
            <DT><A HREF="https://no-date.example.com/">No Date</A>
        </DL><p>"#;
        let bookmarks = parse_netscape(html);
        assert_eq!(bookmarks[0].added, None);
    }

    #[test]
    fn empty_link_text_falls_back_to_url() {
        let html = r#"<DL><p>
            <DT><A HREF="https://blank.example.com/">   </A>
        </DL><p>"#;
        let bookmarks = parse_netscape(html);
        assert_eq!(bookmarks[0].title, "https://blank.example.com/");
    }

    #[test]
    fn link_text_is_html_entity_decoded() {
        let html = r#"<DL><p>
            <DT><A HREF="https://ent.example.com/">Ben &amp; Jerry&#39;s &lt;3</A>
        </DL><p>"#;
        let bookmarks = parse_netscape(html);
        assert_eq!(bookmarks[0].title, "Ben & Jerry's <3");
    }

    #[test]
    fn decodes_numeric_and_named_entities() {
        assert_eq!(decode_entities("a &gt; b &quot;c&quot;"), "a > b \"c\"");
        assert_eq!(
            decode_entities("It&#x2019;s &#8212; done"),
            "It\u{2019}s \u{2014} done"
        );
        assert_eq!(decode_entities("nbsp&nbsp;here"), "nbsp\u{00A0}here");
    }

    #[test]
    fn leaves_unknown_entities_intact() {
        assert_eq!(decode_entities("Tom &copy; & Jerry"), "Tom &copy; & Jerry");
    }

    #[test]
    fn bare_ampersand_before_multibyte_does_not_panic() {
        assert_eq!(decode_entities("R&café ☕ résumé"), "R&café ☕ résumé");
    }

    #[test]
    fn empty_input_yields_no_bookmarks() {
        assert!(parse_netscape("").is_empty());
    }

    #[test]
    fn flat_bookmarks_have_no_folder_tags() {
        let bookmarks = parse_netscape(FIXTURE);
        assert!(bookmarks.iter().all(|b| b.tags.is_empty()));
    }

    #[test]
    fn top_level_bookmark_has_no_tags() {
        let bookmarks = parse_netscape(NESTED);
        assert!(find(&bookmarks, "https://top.example.com/").tags.is_empty());
    }

    #[test]
    fn nested_bookmark_tags_are_outer_to_inner() {
        let bookmarks = parse_netscape(NESTED);
        assert_eq!(
            find(&bookmarks, "https://reading.example.com/").tags,
            vec!["Work".to_string(), "Reading".to_string()]
        );
    }

    #[test]
    fn single_folder_yields_one_tag() {
        let bookmarks = parse_netscape(NESTED);
        assert_eq!(
            find(&bookmarks, "https://work.example.com/").tags,
            vec!["Work".to_string()]
        );
        assert_eq!(
            find(&bookmarks, "https://personal.example.com/").tags,
            vec!["Personal".to_string()]
        );
    }

    #[test]
    fn deeply_nested_yields_one_tag_per_segment() {
        let bookmarks = parse_netscape(NESTED);
        assert_eq!(
            find(&bookmarks, "https://papers.example.com/").tags,
            vec![
                "Work".to_string(),
                "Reading".to_string(),
                "Papers".to_string()
            ]
        );
    }

    #[test]
    fn duplicate_and_empty_folder_names_are_dropped() {
        let html = r#"<DL><p>
            <DT><H3>Work</H3>
            <DL><p>
                <DT><H3></H3>
                <DL><p>
                    <DT><H3>Work</H3>
                    <DL><p>
                        <DT><A HREF="https://dup.example.com/">Dup</A>
                    </DL><p>
                </DL><p>
            </DL><p>
        </DL><p>"#;
        let bookmarks = parse_netscape(html);
        assert_eq!(
            find(&bookmarks, "https://dup.example.com/").tags,
            vec!["Work".to_string()]
        );
    }
}
