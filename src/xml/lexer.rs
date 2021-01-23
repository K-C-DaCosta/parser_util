use std::array;

use super::XMLErrorKind;
/// # Description
/// A token is either:
/// - `OpenTag`
/// - `CloseTag`
/// - `EmptyTag`
/// - `ContentTag` - raw text
/// # Comments
/// The rest of the `TokenKinds` are for states in the lexer
#[derive(Copy, Clone, PartialEq)]
pub enum XmlTokenKind {
    ///This token is for  tages like: `<foo>`
    OpenTag,
    ///This token is for  tages like: `</foo>`
    CloseTag,
    ///This token is for  tages like: `<foo/>`
    EmptyTag,
    ///This tag just has text and nothing else in it
    ContentTag,
    AuxUnknown,
    AuxOpenAttribOpen,
    AuxOpenAttribClose,
    AuxComment,
}
impl XmlTokenKind {
    pub fn is_emptytag(&self) -> bool {
        if let Self::EmptyTag = self {
            true
        } else {
            false
        }
    }
}

#[derive(Clone)]
pub struct XmlToken {
    pub token_kind: XmlTokenKind,
    pub content: String,
    pub attribs: Vec<(String, String)>,
}

impl XmlToken {
    pub fn new(token_kind: XmlTokenKind, content: String) -> XmlToken {
        XmlToken {
            token_kind,
            content,
            attribs: Vec::new(),
        }
    }
    /// # Description
    /// searches for attribute(key) and returns its associated value
    /// # Comments
    /// - I need these functions now that I've removed hash-table from this struct
    /// - I figure a vector would preform much better than a std::collections::HashMap
    pub fn get_attrib(&self, key: &str) -> Option<&String> {
        self.attribs
            .iter()
            .filter(|(k, _)| k.as_str() == key)
            .next()
            .map(|(_, v)| v)
    }

    pub fn get_attrib_mut(&mut self, attrib: &str) -> Option<&mut String> {
        self.attribs
            .iter_mut()
            .filter(|(k, _)| k.as_str() == attrib)
            .next()
            .map(|(_, v)| v)
    }
}
impl Default for XmlToken {
    fn default() -> XmlToken {
        XmlToken {
            token_kind: XmlTokenKind::AuxUnknown,
            content: String::new(),
            attribs: Vec::new(),
        }
    }
}

/// Can correctly parse  only  a subset of XML grammar *only*.\
/// I repeat, this code  cannot parse the entire XML grammar. The parser was intented to parse xml that stores raw data.\
/// All the `<!DOCTYPE .. >`, `<!ENTITY ..>` stuff has been cut out of the grammar in this parser \
/// Comments should still work though.
pub struct XmlLexer {
    pub tokens: Vec<Option<XmlToken>>,
}

impl XmlLexer {
    pub fn new() -> XmlLexer {
        XmlLexer { tokens: Vec::new() }
    }

    ///tokenizes raw  xml text with FSM logic
    pub fn lex(&mut self, src: &str) -> Result<(), XMLErrorKind> {
        let mut state = XmlTokenKind::AuxUnknown;
        let mut accum = String::new();
        let mut current_key = String::new();

        let mut char_iter = src.chars().peekable();
        while let Some(c) = char_iter.next() {
            match state {
                XmlTokenKind::AuxUnknown => {
                    if c == '<' {
                        state = XmlTokenKind::OpenTag;
                    }
                }
                XmlTokenKind::OpenTag => {
                    if c == '>' {
                        state = XmlTokenKind::ContentTag;
                        self.push_token(XmlTokenKind::OpenTag, &mut accum);
                    } else if let ('/', Some('>')) = (c, char_iter.peek()) {
                        state = XmlTokenKind::ContentTag;
                        char_iter.next();
                        self.push_token(XmlTokenKind::EmptyTag, &mut accum);
                    } else if let (' ', Some(lookahead)) = (c, char_iter.peek()) {
                        if lookahead.is_alphabetic() {
                            state = XmlTokenKind::AuxOpenAttribOpen;
                            //label token as "open" by default
                            self.push_token(XmlTokenKind::OpenTag, &mut accum);
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
                XmlTokenKind::AuxOpenAttribOpen => {
                    if c == '=' {
                        current_key = accum.clone();
                        accum.clear();
                        if let Some('"') = char_iter.peek() {
                            state = XmlTokenKind::AuxOpenAttribClose;
                            char_iter.next();
                        } else {
                            return Err(XMLErrorKind::TokenizerErr(
                                "expected: '\"' right after '='",
                            ));
                        }
                    } else if c == '>' {
                        state = XmlTokenKind::ContentTag;
                    } else if let ('/', Some('>')) = (c, char_iter.peek()) {
                        state = XmlTokenKind::ContentTag;
                        char_iter.next();

                        //make sure existing open token is flagged as "openclose"
                        let open_token = &mut self.tokens.last_mut().unwrap().as_mut().unwrap();
                        open_token.token_kind = XmlTokenKind::EmptyTag;
                    } else {
                        accum.push(c);
                    }
                }
                XmlTokenKind::AuxOpenAttribClose => {
                    if c == '\"' {
                        let open_token = &mut self.tokens.last_mut().unwrap().as_mut().unwrap();
                        open_token
                            .attribs
                            .push((current_key.clone(), accum.clone()));
                        accum.clear();

                        state = XmlTokenKind::AuxOpenAttribOpen;
                    } else {
                        accum.push(c);
                    }
                }
                XmlTokenKind::CloseTag => {
                    if c == '>' {
                        state = XmlTokenKind::ContentTag;
                        self.push_token(XmlTokenKind::CloseTag, &mut accum);
                    } else {
                        accum.push(c);
                    }
                }
                XmlTokenKind::ContentTag => {
                    if c == '<' {
                        let peek = char_iter.peek();
                        if let Some('/') = peek {
                            char_iter.next();
                            state = XmlTokenKind::CloseTag;
                        } else if let Some('!') = peek {
                            char_iter.next();
                            state = XmlTokenKind::AuxComment;
                        } else {
                            state = XmlTokenKind::OpenTag;
                        }
                        self.push_token(XmlTokenKind::ContentTag, &mut accum);
                    } else {
                        accum.push(c);
                    }
                }
                XmlTokenKind::AuxComment => {
                    if c == '-' {
                        let peek = char_iter.peek();
                        if let Some('>') = peek {
                            state = XmlTokenKind::ContentTag;
                            char_iter.next();
                        }
                    }
                }
                _ => (),
            }
        }
        Ok(())
    }

    fn push_token(&mut self, token_kind: XmlTokenKind, accum: &mut String) {
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
                    token_kind: XmlTokenKind::OpenTag,
                    content: txt,
                    ..
                }) => {
                    println!("kind=Open Content=\'{}\'", txt);
                }
                Some(XmlToken {
                    token_kind: XmlTokenKind::ContentTag,
                    content: txt,
                    ..
                }) => {
                    println!("kind=Inner Content=\'{}\'", txt.trim());
                }
                Some(XmlToken {
                    token_kind: XmlTokenKind::CloseTag,
                    content: txt,
                    ..
                }) => {
                    println!("kind=Close Content=\'{}\'", txt);
                }
                Some(XmlToken {
                    token_kind: XmlTokenKind::EmptyTag,
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
