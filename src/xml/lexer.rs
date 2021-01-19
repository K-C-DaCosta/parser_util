use std::collections::HashMap;
use super::XMLErrorKind; 

#[derive(Copy, Clone)]
pub enum XMLTokenKind {
    //a token is either 'open',close,openclose, or inner;
    Open,
    Close,
    OpenClose,
    Inner,
    //aux token types(these act like states)
    Unknown,
    OpenAttribOpen,
    OpenAttribClose,
    Comment,
}

pub struct XmlToken {
    pub token_kind: XMLTokenKind,
    pub content: String,
    pub attribs: HashMap<String, String>,
}

impl XmlToken {
    pub fn new(token_kind: XMLTokenKind, content: String) -> XmlToken {
        XmlToken {
            token_kind,
            content,
            attribs: HashMap::new(),
        }
    }
}
impl Default for XmlToken {
    fn default() -> XmlToken {
        XmlToken {
            token_kind: XMLTokenKind::Unknown,
            content: String::new(),
            attribs: HashMap::new(),
        }
    }
}

/// Can correctly parse  only  a subset of XML grammar *only*.\
/// I repeat, this code  cannot parse the entire XML grammar. The parser was intented to parse xml that stores raw data.\
/// All the `<!DOCTYPE .. >`, `<!ENTITY ..>` stuff has been cut out of the grammar in this parser \
/// Comments should still work though.
pub struct Lexer {
    pub tokens: Vec<Option<XmlToken>>,
}

impl Lexer {
    pub fn new() -> Lexer {
        Lexer { tokens: Vec::new() }
    }

    ///tokenizes raw  xml text with FSM logic
    pub fn lex(&mut self, src: &str) -> Result<(), XMLErrorKind> {
        use XMLTokenKind::*;
        let mut state = Unknown;
        let mut accum = String::new();
        let mut current_key = String::new();

        let mut char_iter = src.chars().peekable();
        while let Some(c) = char_iter.next() {
            match state {
                Unknown => {
                    if c == '<' {
                        state = Open;
                    }
                }
                Open => {
                    if c == '>' {
                        state = Inner;
                        self.push_token(Open, &mut accum);
                    } else if let ('/', Some('>')) = (c, char_iter.peek()) {
                        state = Inner;
                        char_iter.next();
                        self.push_token(OpenClose, &mut accum);
                    } else if let (' ', Some(lookahead)) = (c, char_iter.peek()) {
                        if lookahead.is_alphabetic() {
                            state = XMLTokenKind::OpenAttribOpen;
                            //label token as "open" by default
                            self.push_token(Open, &mut accum);
                        }
                    } else {
                        let adding_first_character = accum.len() == 0;
                        if adding_first_character {
                            if c.is_alphabetic() {
                                accum.push(c);
                            }
                        } else {
                            accum.push(c);
                        }
                    }
                }
                OpenAttribOpen => {
                    if c == '=' {
                        current_key = accum.clone();
                        accum.clear();
                        if let Some('"') = char_iter.peek() {
                            state = OpenAttribClose;
                            char_iter.next();
                        } else {
                            return Err(XMLErrorKind::TokenizerErr(
                                "expected: '\"' right after '='",
                            ));
                        }
                    } else if c == '>' {
                        state = Inner;
                    } else if let ('/', Some('>')) = (c, char_iter.peek()) {
                        state = Inner;
                        char_iter.next();

                        //make sure existing open token is flagged as "openclose"
                        let open_token = &mut self.tokens.last_mut().unwrap().as_mut().unwrap();
                        open_token.token_kind = OpenClose;
                    } else {
                        accum.push(c);
                    }
                }
                OpenAttribClose => {
                    if c == '\"' {
                        let open_token = &mut self.tokens.last_mut().unwrap().as_mut().unwrap();
                        open_token
                            .attribs
                            .insert(current_key.clone(), accum.clone());
                        accum.clear();

                        state = XMLTokenKind::OpenAttribOpen;
                    } else {
                        accum.push(c);
                    }
                }
                Close => {
                    if c == '>' {
                        state = Inner;
                        self.push_token(Close, &mut accum);
                    } else {
                        accum.push(c);
                    }
                }
                Inner => {
                    if c == '<' {
                        let peek = char_iter.peek();
                        if let Some('/') = peek {
                            char_iter.next();
                            state = Close;
                        } else if let Some('!') = peek {
                            char_iter.next();
                            state = Comment;
                        } else {
                            state = Open;
                        }
                        self.push_token(Inner, &mut accum);
                    } else {
                        accum.push(c);
                    }
                }
                Comment => {
                    if c == '-' {
                        let peek = char_iter.peek();
                        if let Some('>') = peek {
                            state = Inner;
                            char_iter.next();
                        }
                    }
                }
                _ => (),
            }
        }
        Ok(())
    }

    fn push_token(&mut self, token_kind: XMLTokenKind, accum: &mut String) {
        if accum.len() == 0 || accum.trim().len() == 0 {
            accum.clear();
            return;
        }

        let token = XmlToken::new(token_kind, accum.clone());
        self.tokens.push(Some(token));
        accum.clear();
    }
    #[allow(dead_code)]
    pub fn print_tokens(&self) {
        for tok in self.tokens.iter() {
            match &tok {
                Some(XmlToken {
                    token_kind: XMLTokenKind::Open,
                    content: txt,
                    ..
                }) => {
                    println!("kind=Open Content=\'{}\'", txt);
                }
                Some(XmlToken {
                    token_kind: XMLTokenKind::Inner,
                    content: txt,
                    ..
                }) => {
                    println!("kind=Inner Content=\'{}\'", txt.trim());
                }
                Some(XmlToken {
                    token_kind: XMLTokenKind::Close,
                    content: txt,
                    ..
                }) => {
                    println!("kind=Close Content=\'{}\'", txt);
                }
                Some(XmlToken {
                    token_kind: XMLTokenKind::OpenClose,
                    content: txt,
                    ..
                }) => {
                    println!("kind=OpenClose Content=\'{}\'", txt.trim());
                }
                _ => println!("kind=???"),
            }
        }
    }
}
