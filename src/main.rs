mod adb_utils;
mod commands;
mod selector;
mod ui_element;

use clap::Parser;
use commands::{
    double_tap_element, input_text_element, long_tap_element, scroll_until_visible, tap_element,
    wait_for_element, wait_for_elements,
};
use selector::Selector;
use std::process::exit;

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

    /// Remote ADB server address in host:port format (e.g., 127.0.0.1:5037)
    #[arg(
        long,
        help = "Remote ADB server address (host:port)",
        long_help = r#"Remote ADB server address in host:port format.

Connects to a remote ADB server instead of the local one.
Example: --remote 127.0.0.1:5037
Both host and port must be specified."#,
        help_heading = "Common Parameters",
        display_order = 5
    )]
    remote: Option<String>,

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
            cli.remote.as_deref(),
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
            match wait_for_element(cli.serial.as_deref(), cli.remote.as_deref(), &selector, cli.timeout) {
                Ok(element) => tap_element(cli.serial.as_deref(), cli.remote.as_deref(), &element),
                Err(e) => Err(e),
            }
        }
        BochiCommand::InputText => match cli.text {
            Some(text) => match wait_for_element(cli.serial.as_deref(), cli.remote.as_deref(), &selector, cli.timeout) {
                Ok(element) => input_text_element(cli.serial.as_deref(), cli.remote.as_deref(), &element, &text),
                Err(e) => Err(e),
            },
            None => Err("--text parameter is required for inputText command".to_string()),
        },
        BochiCommand::LongTap => {
            match wait_for_element(cli.serial.as_deref(), cli.remote.as_deref(), &selector, cli.timeout) {
                Ok(element) => long_tap_element(cli.serial.as_deref(), cli.remote.as_deref(), &element, 1000),
                Err(e) => Err(e),
            }
        }
        BochiCommand::DoubleTap => {
            match wait_for_element(cli.serial.as_deref(), cli.remote.as_deref(), &selector, cli.timeout) {
                Ok(element) => double_tap_element(cli.serial.as_deref(), cli.remote.as_deref(), &element),
                Err(e) => Err(e),
            }
        }
        BochiCommand::ScrollUp => match cli.scroll_target {
            Some(target_str) => match Selector::parse(&target_str) {
                Ok(target_selector) => scroll_until_visible(
                    cli.serial.as_deref(),
                    cli.remote.as_deref(),
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
                    cli.remote.as_deref(),
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
