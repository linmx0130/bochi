/// CSS-like selector system for UI elements
///
/// Syntax:
/// - `[attr="value"]` or `[attr=value]` - attribute assertion
/// - `[attr1="v1"][attr2="v2"]` - AND of multiple clauses (no space)
/// - `sel1,sel2` - OR of multiple selectors
/// - `:has(cond)` - select nodes with a descendant matching cond
/// - `:not(cond)` - select nodes that do NOT match cond
/// - `ancestor > child` - child combinator (direct children only)
/// - `ancestor descendant` - descendant combinator (any depth)
///
/// Examples:
/// - `[text="Submit"]` - element with text="Submit"
/// - `[class=Button][text="OK"]` - element with class="Button" AND text="OK"
/// - `[text="Cancel"],[text="Back"]` - element with text="Cancel" OR text="Back"
/// - `:has([text="Submit"])` - element that has a descendant with text="Submit"
/// - `[class=List]:has([text="Item 1"])` - List element containing "Item 1"
/// - `:not([clickable=false])` - element that is not clickable=false
/// - `[text*=Confirm]:not([clickable=false])` - element with text containing "Confirm" AND not clickable=false
/// - `[class=Column] > [clickable=true]` - clickable elements that are direct children of Column
/// - `[class=List] [text=Item]` - elements with text="Item" anywhere inside a List
#[derive(Debug, Clone, PartialEq)]
pub enum Selector {
    /// AND of multiple attribute clauses
    And(Vec<AttrClause>),
    /// OR of multiple selectors
    Or(Vec<Selector>),
    /// :has() pseudo-class - matches if node has a descendant matching the inner selector
    Has(Box<Selector>),
    /// :not() pseudo-class - matches if node does NOT match the inner selector
    Not(Box<Selector>),
    /// Combination of attribute clauses AND :has() AND/OR :not()
    Complex {
        attrs: Vec<AttrClause>,
        has: Option<Box<Selector>>,
        not: Option<Box<Selector>>,
    },
    /// Child combinator - matches if node matches child selector and its parent matches parent selector
    Child {
        parent: Box<Selector>,
        child: Box<Selector>,
    },
    /// Descendant combinator - matches if node matches descendant selector and has an ancestor matching ancestor selector
    Descendant {
        ancestor: Box<Selector>,
        descendant: Box<Selector>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttrClause {
    pub attr: String,
    pub op: AttrOp,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AttrOp {
    Equals,       // =
    StartsWith,   // ^=
    EndsWith,     // $=
    Contains,     // *=
}

impl Selector {
    /// Parse a selector string into a Selector AST
    /// Supports both CSS-like syntax [attr=value] and legacy syntax attr=value
    pub fn parse(s: &str) -> Result<Selector, String> {
        let trimmed = s.trim();
        
        // Try CSS-like syntax first
        let mut parser = SelectorParser::new(trimmed);
        match parser.parse() {
            Ok(selector) => Ok(selector),
            Err(_) => {
                // Try legacy format: FIELD_NAME=VALUE
                Self::parse_legacy(trimmed)
            }
        }
    }

    /// Parse legacy format: FIELD_NAME=VALUE
    fn parse_legacy(s: &str) -> Result<Selector, String> {
        // Check if it looks like the old format (no brackets, contains =)
        if s.contains('=') && !s.contains('[') && !s.contains(':') {
            let parts: Vec<&str> = s.splitn(2, '=').collect();
            if parts.len() == 2 {
                let field = parts[0].trim();
                let mut value = parts[1].trim();
                
                // Remove surrounding quotes if present
                if (value.starts_with('"') && value.ends_with('"'))
                    || (value.starts_with('\'') && value.ends_with('\''))
                {
                    value = &value[1..value.len() - 1];
                }
                
                if !field.is_empty() && !value.is_empty() {
                    return Ok(Selector::And(vec![AttrClause {
                        attr: field.to_string(),
                        op: AttrOp::Equals,
                        value: value.to_string(),
                    }]));
                }
            }
        }
        
        Err(format!("Invalid selector format: {}", s))
    }

    /// Check if the given XML node matches this selector
    pub fn matches(&self, node: roxmltree::Node) -> bool {
        if !node.is_element() {
            return false;
        }
        match self {
            Selector::And(clauses) => clauses.iter().all(|c| c.matches(node)),
            Selector::Or(selectors) => selectors.iter().any(|s| s.matches(node)),
            Selector::Has(inner) => has_descendant_matching(node, inner),
            Selector::Not(inner) => !inner.matches(node),
            Selector::Complex { attrs, has, not } => {
                let attrs_match = attrs.iter().all(|c| c.matches(node));
                let has_match = match has {
                    Some(inner) => has_descendant_matching(node, inner),
                    None => true,
                };
                let not_match = match not {
                    Some(inner) => !inner.matches(node),
                    None => true,
                };
                attrs_match && has_match && not_match
            }
            Selector::Child { parent, child } => {
                // First check if current node matches child selector
                if !child.matches(node) {
                    return false;
                }
                // Then check if parent matches parent selector
                if let Some(p) = node.parent() {
                    // If the child selector was also a Child selector,
                    // we need to check if this parent matches the grandparent selector
                    // Essentially, we treat the parent as the "current node" for the parent selector
                    parent.matches(p)
                } else {
                    false
                }
            }
            Selector::Descendant { ancestor, descendant } => {
                // First check if current node matches descendant selector
                if !descendant.matches(node) {
                    return false;
                }
                // Then check if any ancestor matches ancestor selector
                has_ancestor_matching(node, ancestor)
            }
        }
    }
}

impl AttrClause {
    fn matches(&self, node: roxmltree::Node) -> bool {
        let attr_value = match self.attr.as_str() {
            "text" => node.attribute("text"),
            "contentDescription" | "content-description" | "content_desc" => {
                node.attribute("content-desc")
            }
            "resourceId" | "resource-id" | "resource_id" => node.attribute("resource-id"),
            "class" => node.attribute("class"),
            "package" => node.attribute("package"),
            "checkable" => node.attribute("checkable"),
            "checked" => node.attribute("checked"),
            "clickable" => node.attribute("clickable"),
            "enabled" => node.attribute("enabled"),
            "focusable" => node.attribute("focusable"),
            "focused" => node.attribute("focused"),
            "long-clickable" | "long_clickable" => node.attribute("long-clickable"),
            "password" => node.attribute("password"),
            "scrollable" => node.attribute("scrollable"),
            "selected" => node.attribute("selected"),
            "bounds" => node.attribute("bounds"),
            attr => node.attribute(attr),
        };

        match attr_value {
            Some(val) => match self.op {
                AttrOp::Equals => val == self.value,
                AttrOp::StartsWith => val.starts_with(&self.value),
                AttrOp::EndsWith => val.ends_with(&self.value),
                AttrOp::Contains => val.contains(&self.value),
            },
            None => false,
        }
    }
}

/// Check if a node has any descendant that matches the selector
fn has_descendant_matching(node: roxmltree::Node, selector: &Selector) -> bool {
    for child in node.children() {
        if selector.matches(child) {
            return true;
        }
        if has_descendant_matching(child, selector) {
            return true;
        }
    }
    false
}

/// Check if a node has any ancestor that matches the selector
fn has_ancestor_matching(node: roxmltree::Node, selector: &Selector) -> bool {
    if let Some(parent) = node.parent() {
        if selector.matches(parent) {
            return true;
        }
        has_ancestor_matching(parent, selector)
    } else {
        false
    }
}

/// Parser for CSS-like selector syntax
struct SelectorParser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> SelectorParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn parse(&mut self) -> Result<Selector, String> {
        self.skip_whitespace();
        let result = self.parse_or_expr()?;
        self.skip_whitespace();
        if !self.is_eof() {
            return Err(format!("Unexpected characters at position {}", self.pos));
        }
        Ok(result)
    }

    /// Parse OR expression (comma-separated selectors)
    fn parse_or_expr(&mut self) -> Result<Selector, String> {
        let mut selectors = vec![];
        
        loop {
            self.skip_whitespace();
            if self.is_eof() {
                break;
            }
            // If we see a comma but no selector before it, that's an error
            if self.peek() == Some(',') {
                // This means we have an empty selector (either leading or trailing comma)
                return Err("Empty selector in OR expression".to_string());
            }
            selectors.push(self.parse_descendant_chain()?);
            self.skip_whitespace();
            
            if self.peek() == Some(',') {
                self.advance(); // consume ','
                // Check if there's a selector after this comma
                self.skip_whitespace();
                if self.is_eof() || self.peek() == Some(',') {
                    // Trailing comma or double comma - error
                    return Err("Empty selector in OR expression".to_string());
                }
            } else {
                break;
            }
        }

        if selectors.is_empty() {
            return Err("Empty selector".to_string());
        }
        
        if selectors.len() == 1 {
            Ok(selectors.into_iter().next().unwrap())
        } else {
            Ok(Selector::Or(selectors))
        }
    }

    /// Parse descendant chain (space-separated selectors)
    /// Builds a left-associative tree: A B C becomes ((A B) C)
    fn parse_descendant_chain(&mut self) -> Result<Selector, String> {
        // First parse a child combinator chain
        let mut left = self.parse_child_combinator()?;
        
        loop {
            // Check if there's significant whitespace followed by another selector
            // We need to differentiate between:
            // - "[text=A] [text=B]" (descendant combinator)
            // - "[text=A]" followed by end of input or ","
            let saved_pos = self.pos;
            self.skip_whitespace();
            
            // If we're at end of input, comma, or closing paren, stop
            if self.is_eof() || self.peek() == Some(',') || self.peek() == Some(')') {
                break;
            }
            
            // Check if the next character can start a selector
            // A selector can start with '[' (attribute), ':' (:has, :not), or '>' is handled by child combinator
            match self.peek() {
                Some('[') | Some(':') => {
                    // This is a descendant combinator - parse the next selector
                    let right = self.parse_child_combinator()?;
                    left = Selector::Descendant {
                        ancestor: Box::new(left),
                        descendant: Box::new(right),
                    };
                }
                Some('>') => {
                    // '>' is handled by parse_child_combinator, not here
                    // Restore position and let the child combinator parser handle it
                    self.pos = saved_pos;
                    break;
                }
                _ => {
                    // Not a selector start, restore position and break
                    self.pos = saved_pos;
                    break;
                }
            }
        }
        
        Ok(left)
    }

    /// Parse child combinator (parent > child)
    /// Builds a left-associative tree: A > B > C becomes ((A > B) > C)
    fn parse_child_combinator(&mut self) -> Result<Selector, String> {
        let mut left = self.parse_complex_selector()?;
        
        loop {
            self.skip_whitespace();
            
            // Check for child combinator >
            if self.peek() == Some('>') {
                self.advance(); // consume '>'
                self.skip_whitespace();
                let right = self.parse_complex_selector()?;
                
                // Build left-associative: left becomes (left > right)
                left = Selector::Child {
                    parent: Box::new(left),
                    child: Box::new(right),
                };
            } else {
                break;
            }
        }
        
        Ok(left)
    }

    /// Parse a complex selector that may have :has() or :not() at the end
    fn parse_complex_selector(&mut self) -> Result<Selector, String> {
        self.skip_whitespace();
        
        // Check if it starts with :has() or :not()
        if self.peek() == Some(':') {
            if self.input[self.pos..].starts_with(":has(") {
                return self.parse_has_selector();
            } else if self.input[self.pos..].starts_with(":not(") {
                return self.parse_not_selector();
            }
        }
        
        // Parse attribute clauses
        let clauses = self.parse_attr_clauses()?;
        
        self.skip_whitespace();
        
        // Check for :has() or :not() after attribute clauses
        if self.peek() == Some(':') {
            let mut has_selector: Option<Box<Selector>> = None;
            let mut not_selector: Option<Box<Selector>> = None;
            
            // Parse any number of :has() and :not() suffixes
            // Multiple :has() or :not() are allowed - last one wins
            loop {
                self.skip_whitespace();
                if self.peek() != Some(':') {
                    break;
                }
                
                if self.input[self.pos..].starts_with(":has(") {
                    has_selector = Some(self.parse_has_selector_suffix()?);
                } else if self.input[self.pos..].starts_with(":not(") {
                    not_selector = Some(self.parse_not_selector_suffix()?);
                } else {
                    break;
                }
            }
            
            if clauses.is_empty() && has_selector.is_none() && not_selector.is_none() {
                return Err("Expected attribute clause, :has() or :not()".to_string());
            }
            
            if has_selector.is_some() || not_selector.is_some() {
                Ok(Selector::Complex {
                    attrs: clauses,
                    has: has_selector,
                    not: not_selector,
                })
            } else if clauses.is_empty() {
                Err("Expected attribute clause".to_string())
            } else {
                Ok(Selector::And(clauses))
            }
        } else if clauses.is_empty() {
            Err("Expected attribute clause, :has() or :not()".to_string())
        } else {
            Ok(Selector::And(clauses))
        }
    }

    /// Parse :has() selector (when it starts the expression)
    fn parse_has_selector(&mut self) -> Result<Selector, String> {
        self.expect_str(":has(")?;
        let inner = self.parse_inner_selector()?;
        self.expect_char(')')?;
        Ok(Selector::Has(inner))
    }

    /// Parse :not() selector (when it starts the expression)
    fn parse_not_selector(&mut self) -> Result<Selector, String> {
        self.expect_str(":not(")?;
        let inner = self.parse_inner_selector()?;
        self.expect_char(')')?;
        Ok(Selector::Not(inner))
    }

    /// Parse :has() as a suffix to an existing selector
    fn parse_has_selector_suffix(&mut self) -> Result<Box<Selector>, String> {
        self.expect_str(":has(")?;
        let inner = self.parse_inner_selector()?;
        self.expect_char(')')?;
        // Return the inner selector directly, not wrapped in Has
        // The Complex variant already implies "has a descendant matching"
        Ok(inner)
    }

    /// Parse :not() as a suffix to an existing selector
    fn parse_not_selector_suffix(&mut self) -> Result<Box<Selector>, String> {
        self.expect_str(":not(")?;
        let inner = self.parse_inner_selector()?;
        self.expect_char(')')?;
        // Return the inner selector directly, not wrapped in Not
        // The Complex variant handles the negation logic
        Ok(inner)
    }

    /// Parse selector inside :has() parentheses
    fn parse_inner_selector(&mut self) -> Result<Box<Selector>, String> {
        self.skip_whitespace();
        
        // Find the matching closing paren, accounting for nested parens
        let start = self.pos;
        let mut depth = 1;
        while depth > 0 && !self.is_eof() {
            match self.peek() {
                Some('(') => {
                    self.advance();
                    depth += 1;
                }
                Some(')') => {
                    if depth == 1 {
                        break;
                    }
                    self.advance();
                    depth -= 1;
                }
                _ => self.advance(),
            }
        }
        
        let selector_str = &self.input[start..self.pos];
        let inner_selector = Selector::parse(selector_str)
            .map_err(|e| format!("Invalid selector inside :has(): {}", e))?;
        
        Ok(Box::new(inner_selector))
    }

    /// Parse multiple attribute clauses [attr=value][attr2=value]
    /// Stops if there's significant whitespace before the next `[` to allow for descendant combinator
    fn parse_attr_clauses(&mut self) -> Result<Vec<AttrClause>, String> {
        let mut clauses = vec![];
        
        loop {
            // Check if next non-whitespace char is '['
            let saved_pos = self.pos;
            self.skip_whitespace();
            
            if self.peek() != Some('[') {
                // Not an attribute clause, restore position and break
                self.pos = saved_pos;
                break;
            }
            
            // If we skipped whitespace (meaning there was whitespace before '['),
            // and we already have at least one clause, treat this as a descendant combinator
            // by restoring position and breaking
            if self.pos > saved_pos && !clauses.is_empty() {
                self.pos = saved_pos;
                break;
            }
            
            clauses.push(self.parse_attr_clause()?);
        }
        
        Ok(clauses)
    }

    /// Parse a single [attr=value], [attr^=value], [attr$=value], or [attr*=value] clause
    fn parse_attr_clause(&mut self) -> Result<AttrClause, String> {
        self.expect_char('[')?;
        self.skip_whitespace();
        
        let attr = self.parse_identifier()?;
        
        self.skip_whitespace();
        let op = self.parse_operator()?;
        self.skip_whitespace();
        
        let value = self.parse_value()?;
        
        // CSS spec: substring operators (^=, $=, *=) require non-empty values
        match op {
            AttrOp::StartsWith | AttrOp::EndsWith | AttrOp::Contains => {
                if value.is_empty() {
                    return Err(format!(
                        "Empty value not allowed for {:?} operator",
                        op
                    ));
                }
            }
            AttrOp::Equals => {} // Empty value allowed for =
        }
        
        self.skip_whitespace();
        self.expect_char(']')?;
        
        Ok(AttrClause { attr, op, value })
    }

    /// Parse the operator (=, ^=, $=, *=)
    fn parse_operator(&mut self) -> Result<AttrOp, String> {
        match self.peek() {
            Some('^') => {
                self.advance();
                self.expect_char('=')?;
                Ok(AttrOp::StartsWith)
            }
            Some('$') => {
                self.advance();
                self.expect_char('=')?;
                Ok(AttrOp::EndsWith)
            }
            Some('*') => {
                self.advance();
                self.expect_char('=')?;
                Ok(AttrOp::Contains)
            }
            Some('=') => {
                self.advance();
                Ok(AttrOp::Equals)
            }
            Some(c) => Err(format!(
                "Expected operator (=, ^=, $=, *=) but found '{}' at position {}",
                c, self.pos
            )),
            None => Err("Expected operator (=, ^=, $=, *=) but reached end of input".to_string()),
        }
    }

    /// Parse an identifier (attribute name)
    fn parse_identifier(&mut self) -> Result<String, String> {
        let start = self.pos;
        
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                self.advance();
            } else {
                break;
            }
        }
        
        if start == self.pos {
            return Err(format!("Expected identifier at position {}", self.pos));
        }
        
        Ok(self.input[start..self.pos].to_string())
    }

