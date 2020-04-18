extern crate typed_arena;

use std::io::{Error, Read};

use crate::arena_dom::{html5ever_parse_slice_into_arena, Arena, Node, Ref};

// TODO: What are the performance implications of using a vec of boxed closures instead of one
// transformer function who's size is known at compile time (U: Fn(Ref<'arena>) -> bool)?
// TODO: how to integrate CSS parsing and transforming?
pub struct Transformer<'arena, T>
where
    T: Fn(Ref) -> bool,
{
    arena: typed_arena::Arena<Node<'arena>>,
    should_unwrap: T,
    transformer_fns: Vec<&'arena dyn Fn(Ref<'arena>, Arena<'arena>)>,
}

impl<'arena, T> Transformer<'arena, T>
where
    T: Fn(Ref) -> bool,
{
    pub fn new(
        should_unwrap: T,
        transformers: Vec<&'arena dyn Fn(Ref<'arena>, Arena<'arena>)>,
    ) -> Transformer<'arena, T> {
        Transformer {
            arena: typed_arena::Arena::new(),
            should_unwrap,
            transformer_fns: transformers,
        }
    }

    pub fn parse(&'arena self, data: &mut impl Read) -> Result<Ref<'arena>, Error> {
        let mut bytes = Vec::new();
        data.read_to_end(&mut bytes)?;
        Ok(html5ever_parse_slice_into_arena(&bytes, &self.arena))
    }

    pub fn traverse(&'arena self, node: Ref<'arena>) {
        if (self.should_unwrap)(node) {
            if let Some(unwrapped_node) = node.unwrap() {
                return self.traverse(unwrapped_node);
            } else {
                return;
            }
        }

        for transformer in self.transformer_fns.iter() {
            transformer(node, &self.arena);
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

    use std::str;

    use html5ever::serialize;

    use crate::arena_dom::{create_element, NodeData};

    struct MockRead {
        contents: &'static str,
    }

    impl MockRead {
        fn new(contents: &'static str) -> MockRead {
            MockRead { contents }
        }
    }

    impl Read for MockRead {
        fn read(&mut self, _: &mut [u8]) -> Result<usize, Error> {
            Ok(1)
        }

        fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize, Error> {
            buf.extend_from_slice(self.contents.as_bytes());
            Ok(1)
        }
    }

    // fn node_contains_tag<'arena>(node: Ref<'arena>, tag_name: &str) -> bool {
        // if let NodeData::Element { ref name, .. } = node.data {
            // if name.local == LocalName::from(tag_name) {
                // return true;
            // }
        // }

        // if let Some(child) = node.first_child.get() {
            // if node_contains_tag(child, tag_name) {
                // return true;
            // }
        // }

        // if let Some(sibling) = node.next_sibling.get() {
            // if node_contains_tag(sibling, tag_name) {
                // return true;
            // }
        // }

        // false
    // }

    // fn count_nodes(node: Ref) -> usize {
        // let mut count = 1;

        // if let Some(child) = node.first_child.get() {
            // count += count_nodes(child);
        // }

        // if let Some(sibling) = node.next_sibling.get() {
            // count += count_nodes(sibling);
        // }

        // count
    // }

    fn assert_serialized_html_eq(node: Ref, expected: &str) {
        let mut output = vec![];
        serialize(&mut output, node, Default::default()).unwrap();
        assert_eq!(str::from_utf8(&output).unwrap(), expected);
    }

    #[test]
    fn traversal() {
        let transformer = Transformer::new(|_| false, vec![&|_, _| {}]);
        let mut mock_data = MockRead::new("<div></div>");
        let root = transformer.parse(&mut mock_data).unwrap();
        transformer.traverse(root);
        assert_serialized_html_eq(root, "<html><head></head><body><div></div></body></html>");
    }

    #[test]
    fn unwraps_element() {
        let transformer = Transformer::new(
            |node| {
                if let NodeData::Element { ref name, .. } = node.data {
                    return name.local == local_name!("div");
                }
                false
            },
            vec![&|_, _| {}],
        );
        let mut mock_data = MockRead::new("<div></div>");
        let root = transformer.parse(&mut mock_data).unwrap();
        transformer.traverse(root);
        assert_serialized_html_eq(root, "<html><head></head><body></body></html>");
    }

    #[test]
    fn adds_element() {
        let transformer = Transformer::new(
            |_| false,
            vec![&|node, arena| {
                if let NodeData::Element { ref name, .. } = node.data {
                    if let local_name!("div") = name.local {
                        node.insert_after(create_element(arena, "span"));
                    }
                }
            }],
        );
        let mut mock_data = MockRead::new("<div></div>");
        let root = transformer.parse(&mut mock_data).unwrap();
        transformer.traverse(root);
        assert_serialized_html_eq(
            root,
            "<html><head></head><body><div></div><span></span></body></html>",
        );
    }
}
