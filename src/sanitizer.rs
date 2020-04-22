use std::collections::{HashMap, HashSet};
use std::io::{Error, Read, Write};

use html5ever::interface::tree_builder::QuirksMode;
use html5ever::tendril::TendrilSink;
use html5ever::{parse_document, parse_fragment, serialize, LocalName, QualName};

use crate::arena_dom::{Arena, Node, NodeData, Ref, Sink};
use crate::css_at_rule::CssAtRule;
use crate::css_property::CssProperty;

pub struct Sanitizer<'arena> {
    arena: typed_arena::Arena<Node<'arena>>,
    config: &'arena SanitizerConfig,
    transformers: Vec<&'arena dyn Fn(Ref<'arena>, Arena<'arena>)>,
}

#[derive(Clone)]
pub struct SanitizerConfig {
    pub allow_comments: bool,
    pub allowed_elements: HashSet<LocalName>,
    pub allowed_attributes: HashSet<LocalName>,
    pub allowed_attributes_per_element: HashMap<LocalName, HashSet<LocalName>>,
    pub add_attributes: HashMap<LocalName, &'static str>,
    pub add_attributes_per_element: HashMap<LocalName, HashMap<LocalName, &'static str>>,
    pub allowed_protocols: HashMap<LocalName, HashMap<LocalName, HashSet<&'static str>>>,
    pub allowed_css_at_rules: HashSet<CssAtRule>,
    pub allowed_css_properties: HashSet<CssProperty>,
    pub remove_contents_when_unwrapped: HashSet<LocalName>,
}