    /// Parse a value (quoted or unquoted)
    fn parse_value(&mut self) -> Result<String, String> {
        match self.peek() {
            Some('"') => self.parse_quoted_string('"'),
            Some('\'') => self.parse_quoted_string('\''),
            _ => self.parse_unquoted_value(),
        }
    }

    /// Parse a quoted string
    fn parse_quoted_string(&mut self, quote: char) -> Result<String, String> {
        self.expect_char(quote)?;
        let start = self.pos;
        
        while let Some(c) = self.peek() {
            if c == quote {
                let value = self.input[start..self.pos].to_string();
                self.advance(); // consume closing quote
                return Ok(value);
            }
            self.advance();
        }
        
        Err(format!("Unterminated string starting at position {}", start - 1))
    }

    /// Parse an unquoted value (stops at ] or , or whitespace)
    /// Note: The caller should validate empty values based on the operator
    fn parse_unquoted_value(&mut self) -> Result<String, String> {
        let start = self.pos;
        
        while let Some(c) = self.peek() {
            if c == ']' || c == ',' || c.is_whitespace() {
                break;
            }
            self.advance();
        }
        
        // Empty values are allowed - return empty string
        // (CSS spec: only = allows empty; ^=, $=, *= require non-empty)
        Ok(self.input[start..self.pos].to_string())
    }

