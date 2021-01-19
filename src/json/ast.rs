use sandboxed_collections::{
    narytree::*,
}; 

use super::{lexer::*};

pub enum ParseError {
    LexFailed(LexerError),
    MissingCloseBracket,
    BracketMismatch,
    MissingProperty,
    InvalidToken(&'static str),
}

impl From<LexerError> for ParseError {
    fn from(err: LexerError) -> Self {
        Self::LexFailed(err)
    }
}

pub struct JsAst {
    lexer: JsLexer,
    ast: NaryTree<JsToken>,
}

impl JsAst {
    pub fn new() -> Self {
        Self {
            ast: NaryTree::new(),
            lexer: JsLexer::new(),
        }
    }
    /// # Description
    /// Lexes and parses json
    /// # Arguments
    /// - `raw_text` :  raw json text
    /// # Comments
    /// - I was going to do a recursive decenst parser but I instread decided to do a
    /// stack/PDA based parser instead. The main idea is simple:
    ///     - Create a stack of parent nodes
    ///     - If I see and open bracket push node onto parent stack
    ///     - If I see a close bracket keep poping off parent stack until top of the stack is a bracket node (pop atleast one)
    ///     - If I see a comma token keep popping off parent stack until top of stack is a bracket node (pop 0 or more)
    ///     - For any other token I add it as the child of the parent (roughly though, there are a bunch of sub cases to this)
    pub fn parse(&mut self, raw_text: &String) -> Result<(), ParseError> {
        self.lexer.lex(raw_text)?;

        //predicate the checks if something is comma close_bracket
        let is_comma_or_close = |tok| match tok {
            JsToken::Comma | JsToken::Close(_) => true,
            _ => false,
        };

        let mut parent_stack: Vec<NodeAddr> = Vec::new();
        let mut token_stream = self.lexer.get_tok_stream().iter().peekable();
        let mut first_alloc = true;

        while let Some(&tok) = token_stream.next() {
            // print_token(tok, "current token", raw_text);
            match tok {
                JsToken::Open('[') | JsToken::Open('{') => {
                    let addr = Self::parse_allocate(&mut self.ast, Some(tok), &mut first_alloc);
                    if let Some(&parent_addr) = parent_stack.last() {
                        self.ast[parent_addr].add_child(addr, parent_addr);
                    }
                    parent_stack.push(addr);
                }

                JsToken::Number(_) | JsToken::String { .. } | JsToken::Boolean(_) => {
                    if let Some(&parent_addr) = parent_stack.last() {
                        let &parent_token = self.ast[parent_addr].data.as_ref().unwrap();
                        // print_token(parent_token, "parent token", raw_text);

                        if JsToken::Open('[') == parent_token {
                            let child_addr =
                                Self::parse_allocate(&mut self.ast, Some(tok), &mut first_alloc);

                            self.ast[parent_addr].add_child(child_addr, parent_addr);

                            if lookahead(token_stream.clone(), is_comma_or_close) == false {
                                return Err(ParseError::InvalidToken(
                                    "missing colon or close bracket",
                                ));
                            }
                        } else if JsToken::Open('{') == parent_token {
                            if let JsToken::String { .. } = tok {
                                let child_addr = Self::parse_allocate(
                                    &mut self.ast,
                                    Some(tok),
                                    &mut first_alloc,
                                );
                                self.ast[parent_addr].add_child(child_addr, parent_addr);

                                if let Some(&&lookahead) = token_stream.peek() {
                                    match lookahead {
                                        JsToken::Colon => {
                                            token_stream.next();
                                        }
                                        JsToken::Close(_) => (),
                                        _ => {
                                            return Err(ParseError::InvalidToken("missing colon"));
                                        }
                                    }
                                }
                                parent_stack.push(child_addr);
                            } else {
                                return Err(ParseError::MissingProperty);
                            }
                        } else if let JsToken::String { .. } = parent_token {
                            let child_addr =
                                Self::parse_allocate(&mut self.ast, Some(tok), &mut first_alloc);
                            self.ast[parent_addr].add_child(child_addr, parent_addr);

                            if lookahead(token_stream.clone(), is_comma_or_close) == false {
                                return Err(ParseError::InvalidToken(
                                    "missing colon or close bracket",
                                ));
                            }

                            match tok {
                                JsToken::Number(_) | JsToken::Boolean(_) => {
                                    // pop because numbers and booleans should NEVER have children.
                                    // only strings and open brackets should have children.
                                    parent_stack.pop();
                                }
                                _ => parent_stack.push(child_addr),
                            }
                        }
                    }
                }
                JsToken::Comma => {
                    if let Some(&parent_addr) = parent_stack.last() {
                        let mut parent_token = self.ast[parent_addr].data.unwrap();
                        // print_token(parent_token, "parent token", raw_text);

                        if parent_token != JsToken::Open('{') && parent_token != JsToken::Open('[')
                        {
                            while let Some(parent_addr) = parent_stack.pop() {
                                parent_token = self.ast[parent_addr].data.unwrap();
                                if let JsToken::Open(_) = parent_token {
                                    parent_stack.push(parent_addr);
                                    break;
                                }
                            }
                        }
                    }
                }

                JsToken::Close(cb_char) => {
                    let ob_char = match cb_char {
                        '}' => '{',
                        ']' => '[',
                        _ => panic!("invalid close bracket type detected in token. The lexer is likely bugged. Review lexer.rs"),
                    };

                    if parent_stack.is_empty() {
                        return Err(ParseError::MissingCloseBracket);
                    }
                    let mut parent_tok = JsToken::Unknown;

                    // keep popping until I reach the nearest array or object in the parent stack.
                    // this should make it such that String token only ever has at most ONE child specifically im talking
                    // about the  properties in json Object
                    while let Some(parent_addr) = parent_stack.pop() {
                        parent_tok = self.ast[parent_addr]
                            .data
                            .as_ref()
                            .map(|&parent| parent)
                            .unwrap();
                        if let JsToken::Open(_) = parent_tok {
                            break;
                        }
                    }

                    if JsToken::Open(ob_char) != parent_tok {
                        return Err(ParseError::BracketMismatch);
                    }
                }
                _ => (),
            }
        }
        Ok(())
    }

