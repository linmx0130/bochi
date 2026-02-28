mod adb_utils;
mod selector;
mod ui_element;

use adb_utils::{format_adb_error, get_adb_command};
use clap::Parser;
use selector::Selector;
use std::process::exit;
use std::thread;
use std::time::{Duration, Instant};
use ui_element::{find_elements, find_elements_with_descendants, get_ui_hierarchy, is_element_visible, UiElement};

#[derive(clap::ValueEnum, Clone, Debug)]
enum BochiCommand {
    /// Wait for an element to appear
    #[value(name = "waitFor")]
    WaitFor,
    /// Tap an element
    #[value(name = "tap")]
    Tap,
    /// Input text into an element
    #[value(name = "inputText")]
    InputText,
    /// Long tap an element
    #[value(name = "longTap")]
    LongTap,
    /// Double tap an element
    #[value(name = "doubleTap")]
    DoubleTap,
    /// Scroll up until the target element is visible
    #[value(name = "scrollUp")]
    ScrollUp,
    /// Scroll down until the target element is visible
    #[value(name = "scrollDown")]
    ScrollDown,
}

#[derive(Parser)]
#[command(name = "bochi")]
#[command(about = "A CLI tool for AI agents to control Android devices via ADB")]
struct Cli {
    #[arg(short, long, help_heading = "Common Parameters", display_order = 1)]
    serial: Option<String>,

    /// Element selector
    #[arg(
        short = 'e',
        long,
        help = "Element selector",
        long_help = r#"Element selector.

Supports CSS-like syntax:
 - `[attr="value"]` or `[attr=value]` - attribute assertion
 - `[attr1="v1"][attr2="v2"]` - AND of multiple clauses (no space)
 - `sel1,sel2` - OR of multiple selectors
 - `:has(cond)` - select nodes with a descendant matching cond
 - `:not(cond)` - select nodes that do NOT match cond
 - `ancestor > child` - child combinator (direct children only)
 - `ancestor descendant` - descendant combinator (any depth)"#,
        help_heading = "Common Parameters",
        display_order = 2
    )]
    selector: String,

    #[arg(
        short = 'c',
        long,
        help = "Command to run",
        help_heading = "Common Parameters",
        display_order = 3
    )]
    command: BochiCommand,

    /// Text content for inputText command
    #[arg(long, help_heading = "Command-Specific Parameters", display_order = 10)]
    text: Option<String>,

    #[arg(
        short,
        long,
        default_value = "30",
        help_heading = "Common Parameters",
        display_order = 4
    )]
    timeout: u64,

    /// Print the XML of matched elements including their descendants (for waitFor command)
    #[arg(
        long,
        default_value = "false",
        help_heading = "Command-Specific Parameters",
        display_order = 20
    )]
    print_descendants: bool,

    /// Target element selector for scrollUp/scrollDown commands (element to scroll to)
    #[arg(
        long,
        help = "Target element selector for scrollUp/scrollDown commands",
        long_help = r##"Target element selector for scrollUp/scrollDown commands.

Specifies the element to scroll into view. Supports the same CSS-like syntax as -e/--selector.
Example: --scroll-target '[text="Submit Button"]'
"##,
        help_heading = "Command-Specific Parameters",
        display_order = 21
    )]
    scroll_target: Option<String>,
}

