//! Text validation for engine-specific features.

/// Official ChatterBox Turbo paralinguistic tags (per D-05).
const VALID_TAGS: &[&str] = &[
    "[laugh]", "[chuckle]", "[cough]", "[sigh]",
    "[gasp]", "[groan]", "[yawn]", "[cry]",
];

/// Validate that all bracket-delimited tags in the text are recognized
/// paralinguistic tags for ChatterBox Turbo.
///
/// Returns Ok(()) if all tags are valid, or Err with a message listing
/// invalid tags and the valid set.
///
/// Only call this when engine is ChatterBox and variant is Turbo (per D-06).
pub fn validate_paralinguistic_tags(text: &str) -> Result<(), String> {
    let mut invalid_tags = Vec::new();

    // Find all [tag] patterns in the text
    let mut i = 0;
    let bytes = text.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'[' {
            // Find closing bracket
            if let Some(end) = text[i..].find(']') {
                let tag = &text[i..i + end + 1];
                if !VALID_TAGS.contains(&tag) {
                    invalid_tags.push(tag.to_string());
                }
                i += end + 1;
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }

    if invalid_tags.is_empty() {
        Ok(())
    } else {
        let valid_list = VALID_TAGS.join(", ");
        let invalid_list = invalid_tags.join(", ");
        Err(format!(
            "Invalid paralinguistic tag(s): {invalid_list}\n\
             Valid tags for ChatterBox Turbo: {valid_list}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_tags_accepted() {
        assert!(validate_paralinguistic_tags("Hello [laugh] world").is_ok());
        assert!(validate_paralinguistic_tags("[cry] oh no [sigh]").is_ok());
        assert!(validate_paralinguistic_tags("No tags here").is_ok());
    }

    #[test]
    fn invalid_tags_rejected() {
        let result = validate_paralinguistic_tags("Hello [invalid_tag] world");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("[invalid_tag]"));
        assert!(err.contains("Valid tags for ChatterBox Turbo"));
    }

    #[test]
    fn mixed_valid_and_invalid() {
        let result = validate_paralinguistic_tags("[laugh] hello [boom] bye [cry]");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("[boom]"));
        assert!(!err.contains("[laugh]"));
        assert!(!err.contains("[cry]"));
    }

    #[test]
    fn unclosed_bracket_ignored() {
        assert!(validate_paralinguistic_tags("Hello [unclosed text").is_ok());
    }
}
