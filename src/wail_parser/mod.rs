use crate::json_parser::{Parser as JsonParser, StreamParser};
use crate::json_types::{JsonValue, Number};
use crate::parser_types::*;
use crate::tag_finder::{TagEvent, TagFinder};
use crate::types::*;
use std::collections::HashMap;
use strsim::damerau_levenshtein; // UTF‑8 aware (handles transpositions)

use std::sync::Arc;

use nom_supreme::final_parser::Location;
use std::cell::RefCell;
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum WAILParseError {
    // Syntax errors
    UnexpectedToken {
        found: String,
        location: Location,
    },
    UnexpectedKeyword {
        found: String,
        location: Location,
    },
    UnexpectedEOF {
        expected: String,
        location: Location,
    },
    InvalidIdentifier {
        found: String,
        location: Location,
    },

    SymbolNotFound {
        name: String,
    },

    AmbiguousSymbol {
        name: String,
        matches: Vec<String>,
    },

    // Static analysis errors
    UndefinedType {
        name: String,
        location: Location,
    },
    DuplicateDefinition {
        name: String,
        location: Location,
    },
    MissingMainBlock,
    InvalidTemplateCall {
        template_name: String,
        reason: String,
        location: Location,
    },
    CircularImport {
        path: String,
        chain: Vec<String>,
    },
    InvalidImportPath {
        path: String,
        error: String,
    },
    FileError {
        path: String,
        error: String,
    },
    ImportNotFound {
        name: String,
        path: String,
    },
}

impl std::fmt::Display for WAILParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WAILParseError::CircularImport { path, chain } => {
                write!(f, "Circular import detected: {} in chain {:?}", path, chain)
            }
            // ... handle other variants ...
            _ => write!(f, "{:?}", self),
        }
    }
}

impl std::error::Error for WAILParseError {}

fn count_leading_whitespace(line: &str) -> usize {
    line.chars().take_while(|c| c.is_whitespace()).count()
}

fn adjust_indentation(content: &str, target_indent: usize) -> String {
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return String::new();
    }

    // Find initial whitespace amount from first non-empty line
    let initial_indent = lines
        .iter()
        .find(|line| !line.trim().is_empty())
        .map(|line| count_leading_whitespace(line))
        .unwrap_or(0);

    // Calculate how much to adjust by
    let indent_adjustment = initial_indent.saturating_sub(target_indent);

    // Adjust each line
    lines
        .iter()
        .map(|line| {
            let current_indent = count_leading_whitespace(line);
            if current_indent >= indent_adjustment {
                &line[indent_adjustment..]
            } else {
                line.trim_start()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Debug)]
pub struct WAILParser {
    registry: Arc<RefCell<HashMap<String, WAILField>>>,
    template_registry: Arc<RefCell<HashMap<String, WAILTemplateDef>>>,
    adhoc_obj_ref_id_counter: Arc<RefCell<i64>>,
    adhoc_obj_ids: Arc<RefCell<Vec<String>>>,
    adhoc_obj_refs: Arc<RefCell<HashMap<String, WAILObject>>>,
    // Track object instantiations with their variable names
    object_instances: Arc<RefCell<HashMap<String, WAILObjectInstantiation>>>,
    import_chain: Arc<RefCell<ImportChain>>,
    base_path: PathBuf,
    incremental_parser: Arc<RefCell<Option<Parser>>>,
    current_module: Arc<RefCell<Vec<String>>>,
}

#[derive(Debug)]
pub struct ImportChain {
    // Stack of current import resolution
    stack: Vec<String>,
    // Set of all visited/imported files
    visited: HashSet<String>,
    // Base directory for resolving relative paths
    base_path: PathBuf,
}

impl ImportChain {
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            stack: Vec::new(),
            visited: HashSet::new(),
            base_path,
        }
    }

    pub fn push(&mut self, path: &str) -> Result<bool, WAILParseError> {
        let canonical_path = self.resolve_path(path)?;

        if self.stack.contains(&canonical_path) {
            return Err(WAILParseError::CircularImport {
                path: canonical_path.clone(),
                chain: self.stack.clone(),
            });
        }

        let is_new = self.visited.insert(canonical_path.clone());
        self.stack.push(canonical_path);

        Ok(is_new) // true = first time seen, false = already visited
    }

    pub fn pop(&mut self) {
        self.stack.pop();
    }

    fn resolve_path(&self, path: &str) -> Result<String, WAILParseError> {
        let path = PathBuf::from(path);

        let resolved = if path.is_absolute() {
            path
        } else {
            self.base_path.join(path)
        };

        // Clean up the path without checking file existence
        let cleaned = resolved
            .components()
            .fold(PathBuf::new(), |mut cleaned, component| {
                match component {
                    std::path::Component::ParentDir => {
                        cleaned.pop(); // Remove last component for ..
                    }
                    std::path::Component::Normal(part) => {
                        cleaned.push(part);
                    }
                    std::path::Component::RootDir => {
                        cleaned.push("/");
                    }
                    _ => {} // Skip . and prefix components
                }
                cleaned
            });

        Ok(cleaned.to_string_lossy().to_string())
    }
}

#[derive(Debug, PartialEq)]
pub enum WAILFileType {
    Library,     // .lib.wail - only definitions allowed
    Application, // .wail - requires main block
}

#[derive(Debug, Clone, PartialEq)]
pub struct WAILImport {
    pub items: Vec<String>,
    pub path: String,
}

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Identifier(String),
    String(String),
    Number(i64),
    Float(f64),
    OpenBrace,
    CloseBrace,
    OpenParen,
    CloseParen,
    OpenBracket,
    CloseBracket,
    Colon,
    Semicolon,
    Comma,
    Equals,
    Arrow,
    Pipe,
    At,
    Dollar,
    Keyword(String), // object, template, union, main, let, prompt, etc.
    Hash,
    TripleQuoteStart,
    TripleQuoteEnd,
    Whitespace(String),
    Newline,
    TripleQuoteContent(String),
    Comment(String),
    Eof,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Token::Identifier(s) => write!(f, "{}", s),
            Token::String(s) => write!(f, "\"{}\"", s),
            Token::Number(n) => write!(f, "{}", n),
            Token::Float(n) => write!(f, "{}", n),
            Token::OpenBrace => write!(f, "{{"),
            Token::CloseBrace => write!(f, "}}"),
            Token::OpenParen => write!(f, "("),
            Token::CloseParen => write!(f, ")"),
            Token::OpenBracket => write!(f, "["),
            Token::CloseBracket => write!(f, "]"),
            Token::Colon => write!(f, ":"),
            Token::Semicolon => write!(f, ";"),
            Token::Comma => write!(f, ","),
            Token::Equals => write!(f, "="),
            Token::Arrow => write!(f, "->"),
            Token::Pipe => write!(f, "|"),
            Token::At => write!(f, "@"),
            Token::Hash => write!(f, "#"),
            Token::Dollar => write!(f, "$"),
            Token::Keyword(s) => write!(f, "{}", s),
            Token::TripleQuoteStart => write!(f, "\"\"\""),
            Token::TripleQuoteEnd => write!(f, "\"\"\""),
            Token::TripleQuoteContent(s) => write!(f, "{}", s),
            Token::Whitespace(s) => write!(f, "{}", s),
            Token::Comment(s) => write!(f, "#{}", s),
            Token::Newline => write!(f, "\n"),
            Token::Eof => write!(f, "EOF"),
        }
    }
}

#[derive(Debug)]
pub struct Tokenizer {
    input: String,
    position: usize,
    line: usize,
    column: usize,
    last_position: usize,
    last_line: usize,
    last_column: usize,
}

impl Tokenizer {
    pub fn new(input: String) -> Self {
        Tokenizer {
            input,
            position: 0,
            line: 1,
            column: 1,
            last_position: 0,
            last_line: 1,
            last_column: 1,
        }
    }

    pub fn last_location(&self) -> Location {
        Location {
            line: self.last_line,
            column: self.last_column,
        }
    }

    fn save_position(&mut self) {
        self.last_position = self.position;
        self.last_line = self.line;
        self.last_column = self.column;
    }

    fn advance(&mut self) -> Option<char> {
        if self.position >= self.input.len() {
            return None;
        }

        let c = self.input[self.position..].chars().next().unwrap();
        self.position += c.len_utf8();

        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }

