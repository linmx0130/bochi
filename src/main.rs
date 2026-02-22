mod adb_utils;
mod selector;
mod ui_element;

use adb_utils::{format_adb_error, get_adb_command};
use clap::Parser;
use selector::Selector;
use std::process::exit;
use std::thread;
use std::time::{Duration, Instant};
use ui_element::{find_elements, get_ui_hierarchy, UiElement};

#[derive(Parser)]
#[command(name = "bochi")]
#[command(about = "A CLI tool for AI agents to control Android devices via ADB")]
struct Cli {
    #[arg(short, long)]
    serial: Option<String>,

    /// Element selector. Supports CSS-like syntax:
    /// - [attr=value] - attribute assertion
    /// - [attr1=v1][attr2=v2] - AND of clauses
    /// - sel1,sel2 - OR of selectors
    /// - :has(cond) - has descendant matching cond
    /// Also supports legacy format: field=value
    #[arg(short = 'e', long)]
    selector: String,

    #[arg(short = 'c', long)]
    command: String,

    #[arg(short, long, default_value = "30")]
    timeout: u64,
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

fn wait_for_element(
    serial: Option<&str>,
    selector: &Selector,
    timeout_secs: u64,
) -> Result<UiElement, String> {
    wait_for_elements(serial, selector, timeout_secs)
        .map(|elements| elements.into_iter().next().unwrap())
}

fn wait_for_elements(
    serial: Option<&str>,
    selector: &Selector,
    timeout_secs: u64,
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
        let elements = find_elements(&xml, selector)?;
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

    let result = match cli.command.as_str() {
        "waitFor" => wait_for_elements(cli.serial.as_deref(), &selector, cli.timeout).map(|elements| {
            for element in elements {
                println!("{}", element.raw_xml);
            }
        }),
        "tap" => match wait_for_element(cli.serial.as_deref(), &selector, cli.timeout) {
            Ok(element) => tap_element(cli.serial.as_deref(), &element),
            Err(e) => Err(e),
        },
        _ => Err(format!(
            "Unknown command: {}. Supported: waitFor, tap",
            cli.command
        )),
    };

    match result {
        Ok(()) => exit(0),
        Err(e) => {
            eprintln!("Error: {}", e);
            exit(1);
        }
    }
}