impl<'arena> Sanitizer<'arena> {
    pub fn new(
        config: &'arena SanitizerConfig,
        transformers: Vec<&'arena dyn Fn(Ref<'arena>, Arena<'arena>)>,
    ) -> Sanitizer<'arena> {
        Sanitizer {
            arena: typed_arena::Arena::new(),
            config,
            transformers,
        }
    }

    pub fn sanitize_fragment(
        &'arena self,
        input: &mut impl Read,
        output: &mut impl Write,
    ) -> Result<(), Error> {
        let root = self.parse_fragment(input)?;
        self.traverse(root);
        serialize(output, root, Default::default())
    }

    pub fn sanitize_document(
        &'arena self,
        input: &mut impl Read,
        output: &mut impl Write,
    ) -> Result<(), Error> {
        let root = self.parse_document(input)?;
        self.traverse(root);
        serialize(output, root, Default::default())
    }

    fn parse_document(&'arena self, data: &mut impl Read) -> Result<Ref<'arena>, Error> {
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

    fn parse_fragment(&'arena self, data: &mut impl Read) -> Result<Ref<'arena>, Error> {
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

    fn traverse(&'arena self, node: Ref<'arena>) {
        println!("{}", &node);
        if self.should_unwrap_node(node) {
            let sibling = node.next_sibling.get();

            println!("unwrapping node");
            if self.should_remove_contents_when_unwrapped(node) {
                println!("detaching node");
                node.detach();
                println!("post-detach: {}", &node);
            } else if let Some(unwrapped_node) = node.unwrap() {
                println!("traversing unwrapped node");
                self.traverse(unwrapped_node);
            }

            if let Some(sibling) = sibling {
                println!("traversing sibling");
                self.traverse(sibling);
            }

            return;
        }

        println!("TRANSFORMING: {}", &node);
        for transformer in self.transformers.iter() {
            transformer(node, &self.arena);
        }

        if let Some(child) = node.first_child.get() {
            println!("traversing child");
            self.traverse(child);
        }

        if let Some(sibling) = node.next_sibling.get() {
            println!("traversing sibling");
            self.traverse(sibling);
        }
    }

    fn should_unwrap_node(&'arena self, node: Ref) -> bool {
        match node.data {
            NodeData::Document
            | NodeData::Doctype { .. }
            | NodeData::Text { .. }
            | NodeData::ProcessingInstruction { .. } => false,
            NodeData::Comment { .. } => !self.config.allow_comments,
            NodeData::Element { ref name, .. } => {
                !self.config.allowed_elements.contains(&name.local)
            }
        }
    }

    fn should_remove_contents_when_unwrapped(&'arena self, node: Ref) -> bool {
        match node.data {
            NodeData::Document
            | NodeData::Doctype { .. }
            | NodeData::Text { .. }
            | NodeData::ProcessingInstruction { .. }
            | NodeData::Comment { .. } => false,
            NodeData::Element { ref name, .. } => self
                .config
                .remove_contents_when_unwrapped
                .contains(&name.local),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::str;

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

    lazy_static! {
        static ref EMPTY_CONFIG: SanitizerConfig = SanitizerConfig {
            allow_comments: false,
            allowed_elements: HashSet::new(),
            allowed_attributes: HashSet::new(),
            allowed_attributes_per_element: HashMap::new(),
            add_attributes: HashMap::new(),
            add_attributes_per_element: HashMap::new(),
            allowed_protocols: HashMap::new(),
            allowed_css_at_rules: HashSet::new(),
            allowed_css_properties: HashSet::new(),
            remove_contents_when_unwrapped: HashSet::new(),
        };
    }

    #[test]
    fn disallow_all_elements() {
        let sanitizer = Sanitizer::new(&EMPTY_CONFIG, vec![]);
        let mut mock_data = MockRead::new("<div><!-- remove me --></div>");
        let mut output = vec![];
        sanitizer
            .sanitize_fragment(&mut mock_data, &mut output)
            .unwrap();
        assert_eq!(str::from_utf8(&output).unwrap(), "");
    }

    #[test]
    fn remove_html_comments() {
        let mut disallow_comments_config = EMPTY_CONFIG.clone();
        disallow_comments_config
            .allowed_elements
            .extend(vec![local_name!("html"), local_name!("div")]);
        let sanitizer = Sanitizer::new(&disallow_comments_config, vec![]);
        let mut mock_data = MockRead::new("<div><!-- remove me --></div>");
        let mut output = vec![];
        sanitizer
            .sanitize_fragment(&mut mock_data, &mut output)
            .unwrap();
        assert_eq!(str::from_utf8(&output).unwrap(), "<html><div></div></html>");
    }

    #[test]
    fn remove_script_elements() {
        let mut disallow_script_config = EMPTY_CONFIG.clone();
        disallow_script_config
            .allowed_elements
            .extend(vec![local_name!("html"), local_name!("div")]);
        let sanitizer = Sanitizer::new(&disallow_script_config, vec![]);
        let mut mock_data = MockRead::new("<div><script>alert('haX0rz')</script></div>");
        let mut output = vec![];
        sanitizer
            .sanitize_fragment(&mut mock_data, &mut output)
            .unwrap();
        assert_eq!(
            str::from_utf8(&output).unwrap(),
            "<html><div>alert('haX0rz')</div></html>"
        );
    }

    #[test]
    fn remove_script_element_siblings() {
        let mut disallow_script_config = EMPTY_CONFIG.clone();
        disallow_script_config
            .allowed_elements
            .extend(vec![local_name!("html"), local_name!("div")]);
        let sanitizer = Sanitizer::new(&disallow_script_config, vec![]);
        let mut mock_data =
            MockRead::new("<div><script>alert('haX0rz')</script><script>two</script></div>");
        let mut output = vec![];
        sanitizer
            .sanitize_fragment(&mut mock_data, &mut output)
            .unwrap();
        assert_eq!(
            str::from_utf8(&output).unwrap(),
            "<html><div>alert('haX0rz')two</div></html>"
        );
    }

    #[test]
    fn remove_script_element_in_separate_sub_trees() {
        let mut disallow_script_config = EMPTY_CONFIG.clone();
        disallow_script_config
            .allowed_elements
            .extend(vec![local_name!("html"), local_name!("div")]);
        let sanitizer = Sanitizer::new(&disallow_script_config, vec![]);
        let mut mock_data = MockRead::new(
            "<div><script>alert('haX0rz')</script></div><div><script>two</script></div>",
        );
        let mut output = vec![];
        sanitizer
            .sanitize_fragment(&mut mock_data, &mut output)
            .unwrap();
        assert_eq!(
            str::from_utf8(&output).unwrap(),
            "<html><div>alert('haX0rz')</div><div>two</div></html>"
        );
    }

    #[test]
    fn remove_script_elements_and_contents() {
        let mut disallow_script_config = EMPTY_CONFIG.clone();
        disallow_script_config
            .allowed_elements
            .extend(vec![local_name!("html"), local_name!("div")]);
        disallow_script_config
            .remove_contents_when_unwrapped
            .insert(local_name!("script"));
        let sanitizer = Sanitizer::new(&disallow_script_config, vec![]);
        let mut mock_data = MockRead::new("<div><script>alert('haX0rz')</script></div>");
        let mut output = vec![];
        sanitizer
            .sanitize_fragment(&mut mock_data, &mut output)
            .unwrap();
        assert_eq!(str::from_utf8(&output).unwrap(), "<html><div></div></html>");
    }

    #[test]
    fn remove_script_elements_and_content_siblings() {
        let mut disallow_script_config = EMPTY_CONFIG.clone();
        disallow_script_config
            .allowed_elements
            .extend(vec![local_name!("html"), local_name!("div")]);
        disallow_script_config
            .remove_contents_when_unwrapped
            .insert(local_name!("script"));
        let sanitizer = Sanitizer::new(&disallow_script_config, vec![]);
        let mut mock_data =
            MockRead::new("<div><script>alert('haX0rz')</script><script>two</script></div>");
        let mut output = vec![];
        sanitizer
            .sanitize_fragment(&mut mock_data, &mut output)
            .unwrap();
        assert_eq!(str::from_utf8(&output).unwrap(), "<html><div></div></html>");
    }

    #[test]
    fn remove_script_elements_and_content_in_separate_sub_trees() {
        let mut disallow_script_config = EMPTY_CONFIG.clone();
        disallow_script_config
            .allowed_elements
            .extend(vec![local_name!("html"), local_name!("div")]);
        disallow_script_config
            .remove_contents_when_unwrapped
            .insert(local_name!("script"));
        let sanitizer = Sanitizer::new(&disallow_script_config, vec![]);
        let mut mock_data = MockRead::new(
            "<div><script>alert('haX0rz')</script></div><div><script>two</script></div>",
        );
        let mut output = vec![];
        sanitizer
            .sanitize_fragment(&mut mock_data, &mut output)
            .unwrap();
        assert_eq!(str::from_utf8(&output).unwrap(), "<html><div></div><div></div></html>");
    }
}