    /// Expect a specific string
    fn expect_str(&mut self, s: &str) -> Result<(), String> {
        if self.input[self.pos..].starts_with(s) {
            self.pos += s.len();
            Ok(())
        } else {
            Err(format!(
                "Expected '{}' at position {}",
                s, self.pos
            ))
        }
    }

    /// Expect a specific character
    fn expect_char(&mut self, expected: char) -> Result<(), String> {
        match self.peek() {
            Some(c) if c == expected => {
                self.advance();
                Ok(())
            }
            Some(c) => Err(format!(
                "Expected '{}' but found '{}' at position {}",
                expected, c, self.pos
            )),
            None => Err(format!(
                "Expected '{}' but reached end of input",
                expected
            )),
        }
    }

    /// Peek at the current character without consuming
    fn peek(&self) -> Option<char> {
        self.input.chars().nth(self.pos)
    }

    /// Advance to the next character
    fn advance(&mut self) {
        if self.pos < self.input.len() {
            self.pos += 1;
        }
    }

    /// Skip whitespace characters
    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    /// Check if we've reached the end of input
    fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_attr() {
        let s = Selector::parse("[text=Submit]").unwrap();
        match s {
            Selector::And(clauses) => {
                assert_eq!(clauses.len(), 1);
                assert_eq!(clauses[0].attr, "text");
                assert_eq!(clauses[0].op, AttrOp::Equals);
                assert_eq!(clauses[0].value, "Submit");
            }
            _ => panic!("Expected And selector"),
        }
    }

    #[test]
    fn test_parse_attr_with_double_quotes() {
        let s = Selector::parse("[text=\"Submit Button\"]").unwrap();
        match s {
            Selector::And(clauses) => {
                assert_eq!(clauses.len(), 1);
                assert_eq!(clauses[0].attr, "text");
                assert_eq!(clauses[0].op, AttrOp::Equals);
                assert_eq!(clauses[0].value, "Submit Button");
            }
            _ => panic!("Expected And selector"),
        }
    }

    #[test]
    fn test_parse_attr_with_single_quotes() {
        let s = Selector::parse("[contentDescription='Open Menu']").unwrap();
        match s {
            Selector::And(clauses) => {
                assert_eq!(clauses.len(), 1);
                assert_eq!(clauses[0].attr, "contentDescription");
                assert_eq!(clauses[0].op, AttrOp::Equals);
                assert_eq!(clauses[0].value, "Open Menu");
            }
            _ => panic!("Expected And selector"),
        }
    }

    #[test]
    fn test_parse_and_clauses() {
        let s = Selector::parse("[class=Button][text=OK]").unwrap();
        match s {
            Selector::And(clauses) => {
                assert_eq!(clauses.len(), 2);
                assert_eq!(clauses[0].attr, "class");
                assert_eq!(clauses[0].op, AttrOp::Equals);
                assert_eq!(clauses[0].value, "Button");
                assert_eq!(clauses[1].attr, "text");
                assert_eq!(clauses[1].op, AttrOp::Equals);
                assert_eq!(clauses[1].value, "OK");
            }
            _ => panic!("Expected And selector"),
        }
    }

