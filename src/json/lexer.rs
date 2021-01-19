use std::fmt;

#[derive(Copy, Clone)]
pub enum LexerState {
    Start,
    String,
    Numeric,
    //Boolean, <- I use a lookahed for this state instead
    Finished,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum JsToken {
    Open(char),
    Close(char),
    Colon,
    Comma,
    String { lbound: u32, ubound: u32 },
    Number(f32),
    Boolean(bool),
    Unknown,
}
impl JsToken {
    fn is_unknown(&self) -> bool {
        if let Self::Unknown = self {
            true
        } else {
            false
        }
    }
}
impl fmt::Display for JsToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open(c) => write!(f, "open({})", c),
            Self::Close(c) => write!(f, "close({})", c),
            Self::Colon => write!(f, "colon"),
            Self::Comma => write!(f, "comma"),
            Self::String { lbound, ubound } => write!(f, "String(l:{},u:{})", lbound, ubound),
            Self::Number(val) => write!(f, "Number({})", val),
            Self::Boolean(val) => write!(f, "Boolean({})", val),
            Self::Unknown => write!(f, "unknown"),
        }?;
        Ok(())
    }
}

pub enum LexerError {
    InvalidIdentifier(String),
    InvalidNumber(String),
}

pub struct JsLexer {
    token_stream: Vec<JsToken>,
    state: LexerState,
}

impl JsLexer {
    pub fn new() -> Self {
        Self {
            token_stream: Vec::new(),
            state: LexerState::Start,
        }
    }

    pub fn get_tok_stream(&self) -> &Vec<JsToken> {
        &self.token_stream
    }

    pub fn lex(&mut self, raw_text: &String) -> Result<(), LexerError> {
        let token_stream = &mut self.token_stream;
        let mut char_stream = raw_text.chars().enumerate().peekable();

        let mut accum: Option<usize> = None;

        loop {
            let cur = char_stream.next();
            let peek = char_stream.peek().map(|&a| a);

            let state = self.state;

            match state {
                LexerState::Start => match cur {
                    Some((k, c)) => {
                        let token = match c {
                            '{' => JsToken::Open('{'),
                            '[' => JsToken::Open('['),
                            '}' => JsToken::Close('}'),
                            ']' => JsToken::Close(']'),
                            ':' => JsToken::Colon,
                            ',' => JsToken::Comma,
                            't' | 'f' => {
                                if let Some(val) = Self::is_boolean(char_stream.clone()) {
                                    let ignore_count = if c == 't' { 3 } else { 4 };
                                    Self::flush_chars(&mut char_stream, ignore_count);
                                    JsToken::Boolean(val)
                                } else {
                                    let err = format!("starts at pos ={}", k);
                                    return Err(LexerError::InvalidIdentifier(err));
                                }
                            }
                            '0'..='9' | '.' | '+' | '-' => {
                                accum = Some(k);
                                self.state = LexerState::Numeric;
                                JsToken::Unknown
                            }
                            '\"' => {
                                accum = Some(k);
                                self.state = LexerState::String;
                                JsToken::Unknown
                            }
                            _ => JsToken::Unknown,
                        };
                        if token.is_unknown() == false {
                            token_stream.push(token);
                        }
                        if cur.is_some() {
                            let (_, c) = cur.unwrap();
                            if c.is_alphabetic() && c != 't' && c != 'f' {
                                let err = format!("starts at pos ={}", k);
                                return Err(LexerError::InvalidIdentifier(err));
                            }
                        } else {
                            self.state = LexerState::Finished;
                        }
                    }
                    _ => self.state = LexerState::Finished,
                },
                LexerState::String => {
                    if cur.is_none() {
                        self.state = LexerState::Finished;
                    }
                    if let (Some((_, '\\')), Some((_, '\"'))) = (cur, peek) {
                        Self::flush_chars(&mut char_stream, 1);
                    }
                    if let Some((k, '\"')) = cur {
                        token_stream.push(JsToken::String {
                            lbound: accum.unwrap() as u32,
                            ubound: k as u32,
                        });
                        accum = None;
                        self.state = LexerState::Start;
                    }
                }
                LexerState::Numeric => {
                    if cur.is_none() {
                        self.state = LexerState::Finished;
                    }

                    if let Some((ubound, c)) = cur {
                        if c == ',' || c == ']' || c == '}' {
                            let lbound = accum.unwrap();
                            let num_slice = &raw_text[lbound..ubound];
                            let num_res = num_slice.trim().parse::<f32>();
                            if let Ok(num) = num_res {
                                token_stream.push(JsToken::Number(num));

                                let tok = match c {
                                    ',' => JsToken::Comma,
                                    ']' => JsToken::Close(']'),
                                    '}' => JsToken::Close('}'),
                                    _ => JsToken::Unknown,
                                };

                                token_stream.push(tok);

                                self.state = LexerState::Start;
                                accum = None;
                            } else {
                                let err = format!("at pos = {}", num_slice);
                                return Err(LexerError::InvalidNumber(err));
                            }
                        }
                    }
                }
                LexerState::Finished => {
                    break;
                }
            }
        }
        Ok(())
    }

    fn flush_chars(mut stream: impl Iterator<Item = (usize, char)>, num_chars: u32) {
        for _ in 0..num_chars {
            stream.next();
        }
    }

    fn is_boolean<It>(stream: It) -> Option<bool>
    where
        It: Iterator<Item = (usize, char)> + Clone,
    {
        let case1 = "true";
        let case2 = "false";

        let true_detected = case1[1..]
            .chars()
            .zip(stream.clone())
            .all(|(a, (_, b))| a == b);

        let false_detected = case2[1..]
            .chars()
            .zip(stream.clone())
            .all(|(a, (_, b))| a == b);

        if true_detected {
            Some(true)
        } else if false_detected {
            Some(false)
        } else {
            None
        }
    }
}
