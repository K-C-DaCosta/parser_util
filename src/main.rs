#![allow(unused_imports)]

use parser_util::{
    json::{ast::*, lexer::*},
    xml::ast::*,
};
use std::{fmt, fs, ops, path::Path};

use regex::*;

struct Fixed1616 {
    data: i32,
}
impl From<i32> for Fixed1616 {
    fn from(a: i32) -> Self {
        Self { data: a << 16 }
    }
}
impl Fixed1616 {
    pub fn new(val: f32) -> Self {
        Self {
            data: (val *  (1<<16) as f32  ) as i32,
        }
    }
}

impl ops::Add for Fixed1616 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            data: self.data + rhs.data,
        }
    }
}

impl ops::Sub for Fixed1616 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            data: self.data - rhs.data,
        }
    }
}
impl ops::Mul for Fixed1616 {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        let val = (self.data >> 8)  * (rhs.data >> 8) ;
        Self { data: val }
    }
}

impl fmt::Display for Fixed1616 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const SCALE_FACTOR:f32 = (1<<16) as f32; 
        write!(f, "{}", self.data as f32 / SCALE_FACTOR )
    }
}

fn main() {
    // let list = vec![1,2,3,4,5,6,7,8,9,10];
    // for c in list.chunks(2).enumerate(){
    //     println!("{:?}",c);
    // }
    // std::slice::from_raw_parts_mut(data, len)
    let a = Fixed1616::new(3.14159);
    println!("val = {}", a*Fixed1616::from(2)  );

    // let asd_template = load_and_parse("../blackbot/resources/xml/thread_thumbnail.xml");
    // xml_clone_test();

    // let regex = Regex::new("[^{}/]*/?[a-zA-Z0-9]+\\.[^{}/]+").unwrap();
    // if regex.is_match("a/dasdasdsa/aasdsd/asdasd/asd/asd/a/sd/asd/asdaw.a") {
    //     println!("Yay matches :)");
    // } else {
    //     println!("doesnt match :( ");
    // }
}

pub fn load_and_parse<P: AsRef<Path>>(p: P) -> XmlAst {
    let xml_text = fs::read_to_string(p).expect("missing file");
    let mut parser = XmlParser::new();
    parser.parse(&xml_text).expect("parse error");
    parser.into_ast()
}

pub fn json_test() {
    // let mut json_ast = JsonAst::new();
    // let raw_text = fs::read_to_string("./json_examples/ex1.json").unwrap();
    // if let Err(_) = json_ast.parse(&raw_text) {
    //     println!("Parse Failed!");
    //     json_ast.print_token_stream(&raw_text);
    // } else {
    //     println!("\nParse Tree Dump");
    //     json_ast.print_tree(&raw_text);
    // }
}

pub fn xml_clone_test() {
    let mut thread_template = load_and_parse("../blackbot/resources/xml/thread.xml");
    let post_template = load_and_parse("../blackbot/resources/xml/thread_post.xml");

    let post_ptr = thread_template.clone_tree(&post_template);

    let opt_thread_body_ptr = thread_template.ast.search(0, |node| {
        let token = node.data.as_ref().expect("Option::None found in search");
        match token.get_attrib("class") {
            Some(val) => {
                if val.as_str() == "thread_body" {
                    true
                } else {
                    false
                }
            }
            None => false,
        }
    });

    if let Some(thread_body_ptr) = opt_thread_body_ptr {
        thread_template.ast.add_child(thread_body_ptr, post_ptr);
    }

    println!("{}", thread_template.to_xml_trim());
}