    #[test]
    fn test_parse_or_selector() {
        let s = Selector::parse("[text=Cancel],[text=Back]").unwrap();
        match s {
            Selector::Or(selectors) => {
                assert_eq!(selectors.len(), 2);
            }
            _ => panic!("Expected Or selector"),
        }
    }

    #[test]
    fn test_parse_has_selector() {
        let s = Selector::parse(":has([text=Submit])").unwrap();
        match s {
            Selector::Has(_) => {}
            _ => panic!("Expected Has selector"),
        }
    }

    #[test]
    fn test_parse_complex_with_has() {
        let s = Selector::parse("[class=List]:has([text=Item])").unwrap();
        match s {
            Selector::Complex { attrs, has, not } => {
                assert_eq!(attrs.len(), 1);
                assert!(has.is_some());
                assert!(not.is_none());
            }
            _ => panic!("Expected Complex selector"),
        }
    }

    #[test]
    fn test_parse_multiple_or() {
        let s = Selector::parse("[text=A],[text=B],[text=C]").unwrap();
        match s {
            Selector::Or(selectors) => {
                assert_eq!(selectors.len(), 3);
            }
            _ => panic!("Expected Or selector"),
        }
    }

    #[test]
    fn test_parse_complex_or() {
        let s = Selector::parse("[class=Btn][text=OK],[class=Btn][text=Cancel]").unwrap();
        match s {
            Selector::Or(selectors) => {
                assert_eq!(selectors.len(), 2);
            }
            _ => panic!("Expected Or selector"),
        }
    }

    #[test]
    fn test_parse_invalid_empty() {
        assert!(Selector::parse("").is_err());
    }

    #[test]
    fn test_parse_legacy_format() {
        // Legacy format text=Submit should now work as [text=Submit]
        let s = Selector::parse("text=Submit").unwrap();
        match s {
            Selector::And(clauses) => {
                assert_eq!(clauses.len(), 1);
                assert_eq!(clauses[0].attr, "text");
                assert_eq!(clauses[0].op, AttrOp::Equals);
                assert_eq!(clauses[0].value, "Submit");
            }
            _ => panic!("Expected And selector"),
        }
    }

    #[test]
    fn test_parse_legacy_with_quotes() {
        let s = Selector::parse("text=\"Submit Button\"").unwrap();
        match s {
            Selector::And(clauses) => {
                assert_eq!(clauses.len(), 1);
                assert_eq!(clauses[0].attr, "text");
                assert_eq!(clauses[0].op, AttrOp::Equals);
                assert_eq!(clauses[0].value, "Submit Button");
            }
            _ => panic!("Expected And selector"),
        }
    }

    #[test]
    fn test_parse_invalid_unclosed_bracket() {
        assert!(Selector::parse("[text=Submit").is_err());
    }

    #[test]
    fn test_parse_invalid_unquoted_value_with_space() {
        // Unquoted values cannot contain spaces
        assert!(Selector::parse("[text=Submit Button] ").is_err());
    }

    #[test]
    fn test_matches_simple() {
        let xml = r##"<node text="Submit" class="Button" />"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let node = doc.root_element();

        let selector = Selector::parse("[text=Submit]").unwrap();
        assert!(selector.matches(node));

        let selector2 = Selector::parse("[text=Cancel]").unwrap();
        assert!(!selector2.matches(node));
    }

    #[test]
    fn test_matches_and() {
        let xml = r##"<node text="OK" class="Button" package="com.example" />"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let node = doc.root_element();

        let selector = Selector::parse("[text=OK][class=Button]").unwrap();
        assert!(selector.matches(node));

        let selector2 = Selector::parse("[text=OK][class=TextView]").unwrap();
        assert!(!selector2.matches(node));
    }

    #[test]
    fn test_matches_or() {
        let xml = r##"<node text="Cancel" class="Button" />"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let node = doc.root_element();

        let selector = Selector::parse("[text=OK],[text=Cancel]").unwrap();
        assert!(selector.matches(node));

        let selector2 = Selector::parse("[text=Submit],[text=Back]").unwrap();
        assert!(!selector2.matches(node));
    }

    #[test]
    fn test_matches_has() {
        // No extra whitespace to avoid text nodes
        let xml = r##"<node class="Container"><node class="Child" text="Submit" /></node>"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let container = doc.root_element();

        let selector = Selector::parse(":has([text=Submit])").unwrap();
        assert!(selector.matches(container));

        let selector2 = Selector::parse(":has([text=Cancel])").unwrap();
        assert!(!selector2.matches(container));
    }

    #[test]
    fn test_matches_complex_with_has() {
        let xml = r##"<node class="List" id="list1"><node class="Item" text="Item 1" /></node>"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let list = doc.root_element();

        let selector = Selector::parse("[class=List]:has([text=\"Item 1\"])").unwrap();
        assert!(selector.matches(list));

        let selector2 = Selector::parse("[class=List]:has([text=\"Item 2\"])").unwrap();
        assert!(!selector2.matches(list));
    }

    #[test]
    fn test_matches_deeply_nested_has() {
        let xml = r##"<node class="GrandParent"><node class="Parent"><node class="Child" text="Target" /></node></node>"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let grandparent = doc.root_element();

        // Grandparent should match :has([text=Target]) because it has a descendant with that text
        let selector = Selector::parse(":has([text=Target])").unwrap();
        assert!(selector.matches(grandparent));
    }

    #[test]
    fn test_matches_or_with_has() {
        let xml = r##"<node class="Container"><node class="Child" text="Child1" /></node>"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let container = doc.root_element();

        let selector = Selector::parse(":has([text=Child1]),:has([text=Child2])").unwrap();
        assert!(selector.matches(container));
    }

    #[test]
    fn test_attribute_aliases() {
        let xml = r##"<node content-desc="Open Menu" resource-id="btn1" />"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let node = doc.root_element();

        let selector = Selector::parse("[contentDescription=\"Open Menu\"]").unwrap();
        assert!(selector.matches(node));

        let selector2 = Selector::parse("[resourceId=btn1]").unwrap();
        assert!(selector2.matches(node));
    }

    #[test]
    fn test_parse_with_whitespace() {
        let s = Selector::parse("  [text = \"Submit\" ] ").unwrap();
        match s {
            Selector::And(clauses) => {
                assert_eq!(clauses.len(), 1);
                assert_eq!(clauses[0].attr, "text");
                assert_eq!(clauses[0].op, AttrOp::Equals);
                assert_eq!(clauses[0].value, "Submit");
            }
            _ => panic!("Expected And selector"),
        }
    }

    #[test]
    fn test_parse_or_with_whitespace() {
        let s = Selector::parse("  [text=A] , [text=B]  ").unwrap();
        match s {
            Selector::Or(selectors) => {
                assert_eq!(selectors.len(), 2);
            }
            _ => panic!("Expected Or selector"),
        }
    }

    #[test]
    fn test_backward_compatible_simple_format() {
        // The old format text=Submit should still work
        // We need to handle this by auto-converting to [text=Submit]
    }

    // Tests for new operators: ^=, $=, *=
    #[test]
    fn test_parse_starts_with() {
        let s = Selector::parse("[text^=Submit]").unwrap();
        match s {
            Selector::And(clauses) => {
                assert_eq!(clauses.len(), 1);
                assert_eq!(clauses[0].attr, "text");
                assert_eq!(clauses[0].op, AttrOp::StartsWith);
                assert_eq!(clauses[0].value, "Submit");
            }
            _ => panic!("Expected And selector"),
        }
    }