        Some(c)
    }

    fn peek(&self) -> Option<char> {
        if self.position >= self.input.len() {
            return None;
        }
        self.input[self.position..].chars().next()
    }

    fn peek_n(&self, n: usize) -> Option<&str> {
        if self.position + n <= self.input.len() {
            Some(&self.input[self.position..self.position + n])
        } else {
            None
        }
    }

    fn read_triple_quoted_string(&mut self) -> Token {
        // Skip the opening """
        self.advance();
        self.advance();
        self.advance();

        let start_pos = self.position;
        let mut found_end = false;

        while let Some(c) = self.peek() {
            if c == '"' && self.peek_n(3) == Some("\"\"\"") {
                found_end = true;
                break;
            }
            self.advance();
        }

        let content = self.input[start_pos..self.position].to_string();

        if found_end {
            // Skip the closing """
            self.advance();
            self.advance();
            self.advance();
            Token::TripleQuoteContent(content)
        } else {
            // Unclosed triple quote - treat what we have as content
            Token::TripleQuoteContent(content)
        }
    }

    fn read_string(&mut self) -> Token {
        // Skip the opening quote
        self.advance();

        let start_pos = self.position;
        let mut found_end = false;

        while let Some(c) = self.peek() {
            if c == '"' {
                found_end = true;
                break;
            } else if c == '\\' {
                // Skip escape character and the escaped character
                self.advance();
                if self.peek().is_some() {
                    self.advance();
                }
            } else {
                self.advance();
            }
        }

        let content = self.input[start_pos..self.position].to_string();

        if found_end {
            // Skip the closing quote
            self.advance();
            Token::String(content)
        } else {
            // Unclosed string - treat what we have as content
            Token::String(content)
        }
    }

    fn read_number(&mut self) -> Token {
        let start_pos = self.position;
        let mut is_float = false;

        // Handle negative sign
        if self.peek() == Some('-') {
            self.advance();
        }

        // Read integer part
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.advance();
            } else if c == '.' {
                is_float = true;
                self.advance();
                break;
            } else {
                break;
            }
        }

        // Read decimal part if this is a float
        if is_float {
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        let number_str = &self.input[start_pos..self.position];

        if is_float {
            match number_str.parse::<f64>() {
                Ok(n) => Token::Float(n),
                Err(_) => {
                    // Handle parsing error - return 0.0 as a fallback
                    Token::Float(0.0)
                }
            }
        } else {
            match number_str.parse::<i64>() {
                Ok(n) => Token::Number(n),
                Err(_) => {
                    // Handle parsing error - return 0 as a fallback
                    Token::Number(0)
                }
            }
        }
    }

    fn read_identifier_or_keyword(&mut self) -> Token {
        let start_pos = self.position;

        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                self.advance();
            } else {
                break;
            }
        }

        let text = self.input[start_pos..self.position].to_string();
        let keywords: Vec<String> = vec![
            "object",
            "template",
            "union",
            "main",
            "let",
            "prompt",
            "import",
            "from",
            "template_args",
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect();

        if keywords.contains(&text) {
            Token::Keyword(text)
        } else {
            Token::Identifier(text)
        }
    }

    fn read_comment(&mut self) -> Token {
        // Skip the #
        self.advance();

        let start_pos = self.position;

        while let Some(c) = self.peek() {
            if c == '\n' {
                break;
            }
            self.advance();
        }

        Token::Comment(self.input[start_pos..self.position].to_string())
    }

    fn read_arrow(&mut self) -> Token {
        // Skip the -
        self.advance();

        // Skip the >
        if self.peek() == Some('>') {
            self.advance();
            Token::Arrow
        } else {
            // Not an arrow, just a minus sign (which we'll handle as part of a number)
            // Go back to the minus position
            self.position -= 1;
            self.column -= 1;
            self.read_number()
        }
    }
    pub fn next_token(&mut self) -> Token {
        self.save_position();
        if self.position >= self.input.len() {
            return Token::Eof;
        }
        // Check for whitespace
        let c = self.peek().unwrap();

        if c.is_whitespace() {
            let start_pos = self.position;

            // Consume whitespace
            while let Some(c) = self.peek() {
                if c.is_whitespace() {
                    self.advance();
                } else {
                    break;
                }
            }

            // Return whitespace as a token
            return Token::Whitespace(self.input[start_pos..self.position].to_string());
        }

        // Check for triple-quoted string
        if self.peek_n(3) == Some("\"\"\"") {
            return self.read_triple_quoted_string();
        }

        match self.peek().unwrap() {
            '{' => {
                self.advance();
                Token::OpenBrace
            }
            '}' => {
                self.advance();
                Token::CloseBrace
            }
            '(' => {
                self.advance();
                Token::OpenParen
            }
            ')' => {
                self.advance();
                Token::CloseParen
            }
            '[' => {
                self.advance();
                Token::OpenBracket
            }
            ']' => {
                self.advance();
                Token::CloseBracket
            }
            ':' => {
                self.advance();
                Token::Colon
            }
            ';' => {
                self.advance();
                Token::Semicolon
            }
            ',' => {
                self.advance();
                Token::Comma
            }
            '=' => {
                self.advance();
                Token::Equals
            }
            '|' => {
                self.advance();
                Token::Pipe
            }
            '@' => {
                self.advance();
                Token::At
            }
            '#' => self.read_comment(),
            '$' => {
                self.advance();
                Token::Dollar
            }
            '"' => self.read_string(),
            '-' => self.read_arrow(),
            c if c.is_ascii_digit() => self.read_number(),
            c if c.is_alphabetic() || c == '_' => self.read_identifier_or_keyword(),
            _ => {
                // Skip unrecognized character
                self.advance();
                self.next_token()
            }
        }
    }
}

#[derive(Debug)]
pub struct Parser {
    tokenizer: Tokenizer,
    current_token: Token,
    peek_token: Token,
    registry: Arc<RefCell<HashMap<String, WAILField>>>,
    template_registry: Arc<RefCell<HashMap<String, WAILTemplateDef>>>,
    adhoc_obj_ref_id_counter: Arc<RefCell<i64>>,
    adhoc_obj_ids: Arc<RefCell<Vec<String>>>,
    adhoc_obj_refs: Arc<RefCell<HashMap<String, WAILObject>>>,
    object_instances: Arc<RefCell<HashMap<String, WAILObjectInstantiation>>>,
    import_chain: Arc<RefCell<ImportChain>>,
    in_prompt_block: RefCell<bool>,
    current_module: Arc<RefCell<Vec<String>>>,
}

impl Parser {
    pub fn new(
        input: String,
        registry: Arc<RefCell<HashMap<String, WAILField>>>,
        template_registry: Arc<RefCell<HashMap<String, WAILTemplateDef>>>,
        adhoc_obj_ref_id_counter: Arc<RefCell<i64>>,
        adhoc_obj_ids: Arc<RefCell<Vec<String>>>,
        adhoc_obj_refs: Arc<RefCell<HashMap<String, WAILObject>>>,
        object_instances: Arc<RefCell<HashMap<String, WAILObjectInstantiation>>>,
        import_chain: Arc<RefCell<ImportChain>>,
        current_module: Arc<RefCell<Vec<String>>>,
    ) -> Self {
        let mut tokenizer = Tokenizer::new(input);
        let current_token = tokenizer.next_token();
        let peek_token = tokenizer.next_token();

        Parser {
            tokenizer,
            current_token,
            peek_token,
            registry,
            template_registry,
            adhoc_obj_ref_id_counter,
            adhoc_obj_ids,
            adhoc_obj_refs,
            object_instances,
            import_chain,
            in_prompt_block: RefCell::new(false),
            current_module: current_module,
        }
    }

    // Update parse_object to better handle fields
    fn parse_object(&mut self) -> Result<WAILDefinition, WAILParseError> {
        // Expect "object" keyword
        self.expect_keyword("object")?;
        // Parse object name
        let name = self.expect_identifier()?;

        let namespaced_id = self.namespaced_identifier(name.to_string());

        {
            if self.registry.borrow().contains_key(&namespaced_id) {
                return Ok(WAILDefinition::Object(
                    self.lookup_symbol_in_registry(&name)?.clone(),
                ));
            }
        }

        // Expect opening brace
        self.expect(Token::OpenBrace)?;

        // Parse fields
        let mut fields = Vec::new();

        while !matches!(self.current_token, Token::CloseBrace) {
            if matches!(self.current_token, Token::Identifier(_)) {
                let field = self.parse_field()?;
                fields.push(field);
            } else if matches!(self.current_token, Token::Eof) {
                return Err(WAILParseError::UnexpectedEOF {
                    expected: "field or closing brace".to_string(),
                    location: self.tokenizer.last_location(),
                });
            } else {
                // Skip any non-identifier tokens (this helps with unexpected whitespace/comments)
                self.next_token();
            }
        }

        // Expect closing brace
        self.expect(Token::CloseBrace)?;

        // Parse annotations
        let annotations = self.parse_annotations()?;

        // Convert fields into HashMap
        let mut field_map = HashMap::new();
        for field in &fields {
            field_map.insert(
                WAILString {
                    value: field.name.clone(),
                    type_data: WAILTypeData {
                        json_type: JsonValue::String(field.name.clone()),
                        type_name: "String".to_string(),
                        field_definitions: None,
                        element_type: None,
                    },
                },
                field.field_type.clone(),
            );
        }

        // Add _type field
        field_map.insert(
            WAILString {
                value: "_type".to_string(),
                type_data: WAILTypeData {
                    json_type: JsonValue::String("_type".to_string()),
                    type_name: "String".to_string(),
                    field_definitions: None,
                    element_type: None,
                },
            },
            WAILType::Simple(WAILSimpleType::String(WAILString {
                value: name.to_string(),
                type_data: WAILTypeData {
                    json_type: JsonValue::String("_type".to_string()),
                    type_name: "String".to_string(),
                    field_definitions: None,
                    element_type: None,
                },
            })),
        );

        // Add _type field to fields list
        fields.push(WAILField {
            name: "_type".to_string(),
            field_type: WAILType::Simple(WAILSimpleType::String(WAILString {
                value: name.to_string(),
                type_data: WAILTypeData {
                    json_type: JsonValue::String("_type".to_string()),
                    type_name: "String".to_string(),
                    field_definitions: None,
                    element_type: None,
                },
            })),
            annotations: vec![],
        });

        // Create object
        let object = WAILObject {
            value: field_map,
            type_data: WAILTypeData {
                json_type: JsonValue::Object(HashMap::new()), // Placeholder empty object
                type_name: name.to_string(),
                field_definitions: Some(fields.clone()),
                element_type: None,
            },
        };

        let field = WAILField {
            name: name.to_string(),
            field_type: WAILType::Composite(WAILCompositeType::Object(object)),
            annotations,
        };

        let definition = WAILDefinition::Object(field.clone());

        let namespaced_id = self.namespaced_identifier(name.to_string());

        {
            // Add object to registry
            self.registry.borrow_mut().insert(namespaced_id, field);
        }

        Ok(definition)
    }

