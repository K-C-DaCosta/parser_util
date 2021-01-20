use super::lexer::{XmlLexer, XmlToken, XmlTokenKind};
use super::XMLErrorKind;
use sandboxed_collections::naryforest::{Pointer, NULL, *};
use std::ops;

/// Can correctly parse  only  a subset of XML grammar *only*.\
/// I repeat, this code  cannot parse the entire XML grammar. The parser was intented to parse xml that stores raw data.\
/// All the `<!DOCTYPE .. >`, `<!ENTITY ..>` stuff has been cut out of the grammar in this parser \
/// Comments should still work though.
pub struct XmlParser {
    pub lexer: XmlLexer,
    pub ast: NaryForest<XmlToken>,
}

impl XmlParser {
    pub fn new() -> XmlParser {
        XmlParser {
            lexer: XmlLexer::new(),
            ast: NaryForest::new(),
        }
    }

    ///Builds AST with an explicit stack
    pub fn parse(&mut self, src: &String) -> Result<(), XMLErrorKind> {
        use XmlTokenKind::*;

        //lex raw text first
        self.lexer.lex(src.as_str())?;
        // self.print_tokens();

        //init ast_stack with the root_node
        let root_token = self.lexer.tokens[0].take().unwrap();
        let root_node_ptr = self.ast.allocate(root_token);
        let mut ast_stack = vec![root_node_ptr];

        for k in 1..self.lexer.tokens.len() {
            if let Some(&parent_ptr) = ast_stack.last() {
                let current_token = self.lexer.tokens[k].take().unwrap();
                if let Open = current_token.token_kind {
                    let node_ptr = self.ast.allocate(current_token);
                    self.ast.add_child(parent_ptr, node_ptr);
                    ast_stack.push(node_ptr);
                } else if let Close = current_token.token_kind {
                    let open_tag_name = &self.ast[parent_ptr].data.as_ref().unwrap().content;
                    let close_tag_name = &current_token.content;
                    if open_tag_name != close_tag_name {
                        return Err(XMLErrorKind::ParserErr("Tags mismatch"));
                    }
                    if let None = ast_stack.pop() {
                        return Err(XMLErrorKind::ParserErr("Close Tag without Opening Tag"));
                    }
                } else if let Inner = current_token.token_kind {
                    let node_ptr = self.ast.allocate(current_token);
                    self.ast.add_child(parent_ptr, node_ptr);
                } else if let OpenClose = current_token.token_kind {
                    let node_ptr = self.ast.allocate(current_token);
                    self.ast.add_child(parent_ptr, node_ptr);
                }
            } else {
                return Err(XMLErrorKind::ParserErr("Extra Tag"));
            }
        }
        if ast_stack.is_empty() == false {
            return Err(XMLErrorKind::ParserErr(
                "Opening tags do not match close tags",
            ));
        }
        self.lexer.tokens.clear();
        //set root of tree
        self.ast.root_list.push(root_node_ptr);

        Ok(())
    }

    /// # Description
    /// returns the ast of the xml
    /// # Comments
    /// This function lets you drop the lexer now that its not needed
    pub fn into_ast(self) -> XmlAst {
        XmlAst { ast: self.ast }
    }
}
#[derive(Clone)]
pub struct XmlAst {
    pub ast: NaryForest<XmlToken>,
}

impl XmlAst {
    /// # Description
    /// Clones `tree` into `Self`'s memory space
    /// # Returns
    /// A pointer to the root of the copied `tree` in `Self`'s memory
    pub fn clone_tree(&mut self, tree: &XmlAst) -> Pointer {
        self.clone_tree_helper(tree, NULL, tree.ast.root_list[0])
    }

    fn clone_tree_helper(
        &mut self,
        other_tree: &XmlAst,
        other_parent: Pointer,
        other_node: Pointer,
    ) -> Pointer {
        if other_node == NULL {
            return NULL;
        }
        let cloned_other_node_ptr = self.clone_node(other_tree, other_node);
        
        if other_parent != NULL {
            self.ast.add_child(other_parent, cloned_other_node_ptr);
        }

        for &other_node_child_ptr in other_tree[other_node].children.iter() {
            self.clone_tree_helper(other_tree, cloned_other_node_ptr, other_node_child_ptr);
        }

        cloned_other_node_ptr
    }
    /// # Description
    /// Clones a `other_node` residing in `other_tree` and puts it in
    /// `Self`'s memory
    /// # Returns
    /// The address to the cloned node(cloned node is now in `Self`)
    fn clone_node(&mut self, other_tree: &XmlAst, other_node: Pointer) -> Pointer {
        let other_node = &other_tree[other_node];
        let duplicated_token = other_node.data.as_ref().expect("as_ref fucked up").clone(); 
        self.ast
            .allocate(duplicated_token)
    }

