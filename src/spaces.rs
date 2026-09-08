use crate::alfred::{self, Item};
use crate::config::{SpaceConfig, Spaces};

pub const DEFAULT_TITLE: &str = "(default)";

const SYSTEM_DEFAULT_LABEL: &str = "system default browser";

pub fn render_lines(spaces: &Spaces) -> Vec<String> {
    spaces
        .iter()
        .map(|(name, cfg)| format!("{name}  {}", label(cfg)))
        .collect()
}

pub fn render_alfred(spaces: &Spaces, default_space: Option<&str>) -> String {
    let mut items = Vec::with_capacity(spaces.len() + 1);
    items.push(Item::new(
        DEFAULT_TITLE,
        default_label(spaces, default_space),
        "",
        None,
    ));
    for (name, cfg) in spaces {
        items.push(Item::new(name.as_str(), label(cfg), name.as_str(), None));
    }
    alfred::feed(items)
}

fn label(cfg: &SpaceConfig) -> String {
    match &cfg.profile {
        Some(profile) => format!("{} · {profile}", cfg.browser),
        None => cfg.browser.clone(),
    }
}

fn default_label(spaces: &Spaces, default_space: Option<&str>) -> String {
    match default_space.and_then(|name| spaces.get(name)) {
        Some(cfg) => label(cfg),
        None => SYSTEM_DEFAULT_LABEL.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(browser: &str, profile: Option<&str>) -> SpaceConfig {
        SpaceConfig {
            browser: browser.to_string(),
            profile: profile.map(str::to_string),
            chromium_support_dir: None,
        }
    }

    fn spaces(pairs: &[(&str, &str, Option<&str>)]) -> Spaces {
        pairs
            .iter()
            .map(|(name, browser, profile)| (name.to_string(), config(browser, *profile)))
            .collect()
    }

    #[test]
    fn label_joins_browser_and_profile() {
        assert_eq!(
            label(&config("Google Chrome", Some("Work"))),
            "Google Chrome · Work"
        );
    }

    #[test]
    fn label_is_browser_only_without_a_profile() {
        assert_eq!(label(&config("Safari", None)), "Safari");
    }

    #[test]
    fn human_lines_follow_config_order_one_per_space() {
        let spaces = spaces(&[
            ("work", "Google Chrome", Some("Work")),
            ("home", "Safari", None),
        ]);
        assert_eq!(
            render_lines(&spaces),
            vec![
                "work  Google Chrome · Work".to_string(),
                "home  Safari".to_string(),
            ]
        );
    }

    #[test]
    fn human_lines_are_empty_without_spaces() {
        assert!(render_lines(&spaces(&[])).is_empty());
    }

    #[test]
    fn alfred_leads_with_default_then_spaces_in_config_order() {
        let spaces = spaces(&[
            ("work", "Google Chrome", Some("Work")),
            ("home", "Firefox", None),
        ]);
        let feed: serde_json::Value =
            serde_json::from_str(&render_alfred(&spaces, Some("home"))).unwrap();
        let items = feed["items"].as_array().unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(items[0]["title"], DEFAULT_TITLE);
        assert_eq!(items[0]["subtitle"], "Firefox");
        assert_eq!(items[0]["arg"], "");
        assert_eq!(items[1]["title"], "work");
        assert_eq!(items[1]["arg"], "work");
        assert_eq!(items[1]["subtitle"], "Google Chrome · Work");
        assert_eq!(items[2]["title"], "home");
        assert_eq!(items[2]["arg"], "home");
    }

    #[test]
    fn alfred_rows_carry_no_action() {
        let feed: serde_json::Value =
            serde_json::from_str(&render_alfred(&spaces(&[("work", "Safari", None)]), None))
                .unwrap();
        for item in feed["items"].as_array().unwrap() {
            assert!(item.as_object().unwrap().get("action").is_none());
            assert_eq!(item["valid"], true);
        }
    }

    #[test]
    fn default_row_falls_back_to_system_browser_without_a_default_space() {
        let feed: serde_json::Value =
            serde_json::from_str(&render_alfred(&spaces(&[("work", "Safari", None)]), None))
                .unwrap();
        assert_eq!(feed["items"][0]["subtitle"], SYSTEM_DEFAULT_LABEL);
    }

    #[test]
    fn no_spaces_still_emits_a_working_default_row() {
        let feed: serde_json::Value =
            serde_json::from_str(&render_alfred(&spaces(&[]), None)).unwrap();
        let items = feed["items"].as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["title"], DEFAULT_TITLE);
        assert_eq!(items[0]["arg"], "");
    }
}