fn tap_element(serial: Option<&str>, element: &UiElement) -> Result<(), String> {
    let (x1, y1, x2, y2) = element.bounds;
    let center_x = (x1 + x2) / 2;
    let center_y = (y1 + y2) / 2;

    let output = get_adb_command(serial)
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

fn long_tap_element(
    serial: Option<&str>,
    element: &UiElement,
    duration_ms: u64,
) -> Result<(), String> {
    let (x1, y1, x2, y2) = element.bounds;
    let center_x = (x1 + x2) / 2;
    let center_y = (y1 + y2) / 2;

    // Use swipe with same start and end position to simulate a long press
    let output = get_adb_command(serial)
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

fn double_tap_element(serial: Option<&str>, element: &UiElement) -> Result<(), String> {
    // First tap
    tap_element(serial, element)?;

    // Small delay between taps (typical double tap timing)
    thread::sleep(Duration::from_millis(100));

    // Second tap
    tap_element(serial, element)
}

/// Get the screen dimensions (width, height)
fn get_screen_dimensions(serial: Option<&str>) -> Result<(i32, i32), String> {
    let output = get_adb_command(serial)
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

    let output_str = String::from_utf8_lossy(&output.stdout);
    // Parse "Physical size: 1080x1920" or "Override size: 1080x1920"
    for line in output_str.lines() {
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

    Err(format!("Could not parse screen size from: {}", output_str))
}

/// Perform a swipe gesture
fn perform_swipe(
    serial: Option<&str>,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    duration_ms: u64,
) -> Result<(), String> {
    let output = get_adb_command(serial)
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

/// Scroll gradually until the target element is visible
fn scroll_until_visible(
    serial: Option<&str>,
    scroll_selector: &Selector,
    target_selector: &Selector,
    timeout_secs: u64,
    scroll_up: bool,
) -> Result<(), String> {
    let start = Instant::now();
    let timeout = Duration::from_secs(timeout_secs);

    // Get screen dimensions
    let (screen_width, screen_height) = get_screen_dimensions(serial)?;

    // Calculate swipe parameters
    // Swipe from 70% to 30% of screen height (or reverse for scroll up)
    let start_y = if scroll_up {
        screen_height * 3 / 10 // Start from 30% from top
    } else {
        screen_height * 7 / 10 // Start from 70% from top
    };
    let end_y = if scroll_up {
        screen_height * 7 / 10 // End at 70% from top (swiping down)
    } else {
        screen_height * 3 / 10 // End at 30% from top (swiping up)
    };
    let _center_x = screen_width / 2;

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
        let xml = get_ui_hierarchy(serial)?;

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
        let (ex1, ey1, ex2, ey2) = scroll_element.bounds;
        let swipe_x = (ex1 + ex2) / 2;

        // Calculate swipe coordinates relative to the scrollable element
        let actual_start_y = if scroll_up {
            ey1 + screen_height / 5 // Start lower within the element
        } else {
            ey2 - screen_height / 5 // Start higher within the element
        };
        let actual_end_y = if scroll_up {
            (ey2 - screen_height / 5).min(start_y + (end_y - start_y).abs())
        } else {
            (ey1 + screen_height / 5).max(start_y - (end_y - start_y).abs())
        };

        // Clamp coordinates to be within screen bounds
        let actual_start_y = actual_start_y.max(0).min(screen_height);
        let actual_end_y = actual_end_y.max(0).min(screen_height);

        perform_swipe(
            serial,
            swipe_x,
            actual_start_y,
            swipe_x,
            actual_end_y,
            swipe_duration,
        )?;

        // Small delay between swipes to let UI settle
        thread::sleep(Duration::from_millis(500));
    }
}

fn input_text_element(serial: Option<&str>, element: &UiElement, text: &str) -> Result<(), String> {
    // First tap to focus on the element
    tap_element(serial, element)?;

    // Small delay to ensure the element is focused
    thread::sleep(Duration::from_millis(100));

    // Then type the text
    let output = get_adb_command(serial)
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

fn wait_for_element(
    serial: Option<&str>,
    selector: &Selector,
    timeout_secs: u64,
) -> Result<UiElement, String> {
    wait_for_elements(serial, selector, timeout_secs, false)
        .map(|elements| elements.into_iter().next().unwrap())
}

fn wait_for_elements(
    serial: Option<&str>,
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

        let xml = get_ui_hierarchy(serial)?;
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

fn main() {
    let cli = Cli::parse();

    let selector = match Selector::parse(&cli.selector) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: {}", e);
            exit(1);
        }
    };

    let result = match cli.command {
        BochiCommand::WaitFor => wait_for_elements(
            cli.serial.as_deref(),
            &selector,
            cli.timeout,
            cli.print_descendants,
        )
        .map(|elements| {
            for element in elements {
                println!("{}", element.raw_xml);
            }
        }),
        BochiCommand::Tap => {
            match wait_for_element(cli.serial.as_deref(), &selector, cli.timeout) {
                Ok(element) => tap_element(cli.serial.as_deref(), &element),
                Err(e) => Err(e),
            }
        }
        BochiCommand::InputText => match cli.text {
            Some(text) => match wait_for_element(cli.serial.as_deref(), &selector, cli.timeout) {
                Ok(element) => input_text_element(cli.serial.as_deref(), &element, &text),
                Err(e) => Err(e),
            },
            None => Err("--text parameter is required for inputText command".to_string()),
        },
        BochiCommand::LongTap => {
            match wait_for_element(cli.serial.as_deref(), &selector, cli.timeout) {
                Ok(element) => long_tap_element(cli.serial.as_deref(), &element, 1000),
                Err(e) => Err(e),
            }
        }
        BochiCommand::DoubleTap => {
            match wait_for_element(cli.serial.as_deref(), &selector, cli.timeout) {
                Ok(element) => double_tap_element(cli.serial.as_deref(), &element),
                Err(e) => Err(e),
            }
        }
        BochiCommand::ScrollUp => match cli.scroll_target {
            Some(target_str) => match Selector::parse(&target_str) {
                Ok(target_selector) => scroll_until_visible(
                    cli.serial.as_deref(),
                    &selector,
                    &target_selector,
                    cli.timeout,
                    true, // scroll_up = true
                ),
                Err(e) => Err(format!("Failed to parse scroll target selector: {}", e)),
            },
            None => Err("--scroll-target parameter is required for scrollUp command".to_string()),
        },
        BochiCommand::ScrollDown => match cli.scroll_target {
            Some(target_str) => match Selector::parse(&target_str) {
                Ok(target_selector) => scroll_until_visible(
                    cli.serial.as_deref(),
                    &selector,
                    &target_selector,
                    cli.timeout,
                    false, // scroll_up = false
                ),
                Err(e) => Err(format!("Failed to parse scroll target selector: {}", e)),
            },
            None => Err("--scroll-target parameter is required for scrollDown command".to_string()),
        },
    };

    match result {
        Ok(()) => exit(0),
        Err(e) => {
            eprintln!("Error: {}", e);
            exit(1);
        }
    }
}
