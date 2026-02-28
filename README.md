# bochi

Bochi is a command line tool for AI agents to control Android devices via ADB. Use this tool when you need to interact with Android UI elements programmatically, such as tapping buttons, waiting for elements to appear, or automating Android device interactions. Supports CSS-like selectors with attribute assertions, AND/OR logic, descendant matching, and negation.

## Features

- Uses `adb shell uiautomator dump` to obtain UI layout information
- Supports CSS-like element selectors with attribute assertions, AND/OR logic, descendant matching, and negation
- Commands: `waitFor`, `tap`, `inputText`, `longTap`, `doubleTap`, `scrollUp`, `scrollDown`
- Configurable timeout

## Installation

### Install from crates.io

```bash
cargo install bochi
```

### Build from source

```bash
git clone https://github.com/linmx0130/bochi.git && cd bochi
cargo build --release
```

The binary will be available at `target/release/bochi`.

## Basic Usage

```
bochi [OPTIONS] --selector <SELECTOR> --command <COMMAND>

Options:
  -h, --help  Print help

Common Parameters:
  -s, --serial <SERIAL>
  -e, --selector <SELECTOR>  Element selector. Supports CSS-like syntax
  -c, --command <COMMAND>
  -t, --timeout <TIMEOUT>    [default: 30]

Command-Specific Parameters:
      --text <TEXT>        Text content for inputText command
      --print-descendants  Print the XML of matched elements including their descendants (for waitFor command)
      --scroll-target <SELECTOR>  Target element selector for scrollUp/scrollDown commands
```

### Commands

All commands are executed against the elements matched by the selector. If the element is not found within the specified timeout, an error will be returned. If there are multiple elements matched, the command will be executed against the **first** element.

- `waitFor`: Wait for an element to appear
- `tap`: Tap an element
- `inputText`: Input text into an element
- `longTap`: Long tap (1000ms) an element
- `doubleTap`: Double tap an element
- `scrollUp`: Scroll up until the target element is visible (requires `--scroll-target`)
- `scrollDown`: Scroll down until the target element is visible (requires `--scroll-target`)

## Key Examples

### Wait for an element to appear

```bash
bochi -e '[text=Submit]' -c waitFor
```

### Tap an element

```bash
bochi -e '[contentDescription="Open Menu"]' -c tap
```

### Input text into an element

```bash
bochi -e '[resource-id=com.example:id/username]' -c inputText --text "myusername"
```

### Use with specific device

```bash
bochi -s emulator-5554 -e '[resource-id=com.example:id/button]' -c tap
```

### Set custom timeout

```bash
bochi -e '[text=Loading]' -c waitFor -t 60
```

### Scroll to an element

```bash
bochi -e '[class$=RecyclerView]' -c scrollDown --scroll-target '[text="Item 50"]'
```

## Detailed Documentation (for Agents)

For complete selector syntax, advanced examples, and comprehensive usage instructions, see [SKILL.md](./SKILL.md).

## Exit Codes

- `0` - Success
- `1` - Error (element not found, timeout, ADB error, etc.)

## Requirements

- Android Debug Bridge (ADB) installed and in PATH
- Android device connected and authorized for debugging

## Tips

1. For accurate selection, `resource-id` is the best attribute to query if available.
2. In Jetpack Compose, `testTag` can be exposed as `resource-id` by applying `Modifier.semantics { testTagsAsResourceId = true }`.
3. Adding accurate content descriptions benefits both automated tools and accessibility.
