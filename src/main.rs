use clap::Parser;
use regex::Regex;
use roxmltree::Document;
use std::process::{Command, exit};
use std::thread;
use std::time::{Duration, Instant};

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

#[derive(Debug)]
struct Selector {
    field: String,
    value: String,
}

impl Selector {
    fn parse(s: &str) -> Result<Selector, String> {
        // Using r##"..."## to avoid issues with escaped quotes in raw strings
        let pattern = r##"^(\w+)=["']?(.+?)["']?$"##;
        let re = Regex::new(pattern).unwrap();
        if let Some(caps) = re.captures(s) {
            let field = caps.get(1).unwrap().as_str().to_string();
            let mut value = caps.get(2).unwrap().as_str().to_string();
            // Remove surrounding quotes if present
            if (value.starts_with('"') && value.ends_with('"')) 
                || (value.starts_with('\'') && value.ends_with('\'')) {
                value = value[1..value.len()-1].to_string();
            }
            Ok(Selector { field, value })
        } else {
            Err(format!("Invalid selector format: {}. Expected: FIELD_NAME=VALUE", s))
        }
    }
}

#[derive(Debug)]
struct UiElement {
    bounds: (i32, i32, i32, i32),
}

fn get_adb_command(serial: Option<&str>) -> Command {
    let mut cmd = Command::new("adb");
    if let Some(s) = serial {
        cmd.arg("-s").arg(s);
    }
    cmd
}

fn get_ui_hierarchy(serial: Option<&str>) -> Result<String, String> {
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

    String::from_utf8(output.stdout)
        .map_err(|e| format!("Invalid UTF-8 in dump file: {}", e))
}

fn parse_bounds(bounds_str: &str) -> Option<(i32, i32, i32, i32)> {
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

fn find_element(xml: &str, selector: &Selector) -> Result<Option<UiElement>, String> {
    let doc = Document::parse(xml)
        .map_err(|e| format!("Failed to parse XML: {}", e))?;

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
                            return Ok(Some(UiElement { bounds }));
                        }
                    }
                }
            }
        }
    }

    Ok(None)
}

fn tap_element(serial: Option<&str>, element: &UiElement) -> Result<(), String> {
    let (x1, y1, x2, y2) = element.bounds;
    let center_x = (x1 + x2) / 2;
    let center_y = (y1 + y2) / 2;

    let output = get_adb_command(serial)
        .args(["shell", "input", "tap", &center_x.to_string(), &center_y.to_string()])
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
        "waitFor" => {
            wait_for_element(cli.serial.as_deref(), &selector, cli.timeout)
                .map(|_| ())
        }
        "tap" => {
            match wait_for_element(cli.serial.as_deref(), &selector, cli.timeout) {
                Ok(element) => tap_element(cli.serial.as_deref(), &element),
                Err(e) => Err(e),
            }
        }
        _ => Err(format!("Unknown command: {}. Supported: waitFor, tap", cli.command)),
    };

    match result {
        Ok(()) => exit(0),
        Err(e) => {
            eprintln!("Error: {}", e);
            exit(1);
        }
    }
}