    // Update the parse_template method to handle triple-quoted strings better
    fn parse_template(&mut self) -> Result<WAILDefinition, WAILParseError> {
        // Expect "template" keyword
        self.expect_keyword("template")?;

        // Parse template name
        let name = self.expect_identifier()?;

        let namespaced_id = self.namespaced_identifier(name.to_string());

        {
            // Check for duplicate definition
            if self.template_registry.borrow().contains_key(&namespaced_id) {
                return Ok(WAILDefinition::Template(
                    self.lookup_template_in_registry(&name)?.clone(),
                ));
            }
        }

        // Expect opening parenthesis
        self.expect(Token::OpenParen)?;

        // Parse parameters
        let mut parameters = Vec::new();

        if !matches!(self.current_token, Token::CloseParen) {
            loop {
                // Parse parameter
                let param = self.parse_parameter()?;
                parameters.push(param);

                // Check for more parameters
                if matches!(self.current_token, Token::Comma) {
                    self.next_token();
                } else {
                    break;
                }
            }
        }

        // Expect closing parenthesis
        self.expect(Token::CloseParen)?;

        // Expect arrow token
        self.expect(Token::Arrow)?;

        // Parse return type
        let return_type = self.parse_type()?;

        // Parse annotations
        let annotations = self.parse_annotations()?;

        // Expect opening brace
        self.expect(Token::OpenBrace)?;

        // Parse prompt keyword and colon
        if let Token::Keyword(ref s) = self.current_token {
            if s == "prompt" {
                self.next_token();
            }
        } else {
            return Err(WAILParseError::UnexpectedToken {
                found: format!("{}", self.current_token),
                location: self.tokenizer.last_location(),
            });
        }

        self.expect(Token::Colon)?;

        // Parse prompt template (triple quoted string)
        match self.current_token.clone() {
            Token::TripleQuoteContent(content) => {
                let prompt_template = {
                    let content_str = content.to_string();
                    self.next_token(); // Consume the content
                                       // Adjust indentation for multi-line prompt
                    adjust_indentation(&content_str, 0)
                };

                // Expect closing brace
                self.expect(Token::CloseBrace)?;

                // Create output field
                let output_field = WAILField {
                    name: match &return_type {
                        WAILType::Composite(WAILCompositeType::Object(obj)) => {
                            obj.type_data.type_name.to_string()
                        }
                        _ => "output".to_string(), // Default name if not an object type
                    },
                    field_type: return_type,
                    annotations: Vec::new(),
                };

                let template_def = WAILTemplateDef {
                    name: name.to_string(),
                    inputs: parameters,
                    output: output_field,
                    prompt_template,
                    annotations,
                };

                let namespaced_id = self.namespaced_identifier(name.to_string());

                // Add to template registry
                self.template_registry
                    .borrow_mut()
                    .insert(namespaced_id, template_def.clone());

                Ok(WAILDefinition::Template(template_def))
            }
            _ => Err(WAILParseError::UnexpectedToken {
                found: format!("{}", self.current_token),
                location: self.tokenizer.last_location(),
            }),
        }
    }

    fn parse_wail_file(
        &mut self,
        file_type: WAILFileType,
    ) -> Result<Vec<WAILDefinition>, WAILParseError> {
        let mut definitions = Vec::new();
        let mut imports = Vec::new();

        self.optional_whitespace();

        while let Token::Keyword(ref s) = &self.current_token {
            if s != "import" {
                break;
            }
            let import = self.parse_import()?;
            imports.push(import.clone());
            definitions.push(import);
        }

        // Process imports after they're all parsed
        if !imports.is_empty() {
            self.resolve_imports(&imports)?;
        }

        // Parse regular definitions
        while self.current_token != Token::Eof {
            match self.current_token.clone() {
                Token::Keyword(kw) if kw == "object".to_string() => {
                    let object = self.parse_object()?;
                    definitions.push(object);
                }
                Token::Keyword(kw) if kw == "template".to_string() => {
                    let template = self.parse_template()?;
                    definitions.push(template);
                }
                Token::Keyword(kw) if kw == "union".to_string() => {
                    let union = self.parse_union()?;
                    definitions.push(union);
                }
                Token::Eof => break,
                _ => {
                    // Skip any non-keyword tokens
                    self.next_token();
                }
            }
        }

        Ok(definitions)
    }

    fn resolve_imports(&mut self, imports: &[WAILDefinition]) -> Result<(), WAILParseError> {
        for def in imports {
            if let WAILDefinition::Import(import) = def {
                // Resolve the import path
                let file_path = self.import_chain.borrow().resolve_path(&import.path)?;
                self.current_module.borrow_mut().push(import.path.clone());

                // Check for circular imports
                if let Err(e) = self.import_chain.borrow_mut().push(&file_path) {
                    return Err(e);
                }
                // Read the file content
                let lib_content =
                    std::fs::read_to_string(&file_path).map_err(|e| WAILParseError::FileError {
                        path: import.path.clone(),
                        error: e.to_string(),
                    })?;

                // Create a new parser for this import with the same shared state
                let mut import_parser = Parser::new(
                    lib_content.clone(),
                    self.registry.clone(),
                    self.template_registry.clone(),
                    self.adhoc_obj_ref_id_counter.clone(),
                    self.adhoc_obj_ids.clone(),
                    self.adhoc_obj_refs.clone(),
                    self.object_instances.clone(),
                    self.import_chain.clone(),
                    self.current_module.clone(),
                );

                // Parse the library file
                let lib_defs = import_parser.parse_wail_file(WAILFileType::Library)?;

                // Create a map of definitions from the library
                let mut objects = HashMap::new();
                for lib_def in &lib_defs {
                    if let Some(name) = lib_def.get_name() {
                        objects.insert(name.to_string(), lib_def.clone());
                    }
                }

                // Process each requested item
                for item_name in &import.items {
                    let mut found = false;
                    if let Some(lib_def) = objects.get(item_name) {
                        found = true;

                        // Add the definition to the appropriate registry
                        match lib_def {
                            WAILDefinition::Object(field) => {
                                let namespaced_id = self.namespaced_identifier(field.name.clone());

                                let mut reg_borrow = self.registry.borrow_mut();

                                match reg_borrow.get(&namespaced_id) {
                                    Some(_) => continue,
                                    None => {
                                        reg_borrow.insert(namespaced_id, field.clone());
                                        drop(reg_borrow)
                                    }
                                }
                            }
                            WAILDefinition::Template(template) => {
                                let namespaced_id =
                                    self.namespaced_identifier(template.name.clone());
                                let mut reg_borrow = self.template_registry.borrow_mut();

                                match reg_borrow.get(&namespaced_id) {
                                    Some(_) => continue,
                                    None => {
                                        reg_borrow.insert(namespaced_id, template.clone());
                                        drop(reg_borrow)
                                    }
                                }
                            }
                            WAILDefinition::Union(field) => {
                                let namespaced_id = self.namespaced_identifier(field.name.clone());
                                let mut reg_borrow = self.registry.borrow_mut();

                                match reg_borrow.get(&namespaced_id) {
                                    Some(_) => continue,
                                    None => {
                                        reg_borrow.insert(namespaced_id, field.clone());
                                        drop(reg_borrow)
                                    }
                                }
                            }
                            _ => {}
                        }

                        // Also process referenced types
                        match lib_def {
                            WAILDefinition::Object(field) => {
                                self.add_referenced_types(&field.field_type, &objects);
                            }
                            WAILDefinition::Template(template) => {
                                for param in &template.inputs {
                                    self.add_referenced_types(&param.field_type, &objects);
                                }
                                self.add_referenced_types(&template.output.field_type, &objects);
                            }
                            WAILDefinition::Union(field) => {
                                self.add_referenced_types(&field.field_type, &objects);
                            }
                            _ => {}
                        }
                    }

                    if !found {
                        return Err(WAILParseError::ImportNotFound {
                            name: item_name.clone(),
                            path: import.path.clone(),
                        });
                    }
                }

                // Pop from import chain after processing
                self.import_chain.borrow_mut().pop();
                self.current_module.borrow_mut().pop();
            }
        }

        Ok(())
    }

    // Helper method to add referenced types
    fn add_referenced_types(
        &self,
        field_type: &WAILType,
        objects: &HashMap<String, WAILDefinition>,
    ) {
        match field_type {
            WAILType::Composite(composite) => match composite {
                WAILCompositeType::Object(obj) => {
                    let type_name = obj.type_data.type_name.clone();

                    {
                        // If this type exists in objects map and not already in registry
                        if objects.contains_key(&type_name)
                            && !self.registry.borrow().contains_key(&type_name)
                        {
                            if let Some(WAILDefinition::Object(field)) = objects.get(&type_name) {
                                {
                                    self.registry
                                        .borrow_mut()
                                        .insert(field.name.clone(), field.clone());
                                }

                                // Recursively add fields
                                if let Some(fields) = &obj.type_data.field_definitions {
                                    for field in fields {
                                        self.add_referenced_types(&field.field_type, objects);
                                    }
                                }
                            }
                        }
                    }
                }
                WAILCompositeType::Array(array) => {
                    if let Some(element_type) = &array.type_data.element_type {
                        self.add_referenced_types(element_type, objects);
                    }
                }
                WAILCompositeType::Union(union) => {
                    for member in &union.members {
                        self.add_referenced_types(&member.field_type, objects);
                    }
                }
            },
            _ => {} // Simple types don't reference other objects
        }
    }

