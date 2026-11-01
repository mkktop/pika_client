/// Categories exposed by the adult-content browser must not describe sexualized minors.
pub fn is_supported_category(value: &str) -> bool {
    let key: String = value
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect();
    let excluded = [
        "萝莉", "蘿莉", "幼女", "幼儿", "幼兒", "儿童", "兒童", "未成年", "正太",
        "小学生", "小學生", "中学生", "中學生", "高中生", "高校生", "loli", "shota",
        "underage", "child", "schoolgirl", "highschool",
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
        for title in ["萝莉", "蘿莉", "幼女", "兒童", "正太", "L O L I", "JK", " "] {
            assert!(!is_supported_category(title), "{title}");
        }
    }
}