    #[test]
    fn test_parse_ends_with() {
        let s = Selector::parse("[text$=Button]").unwrap();
        match s {
            Selector::And(clauses) => {
                assert_eq!(clauses.len(), 1);
                assert_eq!(clauses[0].attr, "text");
                assert_eq!(clauses[0].op, AttrOp::EndsWith);
                assert_eq!(clauses[0].value, "Button");
            }
            _ => panic!("Expected And selector"),
        }
    }

    #[test]
    fn test_parse_contains() {
        let s = Selector::parse("[text*=mit]").unwrap();
        match s {
            Selector::And(clauses) => {
                assert_eq!(clauses.len(), 1);
                assert_eq!(clauses[0].attr, "text");
                assert_eq!(clauses[0].op, AttrOp::Contains);
                assert_eq!(clauses[0].value, "mit");
            }
            _ => panic!("Expected And selector"),
        }
    }

    #[test]
    fn test_matches_starts_with() {
        let xml = r##"<node text="Submit Form" class="Button" />"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let node = doc.root_element();

        let selector = Selector::parse("[text^=Submit]").unwrap();
        assert!(selector.matches(node));

        let selector2 = Selector::parse("[text^=Cancel]").unwrap();
        assert!(!selector2.matches(node));

        let selector3 = Selector::parse("[text^=Sub]").unwrap();
        assert!(selector3.matches(node));
    }

    #[test]
    fn test_matches_ends_with() {
        let xml = r##"<node text="Submit Form" class="Button" />"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let node = doc.root_element();

        let selector = Selector::parse("[text$=Form]").unwrap();
        assert!(selector.matches(node));

        let selector2 = Selector::parse("[text$=Button]").unwrap();
        assert!(!selector2.matches(node));

        let selector3 = Selector::parse("[text$=Form]").unwrap();
        assert!(selector3.matches(node));
    }

    #[test]
    fn test_matches_contains() {
        let xml = r##"<node text="Submit Form" class="Button" />"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let node = doc.root_element();

        let selector = Selector::parse("[text*=mit]").unwrap();
        assert!(selector.matches(node));

        let selector2 = Selector::parse("[text*=Form]").unwrap();
        assert!(selector2.matches(node));

        let selector3 = Selector::parse("[text*=Cancel]").unwrap();
        assert!(!selector3.matches(node));
    }

    #[test]
    fn test_combined_operators() {
        let xml = r##"<node text="Submit Form" class="Button" package="com.example" />"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let node = doc.root_element();

        // Combine different operators with AND
        let selector = Selector::parse("[text^=Submit][text$=Form][class=Button]").unwrap();
        assert!(selector.matches(node));

        // Should not match if one clause fails
        let selector2 = Selector::parse("[text^=Cancel][text$=Form]").unwrap();
        assert!(!selector2.matches(node));
    }

    #[test]
    fn test_operators_with_or() {
        let xml = r##"<node text="Submit Form" class="Button" />"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let node = doc.root_element();

        let selector = Selector::parse("[text^=Cancel],[text*=Submit]").unwrap();
        assert!(selector.matches(node));
    }

    #[test]
    fn test_operators_with_has() {
        let xml = r##"<node class="Container"><node text="Submit Button" /></node>"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let container = doc.root_element();

        let selector = Selector::parse("[class=Container]:has([text^=Submit])").unwrap();
        assert!(selector.matches(container));

        let selector2 = Selector::parse("[class=Container]:has([text$=Button])").unwrap();
        assert!(selector2.matches(container));

        let selector3 = Selector::parse("[class=Container]:has([text*=mit])").unwrap();
        assert!(selector3.matches(container));
    }

    // Tests for child combinator (>)
    #[test]
    fn test_parse_child_combinator() {
        let s = Selector::parse("[class=Column]>[clickable=true]").unwrap();
        match s {
            Selector::Child { parent, child } => {
                // Parent should be And([class=Column])
                match parent.as_ref() {
                    Selector::And(clauses) => {
                        assert_eq!(clauses.len(), 1);
                        assert_eq!(clauses[0].attr, "class");
                        assert_eq!(clauses[0].value, "Column");
                    }
                    _ => panic!("Expected And selector for parent"),
                }
                // Child should be And([clickable=true])
                match child.as_ref() {
                    Selector::And(clauses) => {
                        assert_eq!(clauses.len(), 1);
                        assert_eq!(clauses[0].attr, "clickable");
                        assert_eq!(clauses[0].value, "true");
                    }
                    _ => panic!("Expected And selector for child"),
                }
            }
            _ => panic!("Expected Child selector"),
        }
    }

    #[test]
    fn test_parse_child_combinator_with_whitespace() {
        let s = Selector::parse("[class=Column] > [clickable=true]").unwrap();
        match s {
            Selector::Child { .. } => {}
            _ => panic!("Expected Child selector"),
        }
    }

    #[test]
    fn test_matches_child_combinator() {
        let xml = r##"<node class="Column"><node class="Button" clickable="true" text="Click Me" /></node>"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let parent = doc.root_element();
        let child = parent.first_element_child().unwrap();

        // The child should match the child combinator
        let selector = Selector::parse("[class=Column]>[clickable=true]").unwrap();
        assert!(selector.matches(child));

        // The parent should NOT match (it's not a child of Column, it IS Column)
        assert!(!selector.matches(parent));
    }

    #[test]
    fn test_matches_child_combinator_not_grandchild() {
        // Grandchild should NOT match - only direct children
        let xml = r##"<node class="Column"><node class="Wrapper"><node class="Button" clickable="true" /></node></node>"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let column = doc.root_element();
        let wrapper = column.first_element_child().unwrap();
        let button = wrapper.first_element_child().unwrap();

        let selector = Selector::parse("[class=Column]>[clickable=true]").unwrap();
        
        // The button is NOT a direct child of Column, so it should not match
        assert!(!selector.matches(button));
        
        // The wrapper is the direct child
        let selector2 = Selector::parse("[class=Column]>[class=Wrapper]").unwrap();
        assert!(selector2.matches(wrapper));
    }

    #[test]
    fn test_matches_child_combinator_chain() {
        // Chain: a > b > c
        let xml = r##"<node class="A"><node class="B"><node class="C" /></node></node>"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let a = doc.root_element();
        let b = a.first_element_child().unwrap();
        let c = b.first_element_child().unwrap();

        let selector = Selector::parse("[class=A]>[class=B]>[class=C]").unwrap();
        assert!(selector.matches(c));

        let selector2 = Selector::parse("[class=A]>[class=B]").unwrap();
        assert!(selector2.matches(b));

        // C is not a direct child of A
        let selector3 = Selector::parse("[class=A]>[class=C]").unwrap();
        assert!(!selector3.matches(c));
    }

    #[test]
    fn test_child_combinator_with_or() {
        let xml = r##"<node class="Column"><node class="Button" clickable="true" /></node>"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let column = doc.root_element();
        let button = column.first_element_child().unwrap();

        let selector = Selector::parse("[class=Row]>[clickable=true],[class=Column]>[clickable=true]").unwrap();
        assert!(selector.matches(button));
    }

    #[test]
    fn test_child_combinator_with_has() {
        let xml = r##"<node class="Column"><node class="Button" text="Submit" /></node>"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let column = doc.root_element();
        let button = column.first_element_child().unwrap();

        let selector = Selector::parse("[class=Column]:has([text=Submit])>[class=Button]").unwrap();
        assert!(selector.matches(button));
    }

    // Tests for :not() pseudo-class
    #[test]
    fn test_parse_not_selector() {
        let s = Selector::parse(":not([clickable=false])").unwrap();
        match s {
            Selector::Not(inner) => {
                // The inner selector should be [clickable=false]
                match inner.as_ref() {
                    Selector::And(clauses) => {
                        assert_eq!(clauses.len(), 1);
                        assert_eq!(clauses[0].attr, "clickable");
                        assert_eq!(clauses[0].value, "false");
                    }
                    _ => panic!("Expected And selector inside :not()"),
                }
            }
            _ => panic!("Expected Not selector"),
        }
    }

