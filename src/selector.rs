use regex::Regex;

#[derive(Debug)]
pub struct Selector {
    pub field: String,
    pub value: String,
}

impl Selector {
    pub fn parse(s: &str) -> Result<Selector, String> {
        // Using r##"..."## to avoid issues with escaped quotes in raw strings
        let pattern = r##"^(\w+)=["']?(.+?)["']?$"##;
        let re = Regex::new(pattern).unwrap();
        if let Some(caps) = re.captures(s) {
            let field = caps.get(1).unwrap().as_str().to_string();
            let mut value = caps.get(2).unwrap().as_str().to_string();
            // Remove surrounding quotes if present
            if (value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\''))
            {
                value = value[1..value.len() - 1].to_string();
            }
            Ok(Selector { field, value })
        } else {
            Err(format!(
                "Invalid selector format: {}. Expected: FIELD_NAME=VALUE",
                s
            ))
        }
    }

    /// Check if the given XML node matches this selector
    pub fn matches(&self, node: roxmltree::Node) -> bool {
        if !node.is_element() {
            return false;
        }

        let attr_value = match self.field.as_str() {
            "text" => node.attribute("text"),
            "contentDescription" | "content-description" => node.attribute("content-desc"),
            "resourceId" | "resource-id" => node.attribute("resource-id"),
            "class" => node.attribute("class"),
            "package" => node.attribute("package"),
            field => node.attribute(field),
        };

        attr_value == Some(self.value.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let s = Selector::parse("text=Submit").unwrap();
        assert_eq!(s.field, "text");
        assert_eq!(s.value, "Submit");
    }

    #[test]
    fn test_parse_with_double_quotes() {
        let s = Selector::parse("text=\"Submit Button\"").unwrap();
        assert_eq!(s.field, "text");
        assert_eq!(s.value, "Submit Button");
    }

    #[test]
    fn test_parse_with_single_quotes() {
        let s = Selector::parse("contentDescription='Open Menu'").unwrap();
        assert_eq!(s.field, "contentDescription");
        assert_eq!(s.value, "Open Menu");
    }

    #[test]
    fn test_parse_resource_id() {
        let s = Selector::parse("resourceId=com.example:id/button").unwrap();
        assert_eq!(s.field, "resourceId");
        assert_eq!(s.value, "com.example:id/button");
    }

    #[test]
    fn test_parse_invalid() {
        assert!(Selector::parse("invalid").is_err());
    }
}
