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
}
