const MAX_SLUG_LEN: usize = 200;

pub fn slug(title: &str) -> String {
    let mut out = String::new();
    let mut pending_dash = false;

    for c in title.chars() {
        if c.is_ascii_alphanumeric() {
            if pending_dash && !out.is_empty() {
                out.push('-');
            }
            pending_dash = false;
            out.push(c.to_ascii_lowercase());
        } else {
            pending_dash = true;
        }
        if out.len() >= MAX_SLUG_LEN {
            break;
        }
    }

    out.truncate(MAX_SLUG_LEN);
    while out.ends_with('-') {
        out.pop();
    }

    if out.is_empty() {
        out.push_str("bookmark");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowercases_and_dashes_words() {
        assert_eq!(slug("Hello World"), "hello-world");
    }

    #[test]
    fn collapses_runs_of_separators() {
        assert_eq!(slug("A  --  B"), "a-b");
    }

    #[test]
    fn trims_leading_and_trailing_separators() {
        assert_eq!(slug("  Rust! "), "rust");
    }

    #[test]
    fn drops_non_ascii() {
        assert_eq!(slug("Café Del Mar"), "caf-del-mar");
    }

    #[test]
    fn empty_and_symbol_only_fall_back() {
        assert_eq!(slug(""), "bookmark");
        assert_eq!(slug("!!!"), "bookmark");
    }

    #[test]
    fn caps_length_and_leaves_room_for_extension_and_suffix() {
        let long = "a".repeat(1000);
        let out = slug(&long);
        assert!(out.len() <= MAX_SLUG_LEN, "slug too long: {}", out.len());
    }

    #[test]
    fn truncation_does_not_leave_a_trailing_dash() {
        let title = format!("{} tail", "word ".repeat(60));
        let out = slug(&title);
        assert!(out.len() <= MAX_SLUG_LEN);
        assert!(
            !out.ends_with('-'),
            "trailing dash left after truncation: {out}"
        );
    }
}
