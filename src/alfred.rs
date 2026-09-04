use serde::Serialize;

use crate::frontmatter::Frontmatter;
use crate::store::StoredBookmark;

#[derive(Serialize)]
struct Feed<'a> {
    items: Vec<Item<'a>>,
}

#[derive(Serialize)]
struct Item<'a> {
    title: &'a str,
    subtitle: String,
    arg: &'a str,
    valid: bool,
    action: Action<'a>,
    mods: Mods<'a>,
}

#[derive(Serialize)]
struct Action<'a> {
    url: &'a str,
}

#[derive(Serialize)]
struct Mods<'a> {
    cmd: Modifier<'a>,
}

#[derive(Serialize)]
struct Modifier<'a> {
    arg: &'a str,
    subtitle: &'a str,
}

pub fn render(bookmarks: &[&StoredBookmark], default_space: Option<&str>) -> String {
    let items: Vec<Item> = bookmarks
        .iter()
        .map(|b| item(&b.frontmatter, default_space))
        .collect();
    serde_json::to_string_pretty(&Feed { items }).expect("alfred feed is always serializable")
}

fn item<'a>(fm: &'a Frontmatter, default_space: Option<&str>) -> Item<'a> {
    Item {
        title: fm.display_title(),
        subtitle: subtitle(fm, default_space),
        arg: &fm.url,
        valid: true,
        action: Action { url: &fm.url },
        mods: Mods {
            cmd: Modifier {
                arg: &fm.url,
                subtitle: "Copy URL",
            },
        },
    }
}

fn subtitle(fm: &Frontmatter, default_space: Option<&str>) -> String {
    match fm.space.as_deref().or(default_space) {
        Some(space) => format!("{space} · {}", fm.url),
        None => fm.url.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn bookmark(fm: Frontmatter) -> StoredBookmark {
        StoredBookmark {
            path: PathBuf::from("x.md"),
            frontmatter: fm,
        }
    }

    fn frontmatter(url: &str, title: Option<&str>, space: Option<&str>) -> Frontmatter {
        Frontmatter {
            url: url.to_string(),
            title: title.map(str::to_string),
            tags: None,
            added: None,
            description: None,
            space: space.map(str::to_string),
        }
    }

    #[test]
    fn empty_input_renders_an_empty_items_array() {
        let value: serde_json::Value = serde_json::from_str(&render(&[], None)).unwrap();
        assert_eq!(value, serde_json::json!({ "items": [] }));
    }

    #[test]
    fn item_maps_every_field_to_the_script_filter_schema() {
        let b = bookmark(frontmatter(
            "https://example.com/a?utm=1",
            Some("Example"),
            Some("work"),
        ));
        let value: serde_json::Value = serde_json::from_str(&render(&[&b], None)).unwrap();
        let item = &value["items"][0];

        assert_eq!(item["title"], "Example");
        assert_eq!(item["subtitle"], "work · https://example.com/a?utm=1");
        assert_eq!(item["arg"], "https://example.com/a?utm=1");
        assert_eq!(item["valid"], true);
        assert_eq!(
            item["action"],
            serde_json::json!({ "url": "https://example.com/a?utm=1" })
        );
        assert_eq!(
            item["mods"]["cmd"],
            serde_json::json!({ "arg": "https://example.com/a?utm=1", "subtitle": "Copy URL" })
        );
        assert!(
            item.as_object().unwrap().get("uid").is_none(),
            "no uid so Alfred cannot reorder by frecency"
        );
    }

    #[test]
    fn title_falls_back_to_url_when_unset() {
        let b = bookmark(frontmatter("https://example.com/x", None, Some("home")));
        let value: serde_json::Value = serde_json::from_str(&render(&[&b], None)).unwrap();
        assert_eq!(value["items"][0]["title"], "https://example.com/x");
    }

    #[test]
    fn unset_space_is_substituted_with_default_space_never_null() {
        let b = bookmark(frontmatter("https://example.com/y", Some("Y"), None));
        let value: serde_json::Value =
            serde_json::from_str(&render(&[&b], Some("personal"))).unwrap();
        assert_eq!(
            value["items"][0]["subtitle"],
            "personal · https://example.com/y"
        );
    }
}
