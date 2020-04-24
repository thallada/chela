use std::collections::{HashMap, HashSet};
use std::io::{Error, Read, Write};
use url::{ParseError, Url};

use html5ever::interface::tree_builder::QuirksMode;
use html5ever::tendril::{StrTendril, TendrilSink};
use html5ever::{parse_document, parse_fragment, serialize, Attribute, LocalName, QualName};

use crate::arena_dom::{Arena, Node, NodeData, Ref, Sink};
use crate::css_at_rule::CssAtRule;
use crate::css_parser::{parse_css_style_attribute, parse_css_stylesheet, CssRule};
use crate::css_parser_2::parse_and_serialize;
use crate::css_property::CssProperty;

pub struct Sanitizer<'arena> {
    arena: typed_arena::Arena<Node<'arena>>,
    config: &'arena SanitizerConfig,
    transformers: Vec<&'arena dyn Fn(Ref<'arena>, Arena<'arena>)>,
}

#[derive(Debug, Clone)]
pub struct SanitizerConfig {
    pub allow_comments: bool,
    pub allowed_elements: HashSet<LocalName>,
    pub allowed_attributes: HashSet<LocalName>,
    pub allowed_attributes_per_element: HashMap<LocalName, HashSet<LocalName>>,
    pub add_attributes: HashMap<LocalName, &'static str>,
    pub add_attributes_per_element: HashMap<LocalName, HashMap<LocalName, &'static str>>,
    pub allowed_protocols: HashMap<LocalName, HashMap<LocalName, HashSet<Protocol<'static>>>>,
    pub allowed_css_at_rules: HashSet<CssAtRule>,
    pub allowed_css_properties: HashSet<CssProperty>,
    pub remove_contents_when_unwrapped: HashSet<LocalName>,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Protocol<'a> {
    Scheme(&'a str),
    Relative,
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
            QualName::new(None, ns!(), local_name!("body")),
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
        self.remove_attributes(node);
        self.add_attributes(node);
        self.sanitize_attribute_protocols(node);
        self.sanitize_style_tag_css(node);
        // self.sanitize_style_attribute_css(node);
        // self.serialize_css_test(node);

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

    fn should_unwrap_node(&self, node: Ref) -> bool {
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

    fn should_remove_contents_when_unwrapped(&self, node: Ref) -> bool {
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

    fn remove_attributes(&self, node: Ref<'arena>) {
        if let NodeData::Element {
            ref attrs,
            ref name,
            ..
        } = node.data
        {
            let attrs = &mut attrs.borrow_mut();
            let mut i = 0;
            let all_allowed = &self.config.allowed_attributes;
            let per_element_allowed = self.config.allowed_attributes_per_element.get(&name.local);
            while i != attrs.len() {
                if !all_allowed.contains(&attrs[i].name.local) {
                    if let Some(per_element_allowed) = per_element_allowed {
                        if per_element_allowed.contains(&attrs[i].name.local) {
                            i += 1;
                            continue;
                        }
                    }
                    attrs.remove(i);
                    continue;
                }
                i += 1;
            }
        }
    }

    fn add_attributes(&self, node: Ref<'arena>) {
        if let NodeData::Element {
            ref attrs,
            ref name,
            ..
        } = node.data
        {
            let attrs = &mut attrs.borrow_mut();
            let add_attributes = &self.config.add_attributes;
            let add_attributes_per_element =
                self.config.add_attributes_per_element.get(&name.local);

            for (name, &value) in add_attributes.iter() {
                attrs.push(Attribute {
                    name: QualName::new(None, ns!(), name.clone()),
                    value: StrTendril::from(value),
                });
            }

            if let Some(add_attributes_per_element) = add_attributes_per_element {
                for (name, &value) in add_attributes_per_element.iter() {
                    attrs.push(Attribute {
                        name: QualName::new(None, ns!(), name.clone()),
                        value: StrTendril::from(value),
                    });
                }
            }
        }
    }

    fn sanitize_attribute_protocols(&self, node: Ref<'arena>) {
        if let NodeData::Element {
            ref attrs,
            ref name,
            ..
        } = node.data
        {
            let attrs = &mut attrs.borrow_mut();

            if let Some(protocols) = self.config.allowed_protocols.get(&name.local) {
                dbg!(protocols);
                dbg!(&attrs);
                let mut i = 0;
                while i != attrs.len() {
                    dbg!(&attrs[i].name.local);
                    if let Some(allowed_protocols) = protocols.get(&attrs[i].name.local) {
                        dbg!(allowed_protocols);
                        match Url::parse(&attrs[i].value) {
                            Ok(url) => {
                                dbg!(Protocol::Scheme(url.scheme()));
                                if !allowed_protocols.contains(&Protocol::Scheme(url.scheme())) {
                                    attrs.remove(i);
                                } else {
                                    i += 1;
                                }
                            }
                            Err(ParseError::RelativeUrlWithoutBase) => {
                                dbg!("relative");
                                if !allowed_protocols.contains(&Protocol::Relative) {
                                    attrs.remove(i);
                                } else {
                                    i += 1;
                                }
                            }
                            Err(_) => {
                                attrs.remove(i);
                            }
                        }
                    } else {
                        i += 1;
                    }
                }
            }
        }
    }

    fn serialize_sanitized_css_rules(&self, rules: Vec<CssRule>) -> String {
        let mut sanitized_css = String::new();
        for rule in rules {
            match rule {
                CssRule::StyleRule(style_rule) => {
                    sanitized_css += &style_rule.selectors;
                    sanitized_css += "{";
                    for declaration in style_rule.declarations.into_iter() {
                        let declaration_string = &declaration.to_string();
                        if self
                            .config
                            .allowed_css_properties
                            .contains(&CssProperty::from(declaration.property))
                        {
                            sanitized_css += declaration_string;
                        }
                    }
                    sanitized_css += "}";
                }
                CssRule::AtRule(at_rule) => {
                    dbg!(&at_rule);
                    if self
                        .config
                        .allowed_css_at_rules
                        .contains(&CssAtRule::from(at_rule.name.clone()))
                    {
                        sanitized_css += &format!("@{}", &at_rule.name);
                        sanitized_css += &at_rule.prelude;
                        if let Some(block) = at_rule.block {
                            sanitized_css += "{";
                            sanitized_css += &self.serialize_sanitized_css_rules(block);
                            sanitized_css += "}";
                        }
                    }
                }
            }
        }
        sanitized_css
    }

    fn sanitize_style_tag_css(&self, node: Ref<'arena>) {
        if let NodeData::Text { ref contents } = node.data {
            // TODO: seems rather expensive to lookup the parent on every Text node. Better
            // solution would be to pass some sort of context from the parent that marks that this
            // Text node is inside a <style>.
            if let Some(parent) = node.parent.get() {
                if let NodeData::Element { ref name, .. } = parent.data {
                    if name.local == local_name!("style") {
                        let rules = parse_css_stylesheet(&contents.borrow());
                        dbg!(&rules);
                        let sanitized_css = self.serialize_sanitized_css_rules(rules);
                        dbg!(&sanitized_css);
                        contents.replace(StrTendril::from(sanitized_css));
                    }
                }
            }
        }
    }

    fn sanitize_style_attribute_css(&self, node: Ref<'arena>) {
        if let NodeData::Element { ref attrs, .. } = node.data {
            for attr in attrs.borrow_mut().iter_mut() {
                if attr.name.local == local_name!("style") {
                    let css_str = &attr.value;
                    let declarations = parse_css_style_attribute(css_str);
                    dbg!(&declarations);
                    let mut sanitized_css = String::new();
                    for declaration in declarations.into_iter() {
                        let declaration_string = &declaration.to_string();
                        if self
                            .config
                            .allowed_css_properties
                            .contains(&CssProperty::from(declaration.property))
                        {
                            sanitized_css += declaration_string;
                            sanitized_css += " ";
                        }
                    }
                    let sanitized_css = sanitized_css.trim();
                    dbg!(&sanitized_css);
                    attr.value = StrTendril::from(sanitized_css);
                }
            }
        }
    }

    fn serialize_css_test(&self, node: Ref<'arena>) {
        if let NodeData::Text { ref contents } = node.data {
            if let Some(parent) = node.parent.get() {
                if let NodeData::Element { ref name, .. } = parent.data {
                    if name.local == local_name!("style") {
                        let mut serialized_css = String::new();
                        parse_and_serialize(contents.borrow(), &mut serialized_css, true);
                    }
                }
            }
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
        assert_eq!(
            str::from_utf8(&output).unwrap(),
            "<html><div></div><div></div></html>"
        );
    }

    #[test]
    fn remove_attributes() {
        let mut remove_attributes_config = EMPTY_CONFIG.clone();
        remove_attributes_config.allowed_elements.extend(vec![
            local_name!("html"),
            local_name!("a"),
            local_name!("img"),
            local_name!("span"),
        ]);
        remove_attributes_config
            .allowed_attributes
            .extend(vec![local_name!("href"), local_name!("src")]);
        let sanitizer = Sanitizer::new(&remove_attributes_config, vec![]);
        let mut mock_data = MockRead::new(
            "<a href=\"url\"></a>\
                <img src=\"url\" bad=\"1\" />\
                <span bad=\"2\" foo=\"bar\"></span>",
        );
        let mut output = vec![];
        sanitizer
            .sanitize_fragment(&mut mock_data, &mut output)
            .unwrap();
        assert_eq!(
            str::from_utf8(&output).unwrap(),
            "<html><a href=\"url\"></a>\
                <img src=\"url\"></img>\
                <span></span></html>"
        );
    }

    #[test]
    fn remove_attributes_per_element() {
        let mut remove_attributes_config = EMPTY_CONFIG.clone();
        remove_attributes_config.allowed_elements.extend(vec![
            local_name!("html"),
            local_name!("a"),
            local_name!("img"),
            local_name!("span"),
        ]);
        remove_attributes_config
            .allowed_attributes_per_element
            .insert(local_name!("a"), hashset! { local_name!("href") });
        remove_attributes_config
            .allowed_attributes_per_element
            .insert(local_name!("img"), hashset! { local_name!("src") });
        let sanitizer = Sanitizer::new(&remove_attributes_config, vec![]);
        let mut mock_data = MockRead::new(
            "<a href=\"url\" src=\"url\" bad=\"1\"></a>\
                <img src=\"url\" href=\"url\" />\
                <span href=\"url\" src=\"url\"></span>",
        );
        let mut output = vec![];
        sanitizer
            .sanitize_fragment(&mut mock_data, &mut output)
            .unwrap();
        assert_eq!(
            str::from_utf8(&output).unwrap(),
            "<html><a href=\"url\"></a>\
                <img src=\"url\"></img>\
                <span></span></html>"
        );
    }

    #[test]
    fn add_attributes() {
        let mut add_attributes_config = EMPTY_CONFIG.clone();
        add_attributes_config
            .allowed_elements
            .extend(vec![local_name!("html"), local_name!("div")]);
        add_attributes_config
            .add_attributes
            .insert(LocalName::from("foo"), "bar");
        let sanitizer = Sanitizer::new(&add_attributes_config, vec![]);
        let mut mock_data = MockRead::new("<div></div>");
        let mut output = vec![];
        sanitizer
            .sanitize_fragment(&mut mock_data, &mut output)
            .unwrap();
        assert_eq!(
            str::from_utf8(&output).unwrap(),
            "<html foo=\"bar\"><div foo=\"bar\"></div></html>"
        );
    }

    #[test]
    fn add_attributes_per_element() {
        let mut add_attributes_config = EMPTY_CONFIG.clone();
        add_attributes_config.allowed_elements.extend(vec![
            local_name!("html"),
            local_name!("a"),
            local_name!("img"),
        ]);
        add_attributes_config.add_attributes_per_element.insert(
            local_name!("a"),
            hashmap! { LocalName::from("href") => "url1" },
        );
        add_attributes_config.add_attributes_per_element.insert(
            local_name!("img"),
            hashmap! { LocalName::from("src") => "url2" },
        );
        let sanitizer = Sanitizer::new(&add_attributes_config, vec![]);
        let mut mock_data = MockRead::new("<a><img /></a>");
        let mut output = vec![];
        sanitizer
            .sanitize_fragment(&mut mock_data, &mut output)
            .unwrap();
        assert_eq!(
            str::from_utf8(&output).unwrap(),
            "<html><a href=\"url1\"><img src=\"url2\"></img></a></html>"
        );
    }

    #[test]
    fn sanitize_attribute_protocols() {
        let mut sanitize_protocols_config = EMPTY_CONFIG.clone();
        sanitize_protocols_config.allowed_elements.extend(vec![
            local_name!("html"),
            local_name!("a"),
            local_name!("img"),
        ]);
        sanitize_protocols_config
            .allowed_attributes
            .extend(vec![local_name!("href"), local_name!("src")]);
        sanitize_protocols_config.allowed_protocols.insert(
            local_name!("a"),
            hashmap! {
                LocalName::from("href") => hashset! {
                    Protocol::Scheme("https"),
                },
            },
        );
        sanitize_protocols_config.allowed_protocols.insert(
            local_name!("img"),
            hashmap! {
                LocalName::from("src") => hashset! {
                    Protocol::Scheme("http"),
                    Protocol::Scheme("https"),
                    Protocol::Relative,
                },
            },
        );
        let sanitizer = Sanitizer::new(&sanitize_protocols_config, vec![]);
        let mut mock_data = MockRead::new(
            "<a href=\"/relative\"></a>\
            <a href=\"https://example.com\"></a>\
            <a href=\"http://example.com\"></a>\
            <img src=\"/relative\" />\
            <img src=\"https://example.com\" />\
            <img src=\"http://example.com\" />",
        );
        let mut output = vec![];
        sanitizer
            .sanitize_fragment(&mut mock_data, &mut output)
            .unwrap();
        assert_eq!(
            str::from_utf8(&output).unwrap(),
            "<html><a></a>\
            <a href=\"https://example.com\"></a>\
            <a></a>\
            <img src=\"/relative\"></img>\
            <img src=\"https://example.com\"></img>\
            <img src=\"http://example.com\"></img></html>"
        );
    }

    #[test]
    fn sanitize_style_tag_css() {
        let mut sanitize_css_config = EMPTY_CONFIG.clone();
        sanitize_css_config
            .allowed_elements
            .extend(vec![local_name!("html"), local_name!("style")]);
        sanitize_css_config
            .allowed_css_properties
            .extend(vec![css_property!("margin"), css_property!("color")]);
        sanitize_css_config
            .allowed_css_at_rules
            .extend(vec![css_at_rule!("charset")]);
        let sanitizer = Sanitizer::new(&sanitize_css_config, vec![]);
        let mut mock_data = MockRead::new(
            "<style>@charset \"UTF-8\";\
            div { margin: 10px; padding: 10px; color: red; }\
            @media print { div { margin: 50px; } }</style>",
        );
        let mut output = vec![];
        sanitizer
            .sanitize_fragment(&mut mock_data, &mut output)
            .unwrap();
        assert_eq!(
            str::from_utf8(&output).unwrap(),
            "<html><style>@charset \"UTF-8\";\
            div { margin: 10px; color: red; }</style></html>"
        );
    }
}