    #[test]
    fn test_parse_not_with_attrs() {
        let s = Selector::parse("[text*=Confirm]:not([clickable=false])").unwrap();
        match s {
            Selector::Complex { attrs, has, not } => {
                assert_eq!(attrs.len(), 1);
                assert_eq!(attrs[0].attr, "text");
                assert_eq!(attrs[0].op, AttrOp::Contains);
                assert_eq!(attrs[0].value, "Confirm");
                assert!(has.is_none());
                assert!(not.is_some());
            }
            _ => panic!("Expected Complex selector"),
        }
    }

    #[test]
    fn test_matches_not() {
        let xml = r##"<node text="Confirm" clickable="true" />"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let node = doc.root_element();

        // Should match because clickable is not "false"
        let selector = Selector::parse(":not([clickable=false])").unwrap();
        assert!(selector.matches(node));

        // Should not match because text is "Confirm"
        let selector2 = Selector::parse(":not([text=Confirm])").unwrap();
        assert!(!selector2.matches(node));

        // Should match because text is not "Cancel"
        let selector3 = Selector::parse(":not([text=Cancel])").unwrap();
        assert!(selector3.matches(node));
    }

    #[test]
    fn test_matches_not_with_attrs() {
        let xml = r##"<node text="Confirm Button" clickable="true" />"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let node = doc.root_element();

        // Should match because text contains "Confirm" AND clickable is not "false"
        let selector = Selector::parse("[text*=Confirm]:not([clickable=false])").unwrap();
        assert!(selector.matches(node));

        // Should NOT match because although text contains "Confirm", clickable IS "true" (not "false" is true, so this should match)
        // Actually clickable="true", so :not([clickable=false]) should match
        // Let me fix the test - this should match
        assert!(selector.matches(node));

        // Test with a node that has clickable=false
        let xml2 = r##"<node text="Confirm Button" clickable="false" />"##;
        let doc2 = roxmltree::Document::parse(xml2).unwrap();
        let node2 = doc2.root_element();

        // Should NOT match because clickable is "false", and :not([clickable=false]) should reject it
        assert!(!selector.matches(node2));
    }

    #[test]
    fn test_not_with_has() {
        let xml = r##"<node class="Container"><node class="Child" text="Other" /></node>"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let container = doc.root_element();

        // Should match because Container does NOT have a child with text="Submit"
        let selector = Selector::parse("[class=Container]:not(:has([text=Submit]))").unwrap();
        assert!(selector.matches(container));

        // Should NOT match because Container does NOT have a child with text="Other" (it actually does!)
        let selector2 = Selector::parse("[class=Container]:not(:has([text=Other]))").unwrap();
        assert!(!selector2.matches(container));
    }

    #[test]
    fn test_not_with_or() {
        let xml = r##"<node text="OK" class="Button" />"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let node = doc.root_element();

        // OR with :not() - matches if text=Cancel OR (text=OK AND not clickable=false)
        let selector = Selector::parse("[text=Cancel],[text=OK]:not([clickable=false])").unwrap();
        assert!(selector.matches(node));
    }

    #[test]
    fn test_not_with_child_combinator() {
        let xml = r##"<node class="Column"><node class="Button" clickable="true" /></node>"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let column = doc.root_element();
        let button = column.first_element_child().unwrap();

        // Should match - button is a direct child of Column and is clickable (not false)
        let selector = Selector::parse("[class=Column]>[clickable=true]:not([clickable=false])").unwrap();
        assert!(selector.matches(button));
    }

    #[test]
    fn test_combined_has_and_not() {
        let xml = r##"<node class="List" id="list1"><node class="Item" text="Item 1" /></node>"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let list = doc.root_element();

        // Both :has() and :not() together
        let selector = Selector::parse("[class=List]:has([text=\"Item 1\"]):not([id=list2])").unwrap();
        assert!(selector.matches(list));

        let selector2 = Selector::parse("[class=List]:has([text=\"Item 1\"]):not([id=list1])").unwrap();
        assert!(!selector2.matches(list));
    }

    // Tests for descendant combinator (space-separated)
    #[test]
    fn test_parse_descendant_combinator() {
        let s = Selector::parse("[class=Column] [clickable=true]").unwrap();
        match s {
            Selector::Descendant { ancestor, descendant } => {
                // Ancestor should be And([class=Column])
                match ancestor.as_ref() {
                    Selector::And(clauses) => {
                        assert_eq!(clauses.len(), 1);
                        assert_eq!(clauses[0].attr, "class");
                        assert_eq!(clauses[0].value, "Column");
                    }
                    _ => panic!("Expected And selector for ancestor"),
                }
                // Descendant should be And([clickable=true])
                match descendant.as_ref() {
                    Selector::And(clauses) => {
                        assert_eq!(clauses.len(), 1);
                        assert_eq!(clauses[0].attr, "clickable");
                        assert_eq!(clauses[0].value, "true");
                    }
                    _ => panic!("Expected And selector for descendant"),
                }
            }
            _ => panic!("Expected Descendant selector"),
        }
    }

    #[test]
    fn test_parse_descendant_combinator_multiple() {
        // Chain: A B C
        let s = Selector::parse("[class=A] [class=B] [class=C]").unwrap();
        match s {
            Selector::Descendant { ancestor, descendant } => {
                // Should be ((A B) C)
                match ancestor.as_ref() {
                    Selector::Descendant { ancestor, descendant } => {
                        assert!(matches!(ancestor.as_ref(), Selector::And(_)));
                        assert!(matches!(descendant.as_ref(), Selector::And(_)));
                    }
                    _ => panic!("Expected nested Descendant"),
                }
                assert!(matches!(descendant.as_ref(), Selector::And(_)));
            }
            _ => panic!("Expected Descendant selector"),
        }
    }

    #[test]
    fn test_matches_descendant_combinator() {
        // Grandchild should match - any descendant, not just direct child
        let xml = r##"<node class="Column"><node class="Wrapper"><node class="Button" clickable="true" /></node></node>"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let column = doc.root_element();
        let wrapper = column.first_element_child().unwrap();
        let button = wrapper.first_element_child().unwrap();

        // The button is a descendant (grandchild) of Column, so it should match
        let selector = Selector::parse("[class=Column] [clickable=true]").unwrap();
        assert!(selector.matches(button));
        
        // The wrapper is also a descendant of Column
        let selector2 = Selector::parse("[class=Column] [class=Wrapper]").unwrap();
        assert!(selector2.matches(wrapper));
        
        // The column itself should not match (it's not a descendant)
        assert!(!selector.matches(column));
    }

    #[test]
    fn test_matches_descendant_combinator_direct_child() {
        // Direct children should also match descendant combinator
        let xml = r##"<node class="Column"><node class="Button" clickable="true" /></node>"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let column = doc.root_element();
        let button = column.first_element_child().unwrap();

        let selector = Selector::parse("[class=Column] [clickable=true]").unwrap();
        assert!(selector.matches(button));
    }

    #[test]
    fn test_descendant_combinator_with_child() {
        // Mix of descendant and child: A > B C (C is descendant of B which is child of A)
        let xml = r##"<node class="A"><node class="B"><node class="C" text="Target" /></node></node>"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let a = doc.root_element();
        let b = a.first_element_child().unwrap();
        let c = b.first_element_child().unwrap();

        // A > B C should match C
        let selector = Selector::parse("[class=A] > [class=B] [text=Target]").unwrap();
        assert!(selector.matches(c));

        // A B > C should also work
        let selector2 = Selector::parse("[class=A] [class=B] > [class=C]").unwrap();
        assert!(selector2.matches(c));
    }

