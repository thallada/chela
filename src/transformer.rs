use std::io::{Error, Read};

use html5ever::interface::tree_builder::QuirksMode;
use html5ever::tendril::TendrilSink;
use html5ever::{parse_document, parse_fragment, QualName};

use crate::arena_dom::{Arena, Node, NodeData, Ref, Sink};

// TODO: What are the performance implications of using a vec of closures instead of one
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

    pub fn parse_document(&'arena self, data: &mut impl Read) -> Result<Ref<'arena>, Error> {
        let mut bytes = Vec::new();
        data.read_to_end(&mut bytes)?;
        let sink = Sink {
            arena: &self.arena,
            document: self.arena.alloc(Node::new(NodeData::Document)),
            quirks_mode: QuirksMode::NoQuirks,
        };
        Ok(parse_document(sink, Default::default())
            .from_utf8()
            .one(&bytes[..]))
    }

    pub fn parse_fragment(&'arena self, data: &mut impl Read) -> Result<Ref<'arena>, Error> {
        let mut bytes = Vec::new();
        data.read_to_end(&mut bytes)?;
        let sink = Sink {
            arena: &self.arena,
            document: self.arena.alloc(Node::new(NodeData::Document)),
            quirks_mode: QuirksMode::NoQuirks,
        };
        Ok(parse_fragment(
            sink,
            Default::default(),
            QualName::new(None, ns!(html), local_name!("body")),
            vec![],
        )
        .from_utf8()
        .one(&bytes[..]))
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
        let root = transformer.parse_fragment(&mut mock_data).unwrap();
        transformer.traverse(root);
        assert_serialized_html_eq(root, "<html><div></div></html>");
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
        let root = transformer.parse_fragment(&mut mock_data).unwrap();
        transformer.traverse(root);
        assert_serialized_html_eq(root, "<html></html>");
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
        let root = transformer.parse_fragment(&mut mock_data).unwrap();
        transformer.traverse(root);
        assert_serialized_html_eq(root, "<html><div></div><span></span></html>");
    }
}
