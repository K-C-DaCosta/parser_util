pub mod ast; 
pub mod lexer; 

#[derive(Debug)]
pub enum XMLErrorKind {
    TokenizerErr(&'static str),
    ParserErr(&'static str),
}