    // Update other parsing methods that work with newlines and comments
    fn parse_field(&mut self) -> Result<WAILField, WAILParseError> {
        // Parse field name
        let name = self.expect_identifier()?;

        // Expect colon
        self.expect(Token::Colon)?;

        // Parse field type
        let field_type = self.parse_type()?;

        // Parse annotations
        let annotations = self.parse_annotations()?;

        Ok(WAILField {
            name: name.to_string(),
            field_type,
            annotations,
        })
    }

    fn lookup_template_in_registry(&self, name: &str) -> Result<WAILTemplateDef, WAILParseError> {
        let mut matches = vec![];

        {
            for (key, def) in self.template_registry.borrow().iter() {
                if let Some(actual_name) = key.split('.').last() {
                    if actual_name == name {
                        matches.push((key.clone(), def.clone()));
                    }
                }
            }
        }

        match matches.len() {
            0 => Err(WAILParseError::SymbolNotFound {
                name: name.to_string(),
            }),
            1 => Ok(matches.remove(0).1),
            _ => Err(WAILParseError::AmbiguousSymbol {
                name: name.to_string(),
                matches: matches.into_iter().map(|(k, _)| k).collect(),
            }),
        }
    }

    fn lookup_symbol_in_registry(&self, name: &str) -> Result<WAILField, WAILParseError> {
        let mut matches = vec![];

        {
            for (key, def) in self.registry.borrow().iter() {
                if let Some(actual_name) = key.split('.').last() {
                    if actual_name == name {
                        matches.push((key.clone(), def.clone()));
                    }
                }
            }
        }

        match matches.len() {
            0 => Err(WAILParseError::SymbolNotFound {
                name: name.to_string(),
            }),
            1 => Ok(matches.remove(0).1.clone()),
            _ => Err(WAILParseError::AmbiguousSymbol {
                name: name.to_string(),
                matches: matches.into_iter().map(|(k, _)| k).collect(),
            }),
        }
    }

    fn parse_union(&mut self) -> Result<WAILDefinition, WAILParseError> {
        // Expect "union" keyword
        self.expect_keyword("union")?;

        // Parse union name
        let name = self.expect_identifier()?;

        let namespaced_id = self.namespaced_identifier(name.to_string());

        {
            if self.registry.borrow().contains_key(&namespaced_id) {
                return Ok(WAILDefinition::Object(
                    self.lookup_symbol_in_registry(&name)?.clone(),
                ));
            }
        }

        // Expect equals sign
        self.expect(Token::Equals)?;

        // Parse first type
        let first_type_name = self.expect_identifier()?;

        // Check for array syntax
        let first_is_array = if matches!(self.current_token, Token::OpenBracket) {
            self.next_token();
            self.expect(Token::CloseBracket)?;
            true
        } else {
            false
        };

        // Create first type
        let first_type = self.create_type_value(first_type_name, first_is_array)?;

        // Parse additional union members
        let mut member_types = vec![first_type];

        while matches!(self.current_token, Token::Pipe) {
            self.next_token();

            // Parse member type
            let member_name = self.expect_identifier()?;

            // Check for array syntax
            let member_is_array = if matches!(self.current_token, Token::OpenBracket) {
                self.next_token();
                self.expect(Token::CloseBracket)?;
                true
            } else {
                false
            };

            // Create member type
            let member_type = self.create_type_value(member_name, member_is_array)?;
            member_types.push(member_type);
        }

        // Expect semicolon
        self.expect(Token::Semicolon)?;

        // Create union members from types
        let mut members = Vec::new();
        for (i, type_val) in member_types.into_iter().enumerate() {
            members.push(WAILField {
                name: format!("member_{}", i),
                field_type: type_val,
                annotations: Vec::new(),
            });
        }

        // Create union type
        let union = WAILUnion {
            members,
            type_data: WAILTypeData {
                json_type: JsonValue::Object(HashMap::new()),
                type_name: name.to_string(),
                field_definitions: None,
                element_type: None,
            },
        };

        let field = WAILField {
            name: name.to_string(),
            field_type: WAILType::Composite(WAILCompositeType::Union(union)),
            annotations: Vec::new(),
        };

        let namespaced_id = self.namespaced_identifier(name.to_string());

        // Add to registry
        self.registry
            .borrow_mut()
            .insert(namespaced_id, field.clone());

        Ok(WAILDefinition::Union(field))
    }

    fn parse_prompt_block(&mut self) -> Result<String, WAILParseError> {
        self.in_prompt_block.replace(true);
        // Expect opening brace
        self.expect(Token::OpenBrace)?;

        // Collect all tokens until the closing brace, preserving whitespace
        let mut prompt_content = String::new();
        let mut brace_count = 1;

        while brace_count > 0 && self.current_token != Token::Eof {
            match &self.current_token {
                Token::OpenBrace => {
                    brace_count += 1;
                    prompt_content.push('{');
                    self.next_token();
                }
                Token::CloseBrace => {
                    brace_count -= 1;
                    if brace_count > 0 {
                        prompt_content.push('}');
                    }
                    self.next_token();
                }
                Token::Whitespace(s) => {
                    prompt_content.push_str(s);
                    self.next_token();
                }
                Token::Newline => {
                    prompt_content.push('\n');
                    self.next_token();
                }
                Token::Identifier(s) => {
                    prompt_content.push_str(s);
                    self.next_token();
                }
                Token::String(s) => {
                    prompt_content.push_str(&format!("\"{}\"", s));
                    self.next_token();
                }
                Token::Number(n) => {
                    prompt_content.push_str(&n.to_string());
                    self.next_token();
                }
                Token::Eof => {
                    return Err(WAILParseError::UnexpectedEOF {
                        expected: "Closing brace for prompt block".to_string(),
                        location: self.tokenizer.last_location(),
                    });
                }
                _ => {
                    // Add token content to prompt
                    prompt_content.push_str(&format!("{}", self.current_token));
                    self.next_token();
                }
            }
        }

        self.in_prompt_block.replace(false);

        Ok(prompt_content)
    }

    //-------------------------------###########

    fn next_token(&mut self) -> Token {
        let result = self.current_token.clone();
        self.current_token = self.peek_token.clone();

        // Get the next token
        let mut next_token = self.tokenizer.next_token();

        // Skip whitespace and comments by default
        let in_prompt = self.in_prompt_block.borrow();

        while !*in_prompt
            && (matches!(next_token, Token::Whitespace(_))
                || matches!(next_token, Token::Comment(_)))
        {
            next_token = self.tokenizer.next_token();
        }

        self.peek_token = next_token;

        result
    }

    fn expect(&mut self, expected: Token) -> Result<(), WAILParseError> {
        if std::mem::discriminant(&self.current_token) == std::mem::discriminant(&expected) {
            self.next_token();
            Ok(())
        } else {
            Err(WAILParseError::UnexpectedToken {
                found: format!("{}", self.current_token),
                location: self.tokenizer.last_location(),
            })
        }
    }

    fn expect_identifier(&mut self) -> Result<String, WAILParseError> {
        self.optional_whitespace();

        match self.current_token.clone() {
            Token::Identifier(name) => {
                let result = name;
                self.next_token();
                Ok(result)
            }
            Token::Keyword(name) => Err(WAILParseError::UnexpectedKeyword {
                found: format!("{}", name),
                location: self.tokenizer.last_location(),
            }),
            _ => Err(WAILParseError::UnexpectedToken {
                found: format!("{}", self.current_token),
                location: self.tokenizer.last_location(),
            }),
        }
    }

    fn expect_keyword(&mut self, keyword: &str) -> Result<(), WAILParseError> {
        match self.current_token.clone() {
            Token::Keyword(k) if k == keyword => {
                self.next_token();
                Ok(())
            }
            _ => Err(WAILParseError::UnexpectedToken {
                found: format!("{}", self.current_token),
                location: self.tokenizer.last_location(),
            }),
        }
    }

    fn expect_string(&mut self) -> Result<String, WAILParseError> {
        match self.current_token.clone() {
            Token::String(s) => {
                let result = s;
                self.next_token();
                Ok(result)
            }
            _ => Err(WAILParseError::UnexpectedToken {
                found: format!("{}", self.current_token),
                location: self.tokenizer.last_location(),
            }),
        }
    }

    fn parse_import(&mut self) -> Result<WAILDefinition, WAILParseError> {
        self.optional_whitespace();
        self.expect_keyword("import")?;

        self.optional_whitespace();
        self.expect(Token::OpenBrace)?;

        // Parse imported item names
        let mut items = Vec::new();

        while self.current_token != Token::CloseBrace {
            match self.current_token.clone() {
                Token::Identifier(name) => {
                    items.push(name.to_string());
                    self.next_token();

                    // Expect comma or closing brace
                    if self.current_token == Token::Comma {
                        self.next_token();
                    } else if self.current_token != Token::CloseBrace {
                        return Err(WAILParseError::UnexpectedToken {
                            found: format!("{}", self.current_token),
                            location: self.tokenizer.last_location(),
                        });
                    }
                }
                _ => {
                    return Err(WAILParseError::UnexpectedToken {
                        found: format!("{}", self.current_token),
                        location: self.tokenizer.last_location(),
                    });
                }
            }
        }

        self.expect(Token::CloseBrace)?;

        self.expect_keyword("from")?;

        // Parse path string
        let path = self.expect_string()?;

        Ok(WAILDefinition::Import(WAILImport {
            items: items.iter().map(|s| s.to_string()).collect(),
            path: path.to_string(),
        }))
    }

