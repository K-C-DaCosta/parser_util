#[allow(unused_imports)]
use parser_util::json::{ast::*,lexer::*};
use std::fs;

fn main() {
    let mut json_ast = JsonAst::new();
    let raw_text = fs::read_to_string("./json_examples/ex1.json").unwrap();
    if let Err(_) = json_ast.parse(&raw_text) {
        println!("Parse Failed!");
        json_ast.print_token_stream(&raw_text);
    } else {
        println!("\nParse Tree Dump");
        json_ast.print_tree(&raw_text);
    }
    // let raw_text = fs::read_to_string("./json_examples/base64.txt").unwrap();
    // let encoding = Base64::encode(raw_text.as_bytes());
    // println!("encoding:\n{}", encoding);
}
