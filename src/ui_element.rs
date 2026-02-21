use crate::adb_utils::get_adb_command;
use crate::selector::Selector;
use regex::Regex;
use roxmltree::Document;

#[derive(Debug)]
pub struct UiElement {
    pub bounds: (i32, i32, i32, i32),
    pub raw_xml: String,
}

pub fn get_ui_hierarchy(serial: Option<&str>) -> Result<String, String> {
    let output = get_adb_command(serial)
        .args(["shell", "uiautomator", "dump", "/sdcard/window_dump.xml"])
        .output()
        .map_err(|e| format!("Failed to execute adb shell uiautomator dump: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "uiautomator dump failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let output = get_adb_command(serial)
        .args(["shell", "cat", "/sdcard/window_dump.xml"])
        .output()
        .map_err(|e| format!("Failed to read dump file: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Failed to read dump file: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    String::from_utf8(output.stdout).map_err(|e| format!("Invalid UTF-8 in dump file: {}", e))
}

pub fn parse_bounds(bounds_str: &str) -> Option<(i32, i32, i32, i32)> {
    // Using r##"..."## for regex pattern with backslashes
    let pattern = r##"\[(\d+),(\d+)\]\[(\d+),(\d+)\]"##;
    let re = Regex::new(pattern).unwrap();
    if let Some(caps) = re.captures(bounds_str) {
        let x1: i32 = caps.get(1)?.as_str().parse().ok()?;
        let y1: i32 = caps.get(2)?.as_str().parse().ok()?;
        let x2: i32 = caps.get(3)?.as_str().parse().ok()?;
        let y2: i32 = caps.get(4)?.as_str().parse().ok()?;
        Some((x1, y1, x2, y2))
    } else {
        None
    }
}

fn node_to_xml_string(node: roxmltree::Node) -> String {
    if !node.is_element() {
        return String::new();
    }

    let tag_name = node.tag_name().name();
    
    // Build attributes string
    let mut attrs = String::new();
    for attr in node.attributes() {
        let name = attr.name();
        let value = attr.value();
        // Escape special characters in attribute values
        let escaped = value
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;");
        attrs.push_str(&format!(r##" {}="{}""##, name, escaped));
    }

    // Check if node has children
    let has_children = node.children().any(|child| child.is_element());
    
    if has_children {
        format!("<{}{}>", tag_name, attrs)
    } else {
        format!("<{}{} />", tag_name, attrs)
    }
}

pub fn find_element(xml: &str, selector: &Selector) -> Result<Option<UiElement>, String> {
    let doc = Document::parse(xml).map_err(|e| format!("Failed to parse XML: {}", e))?;

    for node in doc.descendants() {
        if node.is_element() {
            let attr_value = match selector.field.as_str() {
                "text" => node.attribute("text"),
                "contentDescription" | "content-description" => node.attribute("content-desc"),
                "resourceId" | "resource-id" => node.attribute("resource-id"),
                "class" => node.attribute("class"),
                "package" => node.attribute("package"),
                field => node.attribute(field),
            };

            if let Some(value) = attr_value {
                if value == selector.value {
                    if let Some(bounds_str) = node.attribute("bounds") {
                        if let Some(bounds) = parse_bounds(bounds_str) {
                            let raw_xml = node_to_xml_string(node);
                            return Ok(Some(UiElement { bounds, raw_xml }));
                        }
                    }
                }
            }
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bounds_valid() {
        let result = parse_bounds("[100,200][300,400]");
        assert_eq!(result, Some((100, 200, 300, 400)));
    }

    #[test]
    fn test_parse_bounds_invalid() {
        assert_eq!(parse_bounds("invalid"), None);
        assert_eq!(parse_bounds(""), None);
    }
}
