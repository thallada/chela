extern crate typed_arena;

use std::io::{self, Read, Error};

use html5ever::{serialize, Attribute, LocalName, QualName};

use crate::arena_dom::{html5ever_parse_slice_into_arena, Arena, Node, Ref};

pub struct Traverser<'arena> {
    arena: typed_arena::Arena<Node<'arena>>,
}

impl<'arena> Traverser<'arena> {
    fn new() -> Traverser<'arena> {
        Traverser {
            arena: typed_arena::Arena::new(),
        }
    }

    fn traverse(&'arena self, data: &mut impl Read) -> Result<(), Error> {
        dbg!("traverse");
        let mut bytes = Vec::new();
        data.read_to_end(&mut bytes)?;
        dbg!(&bytes);
        // let node = html5ever_parse_slice_into_arena(&bytes, &self.arena);
        // dbg!(&node);
        // self.visit(node);
        Ok(())
    }

    fn visit(&'arena self, node: Ref<'arena>) {
        dbg!(&node);
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
        let mut traverser = Traverser::new();
        let mut mock_data = MockRead;
        // let mut file = File::open("src/test/div.html").unwrap();
        traverser.traverse(&mut mock_data).unwrap();
        assert!(false);
    }
}