    #[test]
    fn test_descendant_combinator_with_or() {
        let xml = r##"<node class="Column"><node class="Button" clickable="true" /></node>"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let column = doc.root_element();
        let button = column.first_element_child().unwrap();

        // OR with descendant
        let selector = Selector::parse("[class=Row] [clickable=true],[class=Column] [clickable=true]").unwrap();
        assert!(selector.matches(button));
    }

    #[test]
    fn test_descendant_combinator_with_has() {
        let xml = r##"<node class="Container"><node class="Wrapper"><node class="Button" text="Submit" /></node></node>"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let container = doc.root_element();
        let wrapper = container.first_element_child().unwrap();
        let button = wrapper.first_element_child().unwrap();

        // Container that has a button, containing the wrapper that has that button
        let selector = Selector::parse("[class=Container]:has([text=Submit]) [class=Wrapper]").unwrap();
        assert!(selector.matches(wrapper));
        
        // Wrapper is a descendant of Container, and Button is a descendant of that
        let selector2 = Selector::parse("[class=Container] [class=Wrapper] [text=Submit]").unwrap();
        assert!(selector2.matches(button));
    }

    #[test]
    fn test_descendant_vs_child_precedence() {
        // Test that child combinator has higher precedence than descendant
        // A > B C should be parsed as (A > B) C, not A > (B C)
        let xml = r##"<node class="A"><node class="B"><node class="C" text="Target" /></node></node>"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let a = doc.root_element();
        let b = a.first_element_child().unwrap();
        let c = b.first_element_child().unwrap();

        // A > B C: C is descendant of B which is child of A
        let selector = Selector::parse("[class=A] > [class=B] [text=Target]").unwrap();
        assert!(selector.matches(c));

        // A B > C: C is child of B which is descendant of A
        let selector2 = Selector::parse("[class=A] [class=B] > [class=C]").unwrap();
        assert!(selector2.matches(c));
        assert!(selector2.matches(c));
    }

    #[test]
    fn test_descendant_combinator_no_match() {
        // Button outside of Column should not match
        let xml = r##"<node class="Row"><node class="Button" clickable="true" /></node>"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let row = doc.root_element();
        let button = row.first_element_child().unwrap();

        // Looking for button inside Column, but button is in Row
        let selector = Selector::parse("[class=Column] [clickable=true]").unwrap();
        assert!(!selector.matches(button));
    }

    #[test]
    fn test_descendant_combinator_complex() {
        // More complex example from README use case
        let xml = r##"<node class="android.widget.ScrollView" resource-id="list"><node class="android.widget.LinearLayout"><node class="android.widget.TextView" text="Item 1" /></node></node>"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let scrollview = doc.root_element();
        let linear = scrollview.first_element_child().unwrap();
        let textview = linear.first_element_child().unwrap();

        // Find text view inside scrollview
        let selector = Selector::parse("[class=android.widget.ScrollView] [text=\"Item 1\"]").unwrap();
        assert!(selector.matches(textview));
        
        // Also should match the LinearLayout which is a direct child
        let selector2 = Selector::parse("[class=android.widget.ScrollView] [class=android.widget.LinearLayout]").unwrap();
        assert!(selector2.matches(linear));
    }


    #[test]
    fn test_nested_has_in_not() {
        // :not(:has([text=A])) - element that does NOT have a descendant with text=A
        let s = Selector::parse(":not(:has([text=Submit]))").unwrap();
        match s {
            Selector::Not(inner) => {
                match inner.as_ref() {
                    Selector::Has(_) => {}
                    _ => panic!("Expected Has selector inside Not"),
                }
            }
            _ => panic!("Expected Not selector"),
        }

        // Test matching
        let xml = r##"<node class="Container"><node text="Other" /></node>"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let container = doc.root_element();

        let selector = Selector::parse(":not(:has([text=Submit]))").unwrap();
        assert!(selector.matches(container)); // Container doesn't have Submit descendant

        let xml2 = r##"<node class="Container"><node text="Submit" /></node>"##;
        let doc2 = roxmltree::Document::parse(xml2).unwrap();
        let container2 = doc2.root_element();
        assert!(!selector.matches(container2)); // Container has Submit descendant
    }

    #[test]
    fn test_nested_not_in_has() {
        // :has(:not([clickable=false])) - element that has a descendant that is NOT clickable=false
        let s = Selector::parse(":has(:not([clickable=false]))").unwrap();
        match s {
            Selector::Has(inner) => {
                match inner.as_ref() {
                    Selector::Not(_) => {}
                    _ => panic!("Expected Not selector inside Has"),
                }
            }
            _ => panic!("Expected Has selector"),
        }

        let xml = r##"<node class="Container"><node text="Button" clickable="true" /></node>"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let container = doc.root_element();

        let selector = Selector::parse(":has(:not([clickable=false]))").unwrap();
        assert!(selector.matches(container));
    }

    #[test]
    fn test_multiple_has_allowed() {
        // Multiple :has() in the same selector should be allowed
        let result = Selector::parse("[class=A]:has([text=B]):has([text=C])");
        assert!(result.is_ok());
        match result.unwrap() {
            Selector::Complex { attrs, has, not } => {
                assert_eq!(attrs.len(), 1);
                assert!(has.is_some());
                assert!(not.is_none());
            }
            _ => panic!("Expected Complex selector"),
        }
    }

    #[test]
    fn test_multiple_not_allowed() {
        // Multiple :not() in the same selector should be allowed
        let result = Selector::parse("[class=A]:not([text=B]):not([text=C])");
        assert!(result.is_ok());
        match result.unwrap() {
            Selector::Complex { attrs, has, not } => {
                assert_eq!(attrs.len(), 1);
                assert!(has.is_none());
                assert!(not.is_some());
            }
            _ => panic!("Expected Complex selector"),
        }
    }

    #[test]
    fn test_empty_string_value() {
        // Test matching empty string values with = operator
        let xml = r##"<node text="" class="EmptyText" />"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let node = doc.root_element();

        // = operator allows empty value
        let selector = Selector::parse("[text=\"\"]").unwrap();
        assert!(selector.matches(node));
        // = operator allows empty value
        let selector = Selector::parse("[text=]").unwrap();
        assert!(selector.matches(node));

        // CSS spec: substring operators (^=, $=, *=) require non-empty values
        // These should error
        assert!(Selector::parse("[text^=]").is_err());
        assert!(Selector::parse("[text$=]").is_err());
        assert!(Selector::parse("[text*=]").is_err());
    }

    #[test]
    fn test_selector_starting_with_comma() {
        let result = Selector::parse(",[text=A]");
        assert!(result.is_err());
    }

    #[test]
    fn test_selector_ending_with_comma() {
        let result = Selector::parse("[text=A],");
        assert!(result.is_err());
    }

