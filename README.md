# bochi

A command line tool for AI agents to control Android devices via ADB.

## Features

- Uses `adb shell uiautomator dump` to obtain UI layout information
- Supports element selection by various attributes (text, contentDescription, resourceId, class, package)
- Commands: `waitFor`, `tap`
- Configurable timeout

## Installation

```bash
cargo build --release
```

The binary will be available at `target/release/bochi`.

## Usage

```
bochi [OPTIONS] --selector <SELECTOR> --command <COMMAND>

Options:
  -s, --serial <SERIAL>      Device serial number (optional if only one device)
  -e, --selector <SELECTOR>  Element selector in FIELD_NAME=VALUE format
  -c, --command <COMMAND>    Command to perform: waitFor, tap
  -t, --timeout <TIMEOUT>    Timeout in seconds [default: 30]
  -h, --help                 Print help
```

## Examples

### Wait for an element to appear

```bash
bochi -e 'text=Submit' -c waitFor
```

### Tap an element

```bash
bochi -e 'contentDescription="Open Menu"' -c tap
```

### Use with specific device

```bash
bochi -s emulator-5554 -e 'resource-id=com.example:id/button' -c tap
```

### Set custom timeout

```bash
bochi -e 'text=Loading' -c waitFor -t 60
```

## Selector Format

Selectors use the format `FIELD_NAME=VALUE`. Supported field names:

- `text` - The text content of the element
- `contentDescription` - The content description (maps to `content-desc` in UI Automator)
- `resourceId` - The resource ID (maps to `resource-id` in UI Automator)
- `class` - The class name of the element
- `package` - The package name

Values can be quoted or unquoted:
- `text=Submit`
- `text="Submit"`
- `contentDescription='Open Menu'`

## Exit Codes

- `0` - Success
- `1` - Error (element not found, timeout, ADB error, etc.)

## Requirements

- Android Debug Bridge (ADB) installed and in PATH
- Android device connected and authorized for debugging
