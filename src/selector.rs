/// CSS-like selector system for UI elements
///
/// Syntax:
/// - `[attr="value"]` or `[attr=value]` - attribute assertion
/// - `[attr1="v1"][attr2="v2"]` - AND of multiple clauses
/// - `sel1,sel2` - OR of multiple selectors
/// - `:has(cond)` - select nodes with a descendant matching cond
///
/// Examples:
/// - `[text="Submit"]` - element with text="Submit"
/// - `[class=Button][text="OK"]` - element with class="Button" AND text="OK"
/// - `[text="Cancel"],[text="Back"]` - element with text="Cancel" OR text="Back"
/// - `:has([text="Submit"])` - element that has a descendant with text="Submit"
/// - `[class=List]:has([text="Item 1"])` - List element containing "Item 1"
#[derive(Debug, Clone, PartialEq)]
pub enum Selector {
    /// AND of multiple attribute clauses
    And(Vec<AttrClause>),
    /// OR of multiple selectors
    Or(Vec<Selector>),
    /// :has() pseudo-class - matches if node has a descendant matching the inner selector
    Has(Box<Selector>),
    /// Combination of attribute clauses AND :has()
    Complex {
        attrs: Vec<AttrClause>,
        has: Option<Box<Selector>>,
    },
    /// Child combinator - matches if node matches child selector and its parent matches parent selector
    Child {
        parent: Box<Selector>,
        child: Box<Selector>,
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
            Selector::Complex { attrs, has } => {
                let attrs_match = attrs.iter().all(|c| c.matches(node));
                let has_match = match has {
                    Some(inner) => has_descendant_matching(node, inner),
                    None => true,
                };
                attrs_match && has_match
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
            if self.is_eof() || self.peek() == Some(',') {
                break;
            }
            selectors.push(self.parse_child_combinator()?);
            self.skip_whitespace();
            
            if self.peek() == Some(',') {
                self.advance(); // consume ','
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

    /// Parse a complex selector that may have :has() at the end
    fn parse_complex_selector(&mut self) -> Result<Selector, String> {
        self.skip_whitespace();
        
        // Check if it starts with :has()
        if self.peek() == Some(':') {
            return self.parse_has_selector();
        }
        
        // Parse attribute clauses
        let clauses = self.parse_attr_clauses()?;
        
        self.skip_whitespace();
        
        // Check for :has() after attribute clauses
        if self.peek() == Some(':') && self.input[self.pos..].starts_with(":has(") {
            let has_selector = self.parse_has_selector_suffix()?;
            if clauses.is_empty() {
                Ok(Selector::Has(has_selector))
            } else {
                Ok(Selector::Complex {
                    attrs: clauses,
                    has: Some(has_selector),
                })
            }
        } else if clauses.is_empty() {
            Err("Expected attribute clause or :has()".to_string())
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

    /// Parse :has() as a suffix to an existing selector
    fn parse_has_selector_suffix(&mut self) -> Result<Box<Selector>, String> {
        self.expect_str(":has(")?;
        let inner = self.parse_inner_selector()?;
        self.expect_char(')')?;
        // Return the inner selector directly, not wrapped in Has
        // The Complex variant already implies "has a descendant matching"
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
    fn parse_attr_clauses(&mut self) -> Result<Vec<AttrClause>, String> {
        let mut clauses = vec![];
        
        loop {
            self.skip_whitespace();
            if self.peek() != Some('[') {
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
    fn parse_unquoted_value(&mut self) -> Result<String, String> {
        let start = self.pos;
        
        while let Some(c) = self.peek() {
            if c == ']' || c == ',' || c.is_whitespace() {
                break;
            }
            self.advance();
        }
        
        if start == self.pos {
            return Err(format!("Expected value at position {}", self.pos));
        }
        
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
            Selector::Complex { attrs, has } => {
                assert_eq!(attrs.len(), 1);
                assert!(has.is_some());
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
}
