use crate::adb_utils::{format_adb_error, get_adb_command};
use crate::selector::Selector;
use regex::Regex;
use roxmltree::{Document, Node};

#[derive(Debug)]
pub struct UiElement {
    pub bounds: (i32, i32, i32, i32),
    pub raw_xml: String,
}

/// Check if an element is visible within the given screen dimensions
/// Returns true if the element's bounds are at least partially within the screen
pub fn is_element_visible(element: &UiElement, screen_width: i32, screen_height: i32) -> bool {
    let (x1, y1, x2, y2) = element.bounds;
    // Element is visible if it has any overlap with the screen bounds
    // Check that the element is not completely outside the screen
    let has_horizontal_overlap = x1 < screen_width && x2 > 0;
    let has_vertical_overlap = y1 < screen_height && y2 > 0;
    has_horizontal_overlap && has_vertical_overlap
}

pub fn get_ui_hierarchy(serial: Option<&str>) -> Result<String, String> {
    let output = get_adb_command(serial)
        .map_err(|e| format_adb_error(&e))?
        .args(["shell", "uiautomator", "dump", "/sdcard/window_dump.xml"])
        .output()
        .map_err(|e| format_adb_error(&e))?;

    if !output.status.success() {
        return Err(format!(
            "uiautomator dump failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let output = get_adb_command(serial)
        .map_err(|e| format_adb_error(&e))?
        .args(["shell", "cat", "/sdcard/window_dump.xml"])
        .output()
        .map_err(|e| format_adb_error(&e))?;

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

/// Recursively convert a node and its descendants to XML string
fn node_to_xml_string_with_descendants(node: roxmltree::Node) -> String {
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

    // Collect child elements recursively
    let child_elements: Vec<String> = node
        .children()
        .filter(|child| child.is_element())
        .map(|child| node_to_xml_string_with_descendants(child))
        .filter(|s| !s.is_empty())
        .collect();

    if child_elements.is_empty() {
        format!("<{}{} />", tag_name, attrs)
    } else {
        let children_xml = child_elements.join("");
        format!("<{}{}>{}</{}>", tag_name, attrs, children_xml, tag_name)
    }
}

pub fn find_elements(xml: &str, selector: &Selector) -> Result<Vec<UiElement>, String> {
    let doc = Document::parse(xml).map_err(|e| format!("Failed to parse XML: {}", e))?;
    let mut elements = Vec::new();

    collect_matching_elements(doc.root(), selector, &mut elements);

    Ok(elements)
}

fn collect_matching_elements(node: Node, selector: &Selector, elements: &mut Vec<UiElement>) {
    if node.is_element() && selector.matches(node) {
        if let Some(bounds_str) = node.attribute("bounds") {
            if let Some(bounds) = parse_bounds(bounds_str) {
                let raw_xml = node_to_xml_string(node);
                elements.push(UiElement { bounds, raw_xml });
            }
        }
    }

    // Recursively check children
    for child in node.children() {
        collect_matching_elements(child, selector, elements);
    }
}

fn collect_matching_elements_with_descendants(
    node: Node,
    selector: &Selector,
    elements: &mut Vec<UiElement>,
) {
    if node.is_element() && selector.matches(node) {
        if let Some(bounds_str) = node.attribute("bounds") {
            if let Some(bounds) = parse_bounds(bounds_str) {
                let raw_xml = node_to_xml_string_with_descendants(node);
                elements.push(UiElement { bounds, raw_xml });
            }
        }
    }

    // Recursively check children
    for child in node.children() {
        collect_matching_elements_with_descendants(child, selector, elements);
    }
}

pub fn find_elements_with_descendants(
    xml: &str,
    selector: &Selector,
) -> Result<Vec<UiElement>, String> {
    let doc = Document::parse(xml).map_err(|e| format!("Failed to parse XML: {}", e))?;
    let mut elements = Vec::new();

    collect_matching_elements_with_descendants(doc.root(), selector, &mut elements);

    Ok(elements)
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

    #[test]
    fn test_is_element_visible_fully_inside() {
        let element = UiElement {
            bounds: (100, 100, 200, 200),
            raw_xml: String::new(),
        };
        assert!(is_element_visible(&element, 500, 500));
    }

    #[test]
    fn test_is_element_visible_partially_inside() {
        // Partially visible on the right edge
        let element = UiElement {
            bounds: (450, 100, 550, 200),
            raw_xml: String::new(),
        };
        assert!(is_element_visible(&element, 500, 500));

        // Partially visible on the bottom edge
        let element = UiElement {
            bounds: (100, 450, 200, 550),
            raw_xml: String::new(),
        };
        assert!(is_element_visible(&element, 500, 500));
    }

    #[test]
    fn test_is_element_visible_completely_outside() {
        // Completely to the right
        let element = UiElement {
            bounds: (600, 100, 700, 200),
            raw_xml: String::new(),
        };
        assert!(!is_element_visible(&element, 500, 500));

        // Completely to the bottom
        let element = UiElement {
            bounds: (100, 600, 200, 700),
            raw_xml: String::new(),
        };
        assert!(!is_element_visible(&element, 500, 500));

        // Completely to the left (negative coordinates)
        let element = UiElement {
            bounds: (-100, 100, -50, 200),
            raw_xml: String::new(),
        };
        assert!(!is_element_visible(&element, 500, 500));

        // Completely to the top (negative coordinates)
        let element = UiElement {
            bounds: (100, -100, 200, -50),
            raw_xml: String::new(),
        };
        assert!(!is_element_visible(&element, 500, 500));
    }

    #[test]
    fn test_is_element_visible_exactly_at_edge() {
        // Right edge exactly at 0 (no overlap)
        let element = UiElement {
            bounds: (-100, 100, 0, 200),
            raw_xml: String::new(),
        };
        assert!(!is_element_visible(&element, 500, 500));

        // Bottom edge exactly at 0 (no overlap)
        let element = UiElement {
            bounds: (100, -100, 200, 0),
            raw_xml: String::new(),
        };
        assert!(!is_element_visible(&element, 500, 500));

        // Left edge exactly at screen width (no overlap)
        let element = UiElement {
            bounds: (500, 100, 600, 200),
            raw_xml: String::new(),
        };
        assert!(!is_element_visible(&element, 500, 500));

        // Top edge exactly at screen height (no overlap)
        let element = UiElement {
            bounds: (100, 500, 200, 600),
            raw_xml: String::new(),
        };
        assert!(!is_element_visible(&element, 500, 500));
    }

    #[test]
    fn test_is_element_visible_edge_cases() {
        // Element at (0,0) with size 0
        let element = UiElement {
            bounds: (0, 0, 0, 0),
            raw_xml: String::new(),
        };
        assert!(!is_element_visible(&element, 500, 500));

        // Element exactly filling the screen
        let element = UiElement {
            bounds: (0, 0, 500, 500),
            raw_xml: String::new(),
        };
        assert!(is_element_visible(&element, 500, 500));

        // Element larger than screen
        let element = UiElement {
            bounds: (-100, -100, 600, 600),
            raw_xml: String::new(),
        };
        assert!(is_element_visible(&element, 500, 500));
    }
}