    pub fn print_tree(&self) {
        let mut char_stack = String::new();
        self.print_tree_helper(self.ast.root_list[0], &mut char_stack, ".");
    }

    fn print_tree_helper(&self, node_ptr: u32, char_stack: &mut String, c_kind: &'static str) {
        if node_ptr == !0 {
            return;
        }

        println!(
            "{}{}",
            char_stack,
            self.ast[node_ptr].data.as_ref().unwrap().content.trim()
        );

        char_stack.push_str(c_kind);

        for child_ptr in self.ast[node_ptr].children.iter() {
            self.print_tree_helper(*child_ptr, char_stack, c_kind);
        }

        (0..c_kind.len()).for_each(|_| {
            char_stack.pop();
            ()
        });
    }

    ///converts the xml AST back to text form
    pub fn to_xml(&self) -> String {
        let mut xml = String::new();
        if let Some(&root) = self.ast.root_list.get(0) {
            self.to_xml_helper(root, &mut xml);
        }
        xml
    }
    ///  The recursive helper function that renders the tree into a string (spacing is intact)\
    /// `xml_stream` the destination of the converted parse tree in text form\
    /// `note_ptr` the subtre\
    ///  I wrote this a while back but if I recall it does a depth first traversal over the tree\
    ///  converting all tokens into text form.
    fn to_xml_helper(&self, node_ptr: u32, xml_stream: &mut String) {
        if node_ptr == !0 {
            return;
        }
        match self.ast[node_ptr].data.as_ref() {
            Some(token) => match token.token_kind {
                XmlTokenKind::Open => {
                    xml_stream.push_str(format!("<{}", token.content).as_str());
                    for (key, val) in token.attribs.iter() {
                        xml_stream.push_str(format!(" {}=\"{}\"", key, val).as_str());
                    }
                    xml_stream.push('>');
                    for &child in self.ast[node_ptr].children.iter() {
                        self.to_xml_helper(child, xml_stream);
                    }
                    xml_stream.push_str(format!("</{}>", token.content).as_str());
                }
                XmlTokenKind::Inner => {
                    xml_stream.push_str(format!("{}", token.content).as_str());
                }
                XmlTokenKind::OpenClose => {
                    xml_stream.push_str(format!("<{}", token.content).as_str());
                    for (key, val) in token.attribs.iter() {
                        xml_stream.push_str(format!(" {}=\"{}\"", key, val).as_str());
                    }
                    xml_stream.push_str("/>");
                }
                _ => (),
            },
            None => (),
        }
    }

    /// Like to_xml(..) with removes all spacing
    pub fn to_xml_trim(&self) -> String {
        let mut xml = String::new();
        if let Some(&root) = self.ast.root_list.get(0) {
            self.to_xml_helper_trim(root, &mut xml);
        }
        xml
    }

    /// Pretty much a clone of to_xml_helper(...) but with formatting and trimming in the mix/
    /// Maybe write one helper function that does both?
    fn to_xml_helper_trim(&self, node_ptr: u32, xml_stream: &mut String) {
        if node_ptr == !0 {
            return;
        }
        match self.ast[node_ptr].data.as_ref() {
            Some(token) => match token.token_kind {
                XmlTokenKind::Open => {
                    xml_stream.push_str(format!("<{}", token.content).as_str().trim());
                    for (key, val) in token.attribs.iter() {
                        xml_stream.push_str(format!(" {}=\"{}\"", key.trim(), val.trim()).as_str());
                    }
                    xml_stream.push('>');
                    for &child in self.ast[node_ptr].children.iter() {
                        self.to_xml_helper_trim(child, xml_stream);
                    }
                    xml_stream.push_str(format!("</{}>", token.content.trim()).as_str());
                }
                XmlTokenKind::Inner => {
                    xml_stream.push_str(format!("{}", token.content.trim()).as_str());
                }
                XmlTokenKind::OpenClose => {
                    xml_stream.push_str(format!("<{} ", token.content.trim()).as_str());
                    for (key, val) in token.attribs.iter() {
                        xml_stream.push_str(format!(" {}=\"{}\"", key.trim(), val.trim()).as_str());
                    }
                    xml_stream.push_str("/>");
                }
                _ => (),
            },
            None => (),
        }
    }
}

impl ops::Index<Pointer> for XmlAst {
    type Output = NaryNode<XmlToken>;
    fn index(&self, index: Pointer) -> &Self::Output {
        &self.ast[index]
    }
}

impl ops::IndexMut<Pointer> for XmlAst {
    fn index_mut(&mut self, index: Pointer) -> &mut Self::Output {
        &mut self.ast[index]
    }
}