    pub fn print_token_stream(&mut self, raw_text: &String) {
        for tok in self.lexer.get_tok_stream() {
            if let &JsToken::String { lbound, ubound } = tok {
                println!("String:{}", &raw_text[1 + lbound as usize..ubound as usize]);
            } else {
                println!("tok:{}", tok);
            }
        }
    }

    pub fn print_tree(&self, raw_text: &String) {
        let mut space_stack = String::new();
        self.print_tree_helper(self.ast.root, &mut space_stack, raw_text);
    }

    fn print_tree_helper(&self, root: NodeAddr, space_stack: &mut String, raw_text: &String) {
        if root == NULL {
            return;
        }

        let ast_node = &self.ast[root];
        let token = ast_node.data.as_ref().unwrap();

        if let &JsToken::String { lbound, ubound } = token {
            println!(
                "{}{:?}",
                space_stack,
                &raw_text[1 + lbound as usize..ubound as usize]
            )
        } else {
            println!("{}{}", space_stack, token)
        }

        for &children in ast_node.children.iter() {
            space_stack.push('.');
            self.print_tree_helper(children, space_stack, raw_text);
            space_stack.pop();
        }
    }

    pub fn parse_allocate(
        ast: &mut NaryTree<JsToken>,
        data: Option<JsToken>,
        first_alloc: &mut bool,
    ) -> NodeAddr {
        let addr = ast.allocate_node(data);
        if *first_alloc {
            ast.root = addr;
        }
        *first_alloc = false;
        addr
    }
}

#[allow(dead_code)]
fn print_token(tok: JsToken, caption: &'static str, raw_text: &String) {
    if let JsToken::String { lbound, ubound } = tok {
        println!(
            "{}={}",
            caption,
            &raw_text[1 + lbound as usize..ubound as usize]
        );
    } else {
        println!("{}={}", caption, tok);
    }
}

/// Used to detect tokens via `predicate`, that should be further upstream
/// returns `true` if detection was true otherwise `false
pub fn lookahead<'a, CB>(token_stream: impl Iterator<Item = &'a JsToken>, predicate: CB) -> bool
where
    CB: Fn(JsToken) -> bool,
{
    token_stream.filter(|&&tok| predicate(tok)).next().is_some()
}
