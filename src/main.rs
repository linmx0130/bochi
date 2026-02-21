mod adb_utils;
mod selector;
mod ui_element;

use adb_utils::get_adb_command;
use clap::Parser;
use selector::Selector;
use std::process::exit;
use std::thread;
use std::time::{Duration, Instant};
use ui_element::{find_element, get_ui_hierarchy, UiElement};

#[derive(Parser)]
#[command(name = "bochi")]
#[command(about = "A CLI tool for AI agents to control Android devices via ADB")]
struct Cli {
    #[arg(short, long)]
    serial: Option<String>,

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
        .args([
            "shell",
            "input",
            "tap",
            &center_x.to_string(),
            &center_y.to_string(),
        ])
        .output()
        .map_err(|e| format!("Failed to execute tap command: {}", e))?;

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
    let start = Instant::now();
    let timeout = Duration::from_secs(timeout_secs);

    loop {
        if start.elapsed() > timeout {
            return Err(format!(
                "Timeout waiting for element with selector: {}={}",
                selector.field, selector.value
            ));
        }

        let xml = get_ui_hierarchy(serial)?;
        match find_element(&xml, selector)? {
            Some(element) => return Ok(element),
            None => thread::sleep(Duration::from_millis(500)),
        }
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
        "waitFor" => wait_for_element(cli.serial.as_deref(), &selector, cli.timeout).map(|_| ()),
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
