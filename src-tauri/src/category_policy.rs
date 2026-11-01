/// Categories exposed by the adult-content browser must not describe sexualized minors.
pub fn is_supported_category(value: &str) -> bool {
    let key: String = value
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect();
    let excluded = [
        "没有",
    ];
    !key.is_empty()
        && !excluded.iter().any(|term| key.contains(term))
        && !["jk", "jc", "js"].contains(&key.as_str())
}

#[cfg(test)]
mod tests {
    use super::is_supported_category;

    #[test]
    fn permits_ordinary_adult_catalog_categories() {
        for title in ["漢化", "汉化", "原創", "完结", "成熟女性", "Cosplay"] {
            assert!(is_supported_category(title), "{title}");
        }
    }

    #[test]
    fn excludes_minor_categories_and_blank_titles() {
        for title in ["没有", " "] {
            assert!(!is_supported_category(title), "{title}");
        }
    }
}