    fn parse_type(&mut self) -> Result<WAILType, WAILParseError> {
        // Check for adhoc object type (opening brace without a type name)
        if matches!(self.current_token, Token::OpenBrace) {
            return self.parse_adhoc_object_type();
        }

        // Parse base type name
        let base_type = self.expect_identifier()?;

        // Check for array suffix
        let is_array = if matches!(self.current_token, Token::OpenBracket) {
            self.next_token();
            self.expect(Token::CloseBracket)?;
            true
        } else {
            false
        };

        // Check for union type with pipe operator
        let mut union_members = Vec::new();

        if matches!(self.current_token, Token::Pipe) {
            // Create base type value for first union member
            let base_type_val = self.create_type_value(base_type, is_array)?;
            union_members.push(base_type_val);

            // Parse additional union members
            while matches!(self.current_token, Token::Pipe) {
                self.next_token();

                // Parse member type name
                let member_type = self.expect_identifier()?;

                // Check for array suffix for this member
                let member_is_array = if matches!(self.current_token, Token::OpenBracket) {
                    self.next_token();
                    self.expect(Token::CloseBracket)?;
                    true
                } else {
                    false
                };

                // Create type value for this member
                let member_type_val = self.create_type_value(member_type, member_is_array)?;
                union_members.push(member_type_val);
            }

            // Create union type
            let mut wail_fields = Vec::new();

            for (i, type_data) in union_members.iter().enumerate() {
                let field = WAILField {
                    name: format!("member_{}", i),
                    field_type: type_data.clone(),
                    annotations: Vec::new(),
                };

                wail_fields.push(field);
            }

            return Ok(WAILType::Composite(WAILCompositeType::Union(WAILUnion {
                members: wail_fields,
                type_data: WAILTypeData {
                    json_type: JsonValue::Object(HashMap::new()),
                    type_name: "Union".to_string(), // Default name
                    field_definitions: None,
                    element_type: None,
                },
            })));
        }

        // If not a union, create the base type (possibly wrapped in array)
        self.create_type_value(base_type, is_array)
    }

    fn namespaced_identifier(&self, id: String) -> String {
        let module_prefix = self.current_module.borrow().last().cloned();
        match module_prefix {
            Some(prefix) => format!("{}.{}", prefix, id),
            None => id.to_string(), // top-level file, no prefix
        }
    }

    fn parse_adhoc_object_type(&mut self) -> Result<WAILType, WAILParseError> {
        // Generate a unique ID for the adhoc object
        let adhoc_id = {
            let mut counter = self.adhoc_obj_ref_id_counter.borrow_mut();
            *counter += 1;
            format!("adhoc_{}", *counter)
        };

        // Expect opening brace
        self.expect(Token::OpenBrace)?;

        // Parse fields
        let mut fields = Vec::new();

        while !matches!(self.current_token, Token::CloseBrace) {
            if matches!(self.current_token, Token::Identifier(_)) {
                let field = self.parse_field()?;
                fields.push(field);
            } else if matches!(self.current_token, Token::Newline)
                || matches!(self.current_token, Token::Whitespace(_))
            {
                self.next_token(); // Skip newlines and whitespace between fields
            } else if matches!(self.current_token, Token::Hash) {
                self.parse_comment()?; // Skip comments between fields
            } else {
                return Err(WAILParseError::UnexpectedToken {
                    found: format!("{}", self.current_token),
                    location: self.tokenizer.last_location(),
                });
            }
        }

        // Expect closing brace
        self.expect(Token::CloseBrace)?;

        // Create field map
        let mut field_map = HashMap::new();
        for field in &fields {
            field_map.insert(
                WAILString {
                    value: field.name.clone(),
                    type_data: WAILTypeData {
                        json_type: JsonValue::String(field.name.clone()),
                        type_name: "String".to_string(),
                        field_definitions: None,
                        element_type: None,
                    },
                },
                field.field_type.clone(),
            );
        }

        self.adhoc_obj_ids.borrow_mut().push(adhoc_id.clone());

        // Create the adhoc object
        let adhoc_type_name = adhoc_id.clone();
        let object = WAILObject {
            value: field_map,
            type_data: WAILTypeData {
                json_type: JsonValue::Object(HashMap::new()),
                type_name: adhoc_type_name,
                field_definitions: Some(fields.clone()),
                element_type: None,
            },
        };

        let namespaced_id = self.namespaced_identifier(adhoc_id.clone());

        self.adhoc_obj_refs
            .borrow_mut()
            .insert(namespaced_id.clone(), object.clone());

        let field = WAILField {
            name: adhoc_id.clone(),
            field_type: WAILType::Composite(WAILCompositeType::Object(object.clone())),
            annotations: vec![],
        };

        self.registry
            .borrow_mut()
            .insert(namespaced_id.clone(), field.clone());

        Ok(WAILType::Composite(WAILCompositeType::Object(object)))
    }

    fn create_type_value(
        &self,
        type_name: String,
        is_array: bool,
    ) -> Result<WAILType, WAILParseError> {
        // Create inner type based on name
        let inner_type = match type_name.clone() {
            tn if tn == "String".to_string() => {
                WAILType::Simple(WAILSimpleType::String(WAILString {
                    value: String::new(),
                    type_data: WAILTypeData {
                        json_type: JsonValue::String(String::new()),
                        type_name: type_name.to_string(),
                        field_definitions: None,
                        element_type: None,
                    },
                }))
            }
            tn if tn == "Number".to_string() => {
                WAILType::Simple(WAILSimpleType::Number(WAILNumber::Integer(WAILInteger {
                    value: 0,
                    type_data: WAILTypeData {
                        json_type: JsonValue::Number(Number::Integer(0)),
                        type_name: type_name.to_string(),
                        field_definitions: None,
                        element_type: None,
                    },
                })))
            }
            tn if tn == "Boolean".to_string() => {
                WAILType::Simple(WAILSimpleType::Boolean(WAILBoolean {
                    value: "false".to_string(),
                    type_data: WAILTypeData {
                        json_type: JsonValue::Boolean(false),
                        type_name: type_name.to_string(),
                        field_definitions: None,
                        element_type: None,
                    },
                }))
            }
            // For other types, check if it's registered or assume it's an object/custom type
            _ => match self.lookup_symbol_in_registry(&type_name) {
                Ok(field) => field.field_type.clone(),
                Err(e) => match e {
                    WAILParseError::AmbiguousSymbol { .. } => return Err(e),
                    _ => WAILType::Composite(WAILCompositeType::Object(WAILObject {
                        value: HashMap::new(),
                        type_data: WAILTypeData {
                            json_type: JsonValue::Object(HashMap::new()),
                            type_name: type_name.to_string(),
                            field_definitions: None,
                            element_type: None,
                        },
                    })),
                },
            },
        };

        // Wrap in array if needed
        if is_array {
            Ok(WAILType::Composite(WAILCompositeType::Array(WAILArray {
                values: Vec::new(),
                type_data: WAILTypeData {
                    json_type: JsonValue::Array(Vec::new()),
                    type_name: "Array".to_string(),
                    field_definitions: None,
                    element_type: Some(Box::new(inner_type)),
                },
            })))
        } else {
            Ok(inner_type)
        }
    }

    fn parse_annotations(&mut self) -> Result<Vec<WAILAnnotation>, WAILParseError> {
        let mut annotations = Vec::new();

        while matches!(self.current_token, Token::At) {
            self.next_token();

            // Parse annotation name
            match &self.current_token {
                Token::Identifier(s) if s == "description" => {
                    self.next_token();

                    self.expect(Token::OpenParen)?;
                    let desc = self.expect_string()?;
                    self.expect(Token::CloseParen)?;

                    annotations.push(WAILAnnotation::Description(desc.to_string()));
                }
                _ => {
                    return Err(WAILParseError::UnexpectedToken {
                        found: format!("{}", self.current_token),
                        location: self.tokenizer.last_location(),
                    });
                }
            }
        }

        Ok(annotations)
    }

    fn parse_comment(&mut self) -> Result<String, WAILParseError> {
        match self.current_token.clone() {
            Token::Hash => {
                self.next_token(); // Skip the # token

                match self.current_token.clone() {
                    Token::Comment(comment) => {
                        let result = comment.to_string();
                        self.next_token();
                        Ok(result)
                    }
                    _ => Ok(String::new()), // Empty comment
                }
            }
            _ => Err(WAILParseError::UnexpectedToken {
                found: format!("{}", self.current_token),
                location: self.tokenizer.last_location(),
            }),
        }
    }

    fn parse_parameter(&mut self) -> Result<WAILField, WAILParseError> {
        // Parse parameter name
        let name = self.expect_identifier()?;

        // Expect colon
        self.expect(Token::Colon)?;

        // Parse parameter type
        let param_type = self.parse_type()?;

        Ok(WAILField {
            name: name.to_string(),
            field_type: param_type,
            annotations: Vec::new(),
        })
    }

    fn optional_whitespace(&mut self) {
        // Consume any whitespace tokens if they exist
        while matches!(self.current_token, Token::Whitespace(_))
            || matches!(self.current_token, Token::Newline)
        {
            self.next_token();
        }
    }
}

