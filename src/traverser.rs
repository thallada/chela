extern crate typed_arena;

use std::io::{self, Error, Read};

use html5ever::{serialize, Attribute, LocalName, QualName};

use crate::arena_dom::{html5ever_parse_slice_into_arena, Arena, Node, Ref};

// TODO: I don't love the "Traverser" name. Should maybe come up with something else.
// (it also unwraps nodes and calls transformer functions... does a lot more than traverse)
// TODO: What are the performance implications of using a vec of boxed closures instead of one
// transformer function who's size is known at compile time (U: Fn(Ref<'arena>) -> bool)?
// TODO: how to integrate CSS parsing and transforming?
pub struct Traverser<'arena, T>
where
    T: Fn(Ref) -> bool,
{
    arena: typed_arena::Arena<Node<'arena>>,
    should_unwrap: T,
    transformers: Vec<Box<&'arena dyn Fn(Ref<'arena>, Arena<'arena>) -> bool>>,
}

impl<'arena, T> Traverser<'arena, T>
where
    T: Fn(Ref) -> bool,
{
    pub fn new(
        should_unwrap: T,
        transformers: Vec<Box<&'arena dyn Fn(Ref<'arena>, Arena<'arena>) -> bool>>,
    ) -> Traverser<'arena, T> {
        Traverser {
            arena: typed_arena::Arena::new(),
            should_unwrap,
            transformers,
        }
    }

    pub fn parse(&'arena self, data: &mut impl Read) -> Result<Ref<'arena>, Error> {
        let mut bytes = Vec::new();
        data.read_to_end(&mut bytes)?;
        Ok(html5ever_parse_slice_into_arena(&bytes, &self.arena))
    }

    pub fn traverse(&'arena self, node: Ref<'arena>) {
        println!("{}", &node);
        if (self.should_unwrap)(node) {
            if let Some(unwrapped_node) = node.unwrap() {
                return self.traverse(unwrapped_node);
            } else {
                return;
            }
        }

        for transformer in self.transformers.iter() {
            println!("transformer result: {}", transformer(node, &self.arena));
        }

        if let Some(child) = node.first_child.get() {
            self.traverse(child);
        }

        if let Some(sibling) = node.next_sibling.get() {
            self.traverse(sibling);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::fs::File;

    struct MockRead;

    impl Read for MockRead {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
            Ok(1)
        }

        fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize, Error> {
            buf.extend_from_slice(b"<div></div>");
            Ok(1)
        }
    }

    #[test]
    fn traversal() {
        let mut traverser = Traverser::new(
            |node| false,
            vec![Box::new(&|n, _| false), Box::new(&|m, _| true)],
        );
        let mut mock_data = MockRead;
        // let mut file = File::open("src/test/div.html").unwrap();
        let root = traverser.parse(&mut mock_data).unwrap();
        traverser.traverse(root);
        assert!(false);
    }
}