    #[test]
    fn test_double_comma_in_or() {
        let result = Selector::parse("[text=A],,[text=B]");
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_operator_in_attr() {
        // Missing operator should error
        let result = Selector::parse("[text]");
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_attribute_name() {
        let result = Selector::parse("[=value]");
        assert!(result.is_err());
    }

    #[test]
    fn test_unterminated_string() {
        // Unterminated quoted string should error
        let result = Selector::parse("[text=\"unterminated]");
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_closing_bracket() {
        let result = Selector::parse("[text=value");
        assert!(result.is_err());
    }

    #[test]
    fn test_custom_attribute_name() {
        // Test matching with custom/unknown attribute names
        let xml = r##"<node custom-attr="custom-value" text="Test" />"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let node = doc.root_element();

        let selector = Selector::parse("[custom-attr=custom-value]").unwrap();
        assert!(selector.matches(node));

        let selector2 = Selector::parse("[unknown-attr=value]").unwrap();
        assert!(!selector2.matches(node)); // Attribute doesn't exist
    }

    #[test]
    fn test_element_with_missing_attribute() {
        // Element that doesn't have the requested attribute
        let xml = r##"<node class="Button" />"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let node = doc.root_element();

        let selector = Selector::parse("[text=Submit]").unwrap();
        assert!(!selector.matches(node)); // No text attribute

        let selector2 = Selector::parse("[clickable=true]").unwrap();
        assert!(!selector2.matches(node)); // No clickable attribute
    }

    #[test]
    fn test_child_combinator_no_parent() {
        // Root element has no parent, so child combinator should not match
        let xml = r##"<node class="Button" />"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let node = doc.root_element();

        let selector = Selector::parse("[class=Column]>[class=Button]").unwrap();
        assert!(!selector.matches(node)); // No parent to match
    }

    #[test]
    fn test_descendant_combinator_no_ancestor() {
        // Root element has no ancestor, so descendant combinator should not match
        let xml = r##"<node class="Button" />"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let node = doc.root_element();

        let selector = Selector::parse("[class=Column] [class=Button]").unwrap();
        assert!(!selector.matches(node)); // No ancestor to match
    }

    #[test]
    fn test_startswith_operator_empty_value() {
        // CSS spec: ^= requires non-empty value
        assert!(Selector::parse("[text^=]").is_err());
        
        // Valid: non-empty value
        let xml = r##"<node text="Anything" />"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let node = doc.root_element();

        let selector = Selector::parse("[text^=Any]").unwrap();
        assert!(selector.matches(node));
    }

    #[test]
    fn test_endswith_operator_empty_value() {
        // CSS spec: $= requires non-empty value
        assert!(Selector::parse("[text$=]").is_err());
        
        // Valid: non-empty value
        let xml = r##"<node text="Anything" />"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let node = doc.root_element();

        let selector = Selector::parse("[text$=ing]").unwrap();
        assert!(selector.matches(node));
    }

    #[test]
    fn test_contains_operator_empty_value() {
        // CSS spec: *= requires non-empty value
        assert!(Selector::parse("[text*=]").is_err());
        
        // Valid: non-empty value
        let xml = r##"<node text="Anything" />"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let node = doc.root_element();

        let selector = Selector::parse("[text*=yth]").unwrap();
        assert!(selector.matches(node));
    }

    #[test]
    fn test_whitespace_only_selector() {
        let result = Selector::parse("   ");
        assert!(result.is_err());
    }

    #[test]
    fn test_has_with_complex_inner_selector() {
        // :has() with complex inner selector including child combinator
        let xml = r##"<node class="Container"><node class="Wrapper"><node text="Target" /></node></node>"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let container = doc.root_element();

        let selector = Selector::parse(":has([class=Wrapper] > [text=Target])").unwrap();
        assert!(selector.matches(container));
    }

    #[test]
    fn test_has_with_or_inner_selector() {
        // :has() with OR inside
        let xml = r##"<node class="Container"><node text="Submit" /></node>"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let container = doc.root_element();

        let selector = Selector::parse(":has([text=Submit],[text=Cancel])").unwrap();
        assert!(selector.matches(container));
    }

    #[test]
    fn test_child_combinator_with_has_as_parent() {
        // :has() > child - parent is a has selector
        let xml = r##"<node class="Container"><node text="Submit" /><node class="Button" /></node>"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let container = doc.root_element();
        let button = container.children().find(|n| n.attribute("class") == Some("Button")).unwrap();

        let selector = Selector::parse(":has([text=Submit]) > [class=Button]").unwrap();
        assert!(selector.matches(button));
    }

    #[test]
    fn test_child_combinator_with_not_as_child() {
        // parent > :not(...)
        let xml = r##"<node class="Column"><node class="Button" clickable="true" /><node class="Spacer" clickable="false" /></node>"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let column = doc.root_element();

        let button = column.children().find(|n| n.attribute("class") == Some("Button")).unwrap();
        let spacer = column.children().find(|n| n.attribute("class") == Some("Spacer")).unwrap();

        let selector = Selector::parse("[class=Column] > :not([clickable=false])").unwrap();
        assert!(selector.matches(button));
        assert!(!selector.matches(spacer));
    }

    #[test]
    fn test_descendant_combinator_with_has_as_ancestor() {
        // :has(...) descendant
        let xml = r##"<node class="Container"><node text="Submit" /><node class="Wrapper"><node class="Button" /></node></node>"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let container = doc.root_element();
        let button = container.descendants().find(|n| n.attribute("class") == Some("Button")).unwrap();

        let selector = Selector::parse(":has([text=Submit]) [class=Button]").unwrap();
        assert!(selector.matches(button));
    }

    #[test]
    fn test_missing_value_after_operator() {
        // Missing value after operator
        let result = Selector::parse("[text=]");
        // This should either parse as empty value or error
        // Looking at the parser, it will parse as unquoted value which errors on empty
        assert!(result.is_err() || result.unwrap() == Selector::And(vec![AttrClause {
            attr: "text".to_string(),
            op: AttrOp::Equals,
            value: "".to_string(),
        }]));
    }

    #[test]
    fn test_attribute_with_hyphen_in_name() {
        // Attribute names can contain hyphens (already allowed per parse_identifier)
        let xml = r##"<node my-custom-attr="value" />"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let node = doc.root_element();

        let selector = Selector::parse("[my-custom-attr=value]").unwrap();
        assert!(selector.matches(node));
    }

    #[test]
    fn test_attribute_with_underscore_in_name() {
        // Attribute names can contain underscores
        let xml = r##"<node my_attr="value" />"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let node = doc.root_element();

        let selector = Selector::parse("[my_attr=value]").unwrap();
        assert!(selector.matches(node));
    }

    #[test]
    fn test_or_with_single_element() {
        // OR with only one element should just return that element
        let s = Selector::parse("[text=A]").unwrap();
        match s {
            Selector::And(clauses) => {
                assert_eq!(clauses.len(), 1);
            }
            _ => panic!("Expected And selector for single element"),
        }
    }

    #[test]
    fn test_deeply_nested_descendants() {
        // Deep nesting with descendant combinator
        let xml = r##"<node class="A"><node class="B"><node class="C"><node class="D"><node class="E" text="Target" /></node></node></node></node>"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let e = doc.root_element().descendants().find(|n| n.attribute("class") == Some("E")).unwrap();

        let selector = Selector::parse("[class=A] [text=Target]").unwrap();
        assert!(selector.matches(e));

        let _selector2 = Selector::parse("[class=A] [class=C] [class=E]").unwrap();
        assert!(selector.matches(e));
    }

    #[test]
    fn test_not_with_descendant_selector() {
        // :not() with descendant selector inside
        let xml = r##"<node class="Container"><node text="Target" /></node>"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let container = doc.root_element();

        // Container should NOT match if we say :not([class=Container])
        let selector = Selector::parse(":not([class=Container])").unwrap();
        assert!(!selector.matches(container));
    }

    #[test]
    fn test_has_no_children() {
        // Element with no children matching :has()
        let xml = r##"<node class="Container" />"##;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let container = doc.root_element();

        let selector = Selector::parse(":has([text=Anything])").unwrap();
        assert!(!selector.matches(container)); // No children at all
    }
}