impl WAILParser {
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            registry: Arc::new(RefCell::new(HashMap::new())),
            template_registry: Arc::new(RefCell::new(HashMap::new())),
            adhoc_obj_ref_id_counter: Arc::new(RefCell::new(0)),
            adhoc_obj_ids: Arc::new(RefCell::new(Vec::new())),
            adhoc_obj_refs: Arc::new(RefCell::new(HashMap::new())),
            object_instances: Arc::new(RefCell::new(HashMap::new())),
            import_chain: Arc::new(RefCell::new(ImportChain::new(base_path.clone()))),
            base_path: base_path.clone(),
            incremental_parser: Arc::new(RefCell::new(None)),
            current_module: Arc::new(RefCell::new(vec![])),
        }
    }

    pub fn parse_output(&self, template_name: &str, llm_output: &str) -> Result<JsonValue, String> {
        let tpl = self
            .template_registry
            .borrow()
            .get(template_name)
            .ok_or_else(|| format!("template {template_name} not found"))?;

        // 1. Stream-parse only what’s inside the expected tag
        let tag = tpl.output.field_type.tag();
        let mut sp = StreamParser::new(vec![tag.clone()]);

        let bytes = llm_output.as_bytes().to_vec();
        let mut i = 0;
        let mut payload = None;
        while i < bytes.len() {
            // Feed whatever UTF-8 slice is valid
            match std::str::from_utf8(&bytes[i..]) {
                Ok(chunk) => {
                    payload = sp.step(chunk).unwrap();
                    break;
                }
                Err(e) => {
                    let cut = e.valid_up_to();
                    if cut == 0 {
                        break;
                    }
                    let chunk = std::str::from_utf8(&bytes[i..i + cut]).unwrap();
                    payload = sp.step(chunk).unwrap();
                    i += cut;
                }
            }
        }
        let mut value = payload.ok_or_else(|| "no payload inside tag".to_owned())?;

        // 2. validate, try to auto-fix once if needed
        match tpl.output.field_type.validate_json(&value) {
            Ok(()) => return Ok(value),
            Err(err) => {
                self.fix_json_value(&mut value, &self.get_error_location(&err))
                    .map_err(|e| format!("auto-fix failed: {e}"))?;
                tpl.output
                    .field_type
                    .validate_json(&value)
                    .map_err(|e| format!("{e:?}"))?;
                Ok(value)
            }
        }
    }

    pub fn get_error_location(&self, error: &JsonValidationError) -> Vec<PathSegment> {
        match error {
            JsonValidationError::ObjectMissingMetaType => {
                vec![PathSegment::MissingMetaType]
            }
            JsonValidationError::ObjectNestedTypeValidation((field, nested_error)) => {
                let mut path = vec![PathSegment::Field(field.clone())];
                path.extend(self.get_error_location(nested_error));
                path
            }
            JsonValidationError::ArrayElementTypeError((index, nested_error)) => {
                let mut path = vec![PathSegment::ArrayIndex(*index)];
                path.extend(self.get_error_location(nested_error));
                path
            }
            JsonValidationError::NotMemberOfUnion((field, validation_errors)) => {
                vec![PathSegment::UnionType(
                    field.clone(),
                    validation_errors.clone(),
                )]
            }
            JsonValidationError::ObjectMissingRequiredField(field) => {
                vec![PathSegment::Field(field.clone())]
            }
            JsonValidationError::ExpectedTypeError((field, expected_type)) => {
                vec![PathSegment::ExpectedType(
                    field.clone(),
                    expected_type.clone(),
                )]
            }
            _ => vec![],
        }
    }

    pub fn incremental_parser(&self, input: String) -> Parser {
        Parser::new(
            input,
            self.registry.clone(),
            self.template_registry.clone(),
            self.adhoc_obj_ref_id_counter.clone(),
            self.adhoc_obj_ids.clone(),
            self.adhoc_obj_refs.clone(),
            self.object_instances.clone(),
            self.import_chain.clone(),
            self.current_module.clone(),
        )
    }

    pub fn parse_llm_output(&self, input: &str) -> Result<JsonValue, String> {
        /* ----------------------------------------------------------
         * 2. Parse the JSON (or JSON-ish) payload
         * -------------------------------------------------------- */
        let mut jp = StreamParser::default();
        let bytes = input.as_bytes().to_vec();

        let mut i = 0;
        let mut end_val = None;
        while i < bytes.len() {
            let remaining = &bytes[i..];
            match std::str::from_utf8(remaining) {
                Ok(valid_str) => {
                    end_val = jp.step(valid_str).unwrap();
                    break;
                }
                Err(e) => {
                    let valid_up_to = e.valid_up_to();
                    if valid_up_to == 0 {
                        // We don't yet have a full character, wait for more bytes
                        break;
                    }
                    let chunk = std::str::from_utf8(&bytes[i..i + valid_up_to]).unwrap();
                    end_val = jp.step(chunk).unwrap();
                    i += valid_up_to;
                }
            }
        }

        let payload = end_val.unwrap();

        Ok(payload)
    }

    fn infer_type_from_fields(&self, fields: &HashMap<String, JsonValue>) -> Option<String> {
        let mut matches: Vec<String> = vec![];

        let reg_borrow = self.registry.borrow();

        for (name, field) in reg_borrow.clone().into_iter() {
            match field.field_type.field_definitions() {
                Some(obj_fields) => {
                    let fieldset: HashSet<String> =
                        obj_fields.iter().map(|field| field.name.clone()).collect();

                    let original_set: HashSet<String> = fields.keys().cloned().collect();

                    let diff = original_set.difference(&fieldset);
                    let diff_vec: Vec<String> = diff.cloned().collect();

                    if (diff_vec.len() == 1 && diff_vec.first().unwrap() == "_type")
                        || diff_vec.len() == 0
                    {
                        matches.push(name.clone())
                    }
                }
                None => {
                    continue;
                }
            }
        }
        drop(reg_borrow);

        if matches.len() > 1 || matches.len() == 0 {
            None
        } else {
            Some(matches.first().unwrap().clone())
        }
    }

    // Helper function to apply fixes based on the path
    pub fn fix_json_value(&self, json: &mut JsonValue, path: &[PathSegment]) -> Result<(), String> {
        if path.is_empty() {
            return Ok(()); // Empty path means no fixes needed
        }

        match path.split_first() {
            Some((PathSegment::MissingMetaType, _)) => {
                if let JsonValue::Object(map) = json {
                    if let Some(inferred_type) = self.infer_type_from_fields(map) {
                        map.insert("_type".to_string(), JsonValue::String(inferred_type));
                        Ok(())
                    } else {
                        Err("Could not unambiguously determine type from fields".to_string())
                    }
                } else {
                    Err("Expected object".to_string())
                }
            }
            Some((PathSegment::ArrayIndex(idx), rest)) => {
                if let JsonValue::Array(arr) = json {
                    if let Some(element) = arr.get_mut(*idx) {
                        self.fix_json_value(element, rest)
                    } else {
                        Err(format!("Expected element in array at index {idx}"))
                    }
                } else {
                    Err("Expected to be array at this level.".to_string())
                }
            }
            Some((PathSegment::Field(field), rest)) => {
                if rest.is_empty() {
                    Err(format!("Missing field {field} from object."))
                } else {
                    if let JsonValue::Object(map) = json {
                        match map.get_mut(field) {
                            None => Err(format!("Missing field {field} from object.")),
                            Some(json2) => self.fix_json_value(json2, rest),
                        }
                    } else {
                        Err("Expected object".to_string())
                    }
                }
            }
            Some((PathSegment::UnionType(_field, validation_errors), _)) => {
                match json {
                    JsonValue::Object(map) => {
                        // Try each possible union type and its validation errors
                        for (_type_name, errors) in validation_errors {
                            // Clone the object to try fixes without modifying original
                            let mut test_json = json.clone();

                            // Get a new path from these errors to try fixing
                            let error_path = self.get_error_location(errors);

                            // Try to fix the validation errors for this type
                            let res = self.fix_json_value(&mut test_json, &error_path);
                            if res.is_ok() {
                                // If successful, apply the changes back to original
                                *json = test_json;
                                return Ok(());
                            }
                        }
                        Err("Could not coerce to any union type".to_string())
                    }
                    _ => Err("Expected object for union type".to_string()),
                }
            }
            Some((PathSegment::ExpectedType(field, expected), rest)) => match field {
                None => {
                    match expected.as_str() {
                        "Array" => {
                            *json = JsonValue::Array(vec![json.to_owned()]);
                            Ok(())
                        }
                        "String" => match json {
                            JsonValue::Number(n) => {
                                *json = JsonValue::String(n.to_string());
                                Ok(())
                            }
                            JsonValue::Boolean(b) => {
                                *json = JsonValue::String(b.to_string());
                                Ok(())
                            }
                            JsonValue::Null => {
                                *json = JsonValue::String("null".to_string());
                                Ok(())
                            }
                            JsonValue::String(_) => Ok(()), // Already a string
                            _ => Err("Cannot convert to string".to_string()),
                        },
                        "Number" => match json {
                            JsonValue::String(s) => {
                                if let Ok(n) = s.parse::<i64>() {
                                    *json = JsonValue::Number(Number::Integer(n.into()));
                                    Ok(())
                                } else {
                                    Err("String is not a valid number".to_string())
                                }
                            }
                            JsonValue::Boolean(b) => {
                                *json = JsonValue::Number(Number::Integer(
                                    if *b { 1 } else { 0 }.into(),
                                ));
                                Ok(())
                            }
                            JsonValue::Number(_) => Ok(()), // Already a number
                            _ => Err("Cannot convert to number".to_string()),
                        },
                        "Boolean" => match json {
                            JsonValue::String(s) => match s.to_lowercase().as_str() {
                                "true" | "1" => {
                                    *json = JsonValue::Boolean(true);
                                    Ok(())
                                }
                                "false" | "0" => {
                                    *json = JsonValue::Boolean(false);
                                    Ok(())
                                }
                                _ => Err("String is not a valid boolean".to_string()),
                            },
                            JsonValue::Number(n) => {
                                if n.is_i64() {
                                    match n.as_i64() {
                                        0 => {
                                            *json = JsonValue::Boolean(false);
                                            Ok(())
                                        }
                                        1 => {
                                            *json = JsonValue::Boolean(true);
                                            Ok(())
                                        }
                                        _ => Err("Number is not 0 or 1".to_string()),
                                    }
                                } else {
                                    Err("Cannot convert float to boolean".to_string())
                                }
                            }
                            JsonValue::Boolean(_) => Ok(()), // Already a bool
                            _ => Err("Cannot convert to boolean".to_string()),
                        },
                        "Null" => {
                            *json = JsonValue::Null;
                            Ok(())
                        }
                        _ => Err("Unrecognized type for conversion".to_string()),
                    }
                }
                Some(field_name) => match json {
                    JsonValue::Object(map) => {
                        if let Some(value) = map.get_mut(field_name) {
                            self.fix_json_value(value, rest)?;
                        }
                        Ok(())
                    }
                    _ => Err(format!(
                        "Cannot access field {} on non-object value",
                        field_name
                    )),
                },
            },
            Some((PathSegment::Root((_template, variable)), rest)) => match variable {
                Some(field_name) => match json {
                    JsonValue::Object(map) => {
                        if let Some(value) = map.get_mut(field_name) {
                            self.fix_json_value(value, rest)?;
                        }
                        Ok(())
                    }
                    _ => Err(format!(
                        "Cannot access field {} on non-object value",
                        field_name
                    )),
                },
                None => {
                    println!("{:?}", json);
                    println!("{:?}", rest);
                    self.fix_json_value(json, rest)
                }
            },
            None => Err("Invalid error path -no segments".to_string()),
        }
    }

    fn validate_type(
        &self,
        wail_type: &WAILType,
        registry: &HashMap<String, WAILField>,
        template_name: &str,
        warnings: &mut Vec<ValidationWarning>,
        errors: &mut Vec<ValidationError>,
        is_return_type: bool,
    ) {
        match wail_type {
            WAILType::Simple(_) => (), // Built-in types are always valid
            WAILType::Composite(composite) => match composite {
                WAILCompositeType::Array(array) => {
                    // Check if the element type exists if it's a custom type
                    if let Some(element_type) = &array.type_data.element_type {
                        let element_type_str = element_type.type_name().to_string();
                        let namespaced_id = self.lookup_symbol_in_registry(&element_type_str);
                        if element_type_str != "String"
                            && element_type_str != "Number"
                            && namespaced_id.is_err()
                        {
                            // For array element types in templates, undefined types are errors
                            errors.push(ValidationError::UndefinedTypeInTemplate {
                                template_name: template_name.to_string(),
                                type_name: element_type_str.clone(),
                                is_return_type,
                            });

                            // Check for possible typos
                            for known_type in registry.keys() {
                                if known_type.len() > 2
                                    && element_type_str.len() > 2
                                    && known_type
                                        .chars()
                                        .zip(element_type_str.chars())
                                        .filter(|(a, b)| a != b)
                                        .count()
                                        <= 2
                                {
                                    warnings.push(ValidationWarning::PossibleTypo {
                                        type_name: element_type_str.clone(),
                                        similar_to: known_type.to_string(),
                                        location: format!(
                                            "array element type in template {}",
                                            template_name
                                        ),
                                    });
                                }
                            }
                        }
                    }
                }
                WAILCompositeType::Union(union) => {
                    // Validate each member type
                    for member_type in &union.members {
                        self.validate_type(
                            &member_type.field_type,
                            registry,
                            template_name,
                            warnings,
                            errors,
                            is_return_type,
                        );
                    }
                }
                WAILCompositeType::Object(object) => {
                    let type_name = object.type_data.type_name.to_string();
                    let namespaced_id = self.lookup_symbol_in_registry(&type_name);
                    if type_name != "String" && type_name != "Number" && namespaced_id.is_err() {
                        // For return types and input parameters in templates, undefined types are errors
                        errors.push(ValidationError::UndefinedTypeInTemplate {
                            template_name: template_name.to_string(),
                            type_name: type_name.clone(),
                            is_return_type,
                        });

                        // Check for possible typos
                        for known_type in registry.keys() {
                            if known_type.len() > 2
                                && type_name.len() > 2
                                && known_type
                                    .chars()
                                    .zip(type_name.chars())
                                    .filter(|(a, b)| a != b)
                                    .count()
                                    <= 2
                            {
                                warnings.push(ValidationWarning::PossibleTypo {
                                    type_name: type_name.clone(),
                                    similar_to: known_type.to_string(),
                                    location: format!(
                                        "{} type in template {}",
                                        if is_return_type {
                                            "return"
                                        } else {
                                            "parameter"
                                        },
                                        template_name
                                    ),
                                });
                            }
                        }
                    }
                }
            },
            WAILType::Value(_) => (), // Literal values are always valid
        }
    }

    fn lookup_symbol_in_registry(&self, name: &str) -> Result<WAILField, WAILParseError> {
        let mut matches = vec![];

        {
            for (key, def) in self.registry.borrow().iter() {
                if let Some(actual_name) = key.split('.').last() {
                    if actual_name == name {
                        matches.push((key.clone(), def.clone()));
                    }
                }
            }
        }

        match matches.len() {
            0 => Err(WAILParseError::SymbolNotFound {
                name: name.to_string(),
            }),
            1 => Ok(matches.remove(0).1.clone()),
            _ => Err(WAILParseError::AmbiguousSymbol {
                name: name.to_string(),
                matches: matches.into_iter().map(|(k, _)| k).collect(),
            }),
        }
    }
    pub fn validate(&self) -> (Vec<ValidationWarning>, Vec<ValidationError>) {
        let mut warnings = Vec::new();
        let mut errors = Vec::new();
        let registry = self.registry.borrow();
        let template_registry = self.template_registry.borrow();

        // Check all templates
        for (template_name, template) in template_registry.iter() {
            // Check input parameters
            for param in &template.inputs {
                self.validate_type(
                    &param.field_type,
                    &registry,
                    template_name,
                    &mut warnings,
                    &mut errors,
                    false,
                );
            }

            // Check return type
            self.validate_type(
                &template.output.field_type,
                &registry,
                template_name,
                &mut warnings,
                &mut errors,
                true,
            );
        }

        drop(template_registry);
        drop(registry);

        (warnings, errors)
    }

    // Add this method to use our token-based parser
    pub fn parse_wail_file_with_tokens(
        &self,
        input_string: String,
        file_type: WAILFileType,
        clear: bool,
    ) -> Result<Vec<WAILDefinition>, WAILParseError> {
        let input: &str = Box::leak(Box::new(input_string));

        if clear {
            self.registry.replace(HashMap::new());
            self.template_registry.replace(HashMap::new());
            self.object_instances.replace(HashMap::new());
            self.adhoc_obj_ids.replace(Vec::new());
            self.adhoc_obj_refs.replace(HashMap::new());
        }

        let mut parser = self.incremental_parser(input.to_string());

        let definitions = parser.parse_wail_file(file_type)?;

        Ok(definitions)
    }

    // Replace the original parse_wail_file method to use our token-based implementation
    pub fn parse_wail_file(
        &self,
        input_string: String,
        file_type: WAILFileType,
        clear: bool,
    ) -> Result<Vec<WAILDefinition>, WAILParseError> {
        self.parse_wail_file_with_tokens(input_string, file_type, clear)
    }

    /* ============================================================================
    Best-guess *with* declared return-type in mind
    ========================================================================== */

    /// Try to guess a value’s WAIL type **but** bias / prune the guess by a
    /// template’s declared return-type (`expected`).  
    /// Returns `None` when we cannot map the JSON into the expected shape at all.
    pub fn guess_against_expected(
        &self,
        val: &JsonValue,
        expected: &WAILType,
    ) -> Option<GuessResult> {
        // 1) Plain guess first
        let mut g = self.guess_type(val)?;
        println!("{:?}", g);

        // 2) If the guess is already *structurally* the same as `expected`,
        //    bump confidence to at least `High`.
        if g.ty.same_shape_as(expected) {
            if g.confidence < GuessConfidence::High {
                g.confidence = GuessConfidence::High;
            }
            return Some(g);
        }

        // 3) If the guess is *within* a union/array that the expected allows,
        //    we still accept it but with `Medium` confidence.
        if expected.accepts(&g.ty) {
            g.confidence = GuessConfidence::Medium;
            return Some(g);
        }

        // 4) Otherwise we can’t coerce it.
        None
    }
}

