mod adb_utils;
mod selector;
mod ui_element;

use adb_utils::{format_adb_error, get_adb_command};
use clap::Parser;
use selector::Selector;
use std::process::exit;
use std::thread;
use std::time::{Duration, Instant};
use ui_element::{find_elements, find_elements_with_descendants, get_ui_hierarchy, UiElement};

#[derive(clap::ValueEnum, Clone, Debug)]
enum BochiCommand {
    #[value(name = "waitFor")]
    WaitFor,
    #[value(name = "tap")]
    Tap,
    #[value(name = "inputText")]
    InputText,
    #[value(name = "longTap")]
    LongTap,
    #[value(name = "doubleTap")]
    DoubleTap,
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
- [attr=value] - attribute assertion
- [attr1=v1][attr2=v2] - AND of clauses
- sel1,sel2 - OR of selectors
- :has(cond) - has descendant matching cond"#,
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
        BochiCommand::Tap => match wait_for_element(cli.serial.as_deref(), &selector, cli.timeout) {
            Ok(element) => tap_element(cli.serial.as_deref(), &element),
            Err(e) => Err(e),
        },
        BochiCommand::InputText => match cli.text {
            Some(text) => match wait_for_element(cli.serial.as_deref(), &selector, cli.timeout) {
                Ok(element) => input_text_element(cli.serial.as_deref(), &element, &text),
                Err(e) => Err(e),
            },
            None => Err("--text parameter is required for inputText command".to_string()),
        },
        BochiCommand::LongTap => match wait_for_element(cli.serial.as_deref(), &selector, cli.timeout) {
            Ok(element) => long_tap_element(cli.serial.as_deref(), &element, 1000),
            Err(e) => Err(e),
        },
        BochiCommand::DoubleTap => match wait_for_element(cli.serial.as_deref(), &selector, cli.timeout) {
            Ok(element) => double_tap_element(cli.serial.as_deref(), &element),
            Err(e) => Err(e),
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
