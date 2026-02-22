---
name: operate-android-devices-with-bochi
description: Bochi is a command line tool for AI agents to control Android devices via ADB. Use this skill when you need to interact with Android UI elements programmatically, such as tapping buttons, waiting for elements to appear, or automating Android device interactions. Supports CSS-like selectors with attribute assertions, AND/OR logic, and descendant matching.
license: MIT
metadata:
  author: linmx0130
---

# bochi

Bochi is a command line tool for AI agents to control Android devices via ADB. Use this skill when you need to interact with Android UI elements programmatically, such as tapping buttons, waiting for elements to appear, or automating Android device interactions. Supports CSS-like selectors with attribute assertions, AND/OR logic, and descendant matching.

## Features

- Uses `adb shell uiautomator dump` to obtain UI layout information
- Supports CSS-like element selectors with attribute assertions, AND/OR logic, and descendant matching
- Commands: `waitFor`, `tap`, `inputText`
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
  -e, --selector <SELECTOR>  Element selector (CSS-like syntax)
  -c, --command <COMMAND>    Command to perform: waitFor, tap, inputText
      --text <TEXT>          Text content for inputText command
  -t, --timeout <TIMEOUT>    Timeout in seconds [default: 30]
      --print-descendants    Print the XML of matched elements including their descendants (for waitFor command)
  -h, --help                 Print help
```

## Selector Syntax

The selector syntax is inspired by CSS selectors:

### Basic Attribute Assertion

Use square brackets to match elements by attribute:

```bash
# Match element with text="Submit"
bochi -e '[text="Submit"]' -c tap

# Match element with class="Button"
bochi -e '[class=Button]' -c tap
```

### Attribute Operators

In addition to exact match (`=`), you can use:

- `^=` - starts with: `[attr^=value]` matches if attribute starts with `value`
- `$=` - ends with: `[attr$=value]` matches if attribute ends with `value`
- `*=` - contains: `[attr*=value]` matches if attribute contains `value`

```bash
# Match text starting with "Submit"
bochi -e '[text^=Submit]' -c tap

# Match text ending with "Button"
bochi -e '[text$=Button]' -c tap

# Match text containing "Search"
bochi -e '[text*=Search]' -c tap

# Combine operators
bochi -e '[class^=android.widget][text*=Save]' -c tap
```

### AND Logic (Multiple Clauses)

Multiple square bracket clauses connected together means AND:

```bash
# Match element with class="android.widget.Button" AND text="OK"
bochi -e '[class=android.widget.Button][text="OK"]' -c tap

# Match element with package="com.example" AND clickable="true"
bochi -e '[package=com.example][clickable=true]' -c tap
```

### OR Logic (Comma-separated)

Use comma `,` to represent OR of multiple conditions:

```bash
# Match element with text="Cancel" OR text="Back"
bochi -e '[text=Cancel],[text=Back]' -c tap

# Match element with text="OK" OR text="Confirm"
bochi -e '[text=OK],[text=Confirm]' -c tap
```

### Descendant Matching (`:has()`)

Use `:has(cond)` to select nodes which have a descendant matching the condition:

```bash
# Match a ScrollView element that contains an item with text="Item 1"
bochi -e '[class=android.widget.ScrollView]:has([text="Item 1"])' -c tap

# Match any element that has a descendant with text="Submit"
bochi -e ':has([text=Submit])' -c tap
```

### Child Combinator (`>`)

Use `>` to select direct children:

```bash
# Match clickable elements that are direct children of a ScrollView
bochi -e '[class=android.widget.ScrollView]>[clickable=true]' -c tap

# Chain child combinators
bochi -e '[class=android.widget.ScrollView]>[class*=Item]>[text=Settings]' -c tap
```

Note: `>` only matches direct children, unlike `:has()` which matches any descendant.

### Complex Selectors

Combine all features for powerful selection:

```bash
# Match Button with text "OK" OR "Confirm"
bochi -e '[class=android.widget.Button][text=OK],[class=android.widgetButton][text=Confirm]' -c tap
```

### Supported Attributes

- `text` - The text content of the element
- `contentDescription` (or `content-desc`, `content_desc`) - The content description
- `resourceId` (or `resource-id`, `resource_id`) - The resource ID
- `class` - The class name of the element
- `package` - The package name
- `checkable`, `checked`, `clickable`, `enabled`, `focusable`, `focused`
- `long-clickable` (or `long_clickable`), `password`, `scrollable`, `selected`
- `bounds` - The bounding rectangle

### Quoting Values

Values can be quoted or unquoted:
- `[text=Submit]` - unquoted
- `[text="Submit Button"]` - double quotes (required for values with spaces)
- `[text='Submit']` - single quotes

## Examples

### Wait for an element to appear

```bash
bochi -e '[text=Submit]' -c waitFor
```

### Wait for an element and print its descendants

```bash
bochi -e '[class=android.widget.ScrollView]' -c waitFor --print-descendants
```

### Tap an element

```bash
bochi -e '[contentDescription="Open Menu"]' -c tap
```

### Input text into an element

```bash
bochi -e '[resource-id=com.example:id/username]' -c inputText --text "myusername"
```

### Tap element with OR condition

```bash
bochi -e '[text=OK],[text=Confirm]' -c tap
```

### Tap a list item within a specific container

```bash
bochi -e '[class$=RecyclerView]:has([text="Settings"])' -c tap
```

### Match text starting with a prefix

```bash
bochi -e '[text^=Loading]' -c waitFor
```

### Match resource-id ending with a suffix

```bash
bochi -e '[resource-id$=submit_button]' -c tap
```

### Match text containing a substring

```bash
bochi -e '[text*=Save Changes]' -c tap
```

### Select direct children

```bash
# Select clickable buttons directly under a toolbar
bochi -e '[class$=Toolbar]>[clickable=true]' -c tap

# Chain: Select Settings item in a RecycleView
bochi -e '[class$=RecyclerView]>[class$=LinearLayout]>[text=Settings]' -c tap
```

### Use with specific device

```bash
bochi -s emulator-5554 -e '[resource-id=com.example:id/button]' -c tap
```

### Set custom timeout

```bash
bochi -e '[text=Loading]' -c waitFor -t 60
```

### Selecting a Button Within a Specific Container
When you need to interact with a button that appears multiple times on the screen (e.g., "Reset" buttons for different layout configurations), you can combine the :has() pseudo-class with the child combinator (>) to precisely target the button within a specific container.

```bash
# Click the "Reset" button within the Portrait Layout section
bochi -e ':has([text*=Portrait]) > [clickable=true]:has([text="Reset"])' -c tap
```
How it works:

1. `:has([text*=Portrait])` - Selects a container element that contains a descendant with text matching "Portrait" (e.g., the "Portrait Layout" card)
2. `>` - The child combinator restricts the search to direct children of the container
3. `[clickable=true]:has([text="Reset"])` - Matches a clickable element that contains the text "Reset"

This pattern is useful when:

• Multiple similar buttons exist on the same screen (e.g., "Edit" or "Reset" buttons for different settings categories)
• You need to distinguish between buttons based on their container context
• Elements don't have unique resource IDs but their parent containers have distinguishing text

## Exit Codes

- `0` - Success
- `1` - Error (element not found, timeout, ADB error, etc.)

## Requirements

- Android Debug Bridge (ADB) installed and in PATH
- Android device connected and authorized for debugging