/* ------------------------------------------------------------------------- */
/*  Public API                                                               */
/* ------------------------------------------------------------------------- */

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]

pub enum GuessConfidence {
    None,
    Low,
    Medium,
    High,
    Exact,
}

#[derive(Debug, Clone)]
pub struct GuessResult {
    pub ty: WAILType,
    pub confidence: GuessConfidence,
}

impl WAILParser {
    pub fn guess_type(&self, val: &JsonValue) -> Option<GuessResult> {
        self.guess_type_inner(val).ok()
    }

    /* ------------------------------------------------------------------ */
    /*  Core dispatch                                                     */
    /* ------------------------------------------------------------------ */
    fn guess_type_inner(&self, val: &JsonValue) -> Result<GuessResult, ()> {
        println!("HEREINNER");
        match val {
            JsonValue::String(_) => {
                println!("HERESTR");
                return scalar(WAILType::Simple(WAILSimpleType::String(Default::default())));
            }
            JsonValue::Boolean(_) => {
                return scalar(WAILType::Simple(
                    WAILSimpleType::Boolean(Default::default()),
                ))
            }
            JsonValue::Number(_) => {
                return scalar(WAILType::Simple(WAILSimpleType::Number(Default::default())))
            }
            JsonValue::Null => return Err(()),

            //  ↓↓↓ add the `return`s here ↓↓↓
            JsonValue::Array(a) => return self.guess_array(a),
            JsonValue::Object(o) => return self.guess_object(o),
        }
        fn scalar(ty: WAILType) -> Result<GuessResult, ()> {
            Ok(GuessResult {
                ty,
                confidence: GuessConfidence::Exact,
            })
        }
    }

