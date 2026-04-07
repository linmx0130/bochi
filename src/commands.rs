use crate::adb_utils::{format_adb_error, get_adb_command};
use crate::selector::Selector;
use crate::ui_element::{
    find_elements, find_elements_with_descendants, get_ui_hierarchy, is_element_visible, UiElement,
};
use std::thread;
use std::time::{Duration, Instant};

/// Get the center coordinates of an element
pub fn get_element_center(element: &UiElement) -> (i32, i32) {
    let (x1, y1, x2, y2) = element.bounds;
    let center_x = (x1 + x2) / 2;
    let center_y = (y1 + y2) / 2;
    (center_x, center_y)
}

pub fn tap_element(serial: Option<&str>, remote: Option<&str>, element: &UiElement) -> Result<(), String> {
    let (center_x, center_y) = get_element_center(element);

    let output = get_adb_command(serial, remote)
        .map_err(|e| format_adb_error(&e))?
        .args([
            "shell",
            "input",
            "tap",
            &center_x.to_string(),
            &center_y.to_string(),
        ])
        .output()
        .map_err(|e| format_adb_error(&e))?;

    if !output.status.success() {
        return Err(format!(
            "Tap command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

pub fn long_tap_element(
    serial: Option<&str>,
    remote: Option<&str>,
    element: &UiElement,
    duration_ms: u64,
) -> Result<(), String> {
    let (center_x, center_y) = get_element_center(element);

    // Use swipe with same start and end position to simulate a long press
    let output = get_adb_command(serial, remote)
        .map_err(|e| format_adb_error(&e))?
        .args([
            "shell",
            "input",
            "swipe",
            &center_x.to_string(),
            &center_y.to_string(),
            &center_x.to_string(),
            &center_y.to_string(),
            &duration_ms.to_string(),
        ])
        .output()
        .map_err(|e| format_adb_error(&e))?;

    if !output.status.success() {
        return Err(format!(
            "Long tap command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

pub fn double_tap_element(serial: Option<&str>, remote: Option<&str>, element: &UiElement) -> Result<(), String> {
    // First tap
    tap_element(serial, remote, element)?;

    // Small delay between taps (typical double tap timing)
    thread::sleep(Duration::from_millis(100));

    // Second tap
    tap_element(serial, remote, element)
}

/// Get the screen dimensions (width, height)
pub fn get_screen_dimensions(serial: Option<&str>, remote: Option<&str>) -> Result<(i32, i32), String> {
    let output = get_adb_command(serial, remote)
        .map_err(|e| format_adb_error(&e))?
        .args(["shell", "wm", "size"])
        .output()
        .map_err(|e| format_adb_error(&e))?;

    if !output.status.success() {
        return Err(format!(
            "Failed to get screen size: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    parse_screen_dimensions(&String::from_utf8_lossy(&output.stdout))
}

/// Parse screen dimensions from wm size output
pub fn parse_screen_dimensions(output: &str) -> Result<(i32, i32), String> {
    // Parse "Physical size: 1080x1920" or "Override size: 1080x1920"
    for line in output.lines() {
        if let Some(idx) = line.find("size: ") {
            let size_part = &line[idx + 6..];
            let parts: Vec<&str> = size_part.split('x').collect();
            if parts.len() == 2 {
                if let (Ok(width), Ok(height)) = (parts[0].trim().parse(), parts[1].trim().parse()) {
                    return Ok((width, height));
                }
            }
        }
    }

    Err(format!("Could not parse screen size from: {}", output))
}

/// Perform a swipe gesture
pub fn perform_swipe(
    serial: Option<&str>,
    remote: Option<&str>,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    duration_ms: u64,
) -> Result<(), String> {
    let output = get_adb_command(serial, remote)
        .map_err(|e| format_adb_error(&e))?
        .args([
            "shell",
            "input",
            "swipe",
            &x1.to_string(),
            &y1.to_string(),
            &x2.to_string(),
            &y2.to_string(),
            &duration_ms.to_string(),
        ])
        .output()
        .map_err(|e| format_adb_error(&e))?;

    if !output.status.success() {
        return Err(format!(
            "Swipe command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

/// Calculate swipe coordinates for scrolling within a scrollable element
pub fn calculate_scroll_coordinates(
    scroll_element: &UiElement,
    screen_height: i32,
    scroll_up: bool,
) -> (i32, i32, i32, i32) {
    let (ex1, ey1, ex2, ey2) = scroll_element.bounds;
    let swipe_x = (ex1 + ex2) / 2;

    // Calculate swipe coordinates relative to the scrollable element
    let start_y = if scroll_up {
        ey1 + screen_height / 5 // Start lower within the element
    } else {
        ey2 - screen_height / 5 // Start higher within the element
    };

    let end_y = if scroll_up {
        (ey2 - screen_height / 5).min(screen_height * 7 / 10)
    } else {
        (ey1 + screen_height / 5).max(screen_height * 3 / 10)
    };

    // Clamp coordinates to be within screen bounds
    let actual_start_y = start_y.max(0).min(screen_height);
    let actual_end_y = end_y.max(0).min(screen_height);

    (swipe_x, actual_start_y, swipe_x, actual_end_y)
}

/// Scroll gradually until the target element is visible
pub fn scroll_until_visible(
    serial: Option<&str>,
    remote: Option<&str>,
    scroll_selector: &Selector,
    target_selector: &Selector,
    timeout_secs: u64,
    scroll_up: bool,
) -> Result<(), String> {
    let start = Instant::now();
    let timeout = Duration::from_secs(timeout_secs);

    // Get screen dimensions
    let (screen_width, screen_height) = get_screen_dimensions(serial, remote)?;

    // Swipe duration in ms - moderate speed for smooth scrolling
    let swipe_duration = 300;

    loop {
        if start.elapsed() > timeout {
            return Err(format!(
                "Timeout waiting for target element to become visible: {:?}",
                target_selector
            ));
        }

        // Get current UI hierarchy
        let xml = get_ui_hierarchy(serial, remote)?;

        // First, check if target is already visible
        let target_elements = find_elements(&xml, target_selector)?;
        if let Some(target) = target_elements.first() {
            if is_element_visible(target, screen_width, screen_height) {
                return Ok(());
            }
        }

        // Find scrollable element (the element we swipe on)
        let scroll_elements = find_elements(&xml, scroll_selector)?;
        if scroll_elements.is_empty() {
            return Err(format!(
                "Scroll element not found with selector: {:?}",
                scroll_selector
            ));
        }

        // Perform swipe on the first scrollable element's center area
        let scroll_element = &scroll_elements[0];
        let (x1, y1, x2, y2) = calculate_scroll_coordinates(scroll_element, screen_height, scroll_up);

        perform_swipe(serial, remote, x1, y1, x2, y2, swipe_duration)?;

        // Small delay between swipes to let UI settle
        thread::sleep(Duration::from_millis(500));
    }
}

pub fn input_text_element(serial: Option<&str>, remote: Option<&str>, element: &UiElement, text: &str) -> Result<(), String> {
    // First tap to focus on the element
    tap_element(serial, remote, element)?;

    // Small delay to ensure the element is focused
    thread::sleep(Duration::from_millis(100));

    // Then type the text
    let output = get_adb_command(serial, remote)
        .map_err(|e| format_adb_error(&e))?
        .args(["shell", "input", "text", text])
        .output()
        .map_err(|e| format_adb_error(&e))?;

    if !output.status.success() {
        return Err(format!(
            "Input text command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

pub fn wait_for_element(
    serial: Option<&str>,
    remote: Option<&str>,
    selector: &Selector,
    timeout_secs: u64,
) -> Result<UiElement, String> {
    wait_for_elements(serial, remote, selector, timeout_secs, false)
        .map(|elements| elements.into_iter().next().unwrap())
}

pub fn wait_for_elements(
    serial: Option<&str>,
    remote: Option<&str>,
    selector: &Selector,
    timeout_secs: u64,
    with_descendants: bool,
) -> Result<Vec<UiElement>, String> {
    let start = Instant::now();
    let timeout = Duration::from_secs(timeout_secs);

    loop {
        if start.elapsed() > timeout {
            return Err(format!(
                "Timeout waiting for element with selector: {:?}",
                selector
            ));
        }

        let xml = get_ui_hierarchy(serial, remote)?;
        let elements = if with_descendants {
            find_elements_with_descendants(&xml, selector)?
        } else {
            find_elements(&xml, selector)?
        };
        if !elements.is_empty() {
            return Ok(elements);
        }
        thread::sleep(Duration::from_millis(500));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_element_center() {
        let element = UiElement {
            bounds: (100, 200, 300, 400),
            raw_xml: String::new(),
        };
        assert_eq!(get_element_center(&element), (200, 300));
    }

    #[test]
    fn test_get_element_center_with_zero_bounds() {
        let element = UiElement {
            bounds: (0, 0, 0, 0),
            raw_xml: String::new(),
        };
        assert_eq!(get_element_center(&element), (0, 0));
    }

    #[test]
    fn test_get_element_center_negative_coordinates() {
        let element = UiElement {
            bounds: (-100, -100, 100, 100),
            raw_xml: String::new(),
        };
        assert_eq!(get_element_center(&element), (0, 0));
    }

    #[test]
    fn test_parse_screen_dimensions_physical() {
        let output = "Physical size: 1080x1920\n";
        let result = parse_screen_dimensions(output);
        assert_eq!(result, Ok((1080, 1920)));
    }

    #[test]
    fn test_parse_screen_dimensions_override() {
        let output = "Override size: 720x1280\nPhysical size: 1080x1920\n";
        let result = parse_screen_dimensions(output);
        // Should pick the first "size:" found, which is Override size
        assert_eq!(result, Ok((720, 1280)));
    }

    #[test]
    fn test_parse_screen_dimensions_with_whitespace() {
        let output = "Physical size: 1080 x 1920\n";
        let result = parse_screen_dimensions(output);
        // trim() should handle whitespace
        assert_eq!(result, Ok((1080, 1920)));
    }

    #[test]
    fn test_parse_screen_dimensions_invalid() {
        let output = "Invalid output\n";
        let result = parse_screen_dimensions(output);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Could not parse screen size"));
    }

    #[test]
    fn test_parse_screen_dimensions_malformed() {
        let output = "Physical size: 1080\n";
        let result = parse_screen_dimensions(output);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_screen_dimensions_empty() {
        let output = "";
        let result = parse_screen_dimensions(output);
        assert!(result.is_err());
    }

    #[test]
    fn test_calculate_scroll_coordinates_scroll_down() {
        // Element at (100, 100) to (300, 500), screen height 800
        let scroll_element = UiElement {
            bounds: (100, 100, 300, 500),
            raw_xml: String::new(),
        };
        let (x1, y1, x2, y2) = calculate_scroll_coordinates(&scroll_element, 800, false);

        // swipe_x should be center x: (100 + 300) / 2 = 200
        assert_eq!(x1, 200);
        assert_eq!(x2, 200);

        // For scroll down (swipe up):
        // start_y = ey2 - screen_height/5 = 500 - 160 = 340
        // end_y should be max(ey1 + screen_height/5, screen_height * 3/10)
        // = max(100 + 160, 240) = 260, but then max with 0 -> 260
        assert_eq!(y1, 340);
        assert_eq!(y2, 260);
    }

    #[test]
    fn test_calculate_scroll_coordinates_scroll_up() {
        // Element at (100, 100) to (300, 500), screen height 800
        let scroll_element = UiElement {
            bounds: (100, 100, 300, 500),
            raw_xml: String::new(),
        };
        let (x1, y1, x2, y2) = calculate_scroll_coordinates(&scroll_element, 800, true);

        // swipe_x should be center x: 200
        assert_eq!(x1, 200);
        assert_eq!(x2, 200);

        // For scroll up (swipe down):
        // start_y = ey1 + screen_height/5 = 100 + 160 = 260
        // end_y = min(ey2 - screen_height/5, screen_height * 7/10)
        // = min(500 - 160, 560) = 340
        assert_eq!(y1, 260);
        assert_eq!(y2, 340);
    }

    #[test]
    fn test_calculate_scroll_coordinates_clamping() {
        // Element near edge of screen
        let scroll_element = UiElement {
            bounds: (0, -100, 100, 50),
            raw_xml: String::new(),
        };
        let (_x1, y1, _x2, y2) = calculate_scroll_coordinates(&scroll_element, 800, false);

        // Coordinates should be clamped to screen bounds (0 to screen_height)
        assert!(y1 >= 0);
        assert!(y2 >= 0);
        assert!(y1 <= 800);
        assert!(y2 <= 800);
    }
}