    /* ------------------------------------------------------------------ */
    /*  Array                                                             */
    /* ------------------------------------------------------------------ */

    fn guess_array(&self, arr: &[JsonValue]) -> Result<GuessResult, ()> {
        println!("HERE2");
        if arr.is_empty() {
            return Err(());
        }
        let mut guesses = vec![];
        for v in arr {
            if let Ok(g) = self.guess_type_inner(v) {
                guesses.push(g)
            }
        }
        if guesses.is_empty() {
            return Err(());
        }

        if guesses.iter().all(|g| g.ty == guesses[0].ty) {
            let mut array_ty = WAILType::Composite(WAILCompositeType::Array(Default::default()));
            if let WAILType::Composite(WAILCompositeType::Array(ref mut a)) = array_ty {
                a.type_data.element_type = Some(Box::new(guesses[0].ty.clone()));
            }
            return Ok(GuessResult {
                ty: array_ty,
                confidence: guesses[0].confidence,
            });
        }

        // heterogeneous → Array<Union> Low
        let mut members = vec![];
        for (i, g) in guesses.into_iter().enumerate() {
            members.push(WAILField {
                name: format!("member_{i}"),
                field_type: g.ty,
                annotations: vec![],
            });
        }
        let union_ty = WAILType::Composite(WAILCompositeType::Union(Default::default()));
        let mut array_ty = WAILType::Composite(WAILCompositeType::Array(Default::default()));
        if let WAILType::Composite(WAILCompositeType::Array(ref mut a)) = array_ty {
            a.type_data.element_type = Some(Box::new(union_ty));
        }
        Ok(GuessResult {
            ty: array_ty,
            confidence: GuessConfidence::Low,
        })
    }

    /* ------------------------------------------------------------------ */
    /*  Object                                                            */
    /* ------------------------------------------------------------------ */

    fn guess_object(&self, map: &HashMap<String, JsonValue>) -> Result<GuessResult, ()> {
        println!("HEREOBJ");
        /* 0. _type exact */
        if let Some(JsonValue::String(tn)) = map.get("_type") {
            if let Ok(f) = self.lookup_symbol_in_registry(tn) {
                return Ok(GuessResult {
                    ty: f.field_type,
                    confidence: GuessConfidence::Exact,
                });
            }
        }

        println!("HEREOBJ2");

        /* Build snapshot (name, fields, def) */
        let objs: Vec<(String, Vec<String>, WAILField)> = self
            .registry
            .borrow()
            .values()
            .filter_map(|def| match &def.field_type {
                WAILType::Composite(WAILCompositeType::Object(obj)) => {
                    obj.type_data.field_definitions.as_ref().map(|flds| {
                        (
                            obj.type_data.type_name.clone(),
                            flds.iter().map(|f| f.name.clone()).collect(),
                            def.clone(),
                        )
                    })
                }
                _ => None,
            })
            .collect();

        let observed: Vec<String> = map.keys().filter(|k| *k != "_type").cloned().collect();

        println!("observed: {:?}", observed);

        if !observed.is_empty() {
            /* 1. exact field‑set */
            for (_, decl, def) in &objs {
                if decl.len() == observed.len() && decl.iter().all(|d| observed.contains(d)) {
                    return Ok(GuessResult {
                        ty: def.field_type.clone(),
                        confidence: GuessConfidence::Exact,
                    });
                }
            }

            /* 2. all required present → High */
            if let Some(def) = objs
                .iter()
                .find(|(_, decl, _)| observed.iter().all(|k| fuzzy_contains(decl, k)))
            {
                return Ok(GuessResult {
                    ty: def.2.field_type.clone(),
                    confidence: GuessConfidence::High,
                });
            }

            /* 3. subset hits */
            let mut hits: Vec<&WAILField> = objs
                .iter()
                .filter(|(_, decl, _)| fuzzy_subset(&observed, decl))
                .map(|(_, _, d)| d)
                .collect();
            match hits.len() {
                0 => {}
                1 => {
                    return Ok(GuessResult {
                        ty: hits[0].field_type.clone(),
                        confidence: GuessConfidence::Medium,
                    })
                }
                _ => {
                    return Ok(GuessResult {
                        ty: hits[0].field_type.clone(),
                        confidence: GuessConfidence::Low,
                    })
                }
            }
        }

        /* 4. fuzzy _type */
        if let Some(JsonValue::String(tn)) = map.get("_type") {
            let matches: Vec<&WAILField> = objs
                .iter()
                .filter(|(name, _, _)| damerau_levenshtein(name, tn) <= 1)
                .map(|(_, _, d)| d)
                .collect();
            match matches.len() {
                0 => {}
                1 => {
                    return Ok(GuessResult {
                        ty: matches[0].field_type.clone(),
                        confidence: GuessConfidence::Exact,
                    })
                }
                _ => {
                    return Ok(GuessResult {
                        ty: matches[0].field_type.clone(),
                        confidence: GuessConfidence::High,
                    })
                }
            }
        }

        Err(())
    }

    fn is_declared_type(&self, ty: &WAILType) -> bool {
        let name = match ty {
            // Array ⇒ look through one layer and grab the element’s name
            WAILType::Composite(WAILCompositeType::Array(arr)) => arr
                .type_data
                .element_type
                .as_ref()
                .map(|elem| elem.type_name()),
            // Otherwise just take this type’s own name
            _ => Some(ty.type_name()),
        };

        name.map(|n| self.registry.borrow().contains_key(&n))
            .unwrap_or(false)
    }
}

/* ------------------------------------------------------------------------- */
/*  Fuzzy helpers (w/ Damerau‑Levenshtein)                                   */
/* ------------------------------------------------------------------------- */

const MAX_DIST: usize = 1;

fn fuzzy_contains(declared: &[String], key: &str) -> bool {
    declared
        .iter()
        .any(|d| damerau_levenshtein(d, key) <= MAX_DIST)
}

fn fuzzy_subset(observed: &[String], declared: &[String]) -> bool {
    observed.iter().all(|k| fuzzy_contains(declared, k))
}

#[derive(Debug, Clone, PartialEq)]
pub enum WAILDefinition {
    Object(WAILField),
    Template(WAILTemplateDef),
    Union(WAILField),
    Comment(String),
    Import(WAILImport),
}

impl WAILDefinition {
    fn get_name(&self) -> Option<&str> {
        match self {
            WAILDefinition::Object(field) => Some(&field.name),
            WAILDefinition::Template(template) => Some(&template.name),
            WAILDefinition::Union(field) => Some(&field.name),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ValidationWarning {
    UndefinedType {
        type_name: String,
        location: String,
    },
    PossibleTypo {
        type_name: String,
        similar_to: String,
        location: String,
    },
}

#[derive(Debug, Clone)]
pub enum ValidationError {
    UndefinedTypeInTemplate {
        template_name: String,
        type_name: String,
        is_return_type: bool,
    },
}

impl WAILParser {
    /// 1️⃣ parse JSON → 2️⃣ validate+fix → 3️⃣ guess type (guaranteed declared)  
    ///
    /// * `json` is the raw LLM chunk (already JSON-ish enough for `JsonParser`).  
    /// * `expected` is the schema node you know you’re expecting
    ///   (e.g. `first_template_output(&parser)`).
    ///
    /// On success you get `(fixed_json, guess)` where  
    /// `guess.ty` is **always** a declared type (or Array/Union whose
    /// element/member types are all declared).
    pub fn fix_and_guess<'a>(
        &self,
        mut json: JsonValue,
        expected: &'a WAILType,
    ) -> Result<(JsonValue, GuessResult), String> {
        // 2️⃣  guess against the schema
        let mut guess = self
            .guess_against_expected(&json, expected)
            .ok_or_else(|| format!("failed to guess type from JSON: {}", json.to_string()))?;

        // 3️⃣  ensure the guess references only declared types
        if !self.is_declared_or_composite_of_declared(&guess.ty) {
            // downgrade or bail – up to you; here we treat as “no usable guess”
            return Err("no declared type matched".into());
        }

        Ok((json, guess))
    }

    /// helper: works for Array<…>, Union<…> etc.
    fn is_declared_or_composite_of_declared(&self, ty: &WAILType) -> bool {
        match ty {
            WAILType::Composite(WAILCompositeType::Array(arr)) => arr
                .type_data
                .element_type
                .as_ref()
                .map_or(false, |t| self.is_declared_or_composite_of_declared(t)),
            WAILType::Composite(WAILCompositeType::Union(un)) => un
                .members
                .iter()
                .all(|fld| self.is_declared_or_composite_of_declared(&fld.field_type)),
            _ => self.is_declared_type(ty),
        }
    }
}

#[cfg(test)]
mod tests;
