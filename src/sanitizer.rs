use std::collections::{HashMap, HashSet};
use std::io::{Error, Read, Write};
use url::{ParseError, Url};

use html5ever::interface::tree_builder::QuirksMode;
use html5ever::tendril::{format_tendril, StrTendril, TendrilSink};
use html5ever::{
    parse_document, parse_fragment, serialize, Attribute as HTML5everAttribute, LocalName, QualName,
};

use crate::arena_dom::{Arena, Attribute, Node, NodeData, Ref, Sink, StyleAttribute};
use crate::css_at_rule::CssAtRule;
use crate::css_parser::{parse_css_style_attribute, parse_css_stylesheet, CssRule, CssStyleRule};
use crate::css_property::CssProperty;

pub struct Sanitizer<'arena> {
    arena: typed_arena::Arena<Node<'arena>>,
    config: &'arena SanitizerConfig,
    transformers: Vec<&'arena dyn Fn(Ref<'arena>, Arena<'arena>)>,
}

#[derive(Debug, Clone)]
pub struct SanitizerConfig {
    pub allow_comments: bool,
    pub allow_doctype: bool,
    pub allowed_elements: HashSet<LocalName>,
    pub allowed_attributes: HashSet<LocalName>,
    pub allowed_attributes_per_element: HashMap<LocalName, HashSet<LocalName>>,
    pub add_attributes: HashMap<LocalName, &'static str>,
    pub add_attributes_per_element: HashMap<LocalName, HashMap<LocalName, &'static str>>,
    pub allowed_protocols: HashMap<LocalName, HashMap<LocalName, HashSet<Protocol<'static>>>>,
    pub allowed_css_at_rules: HashSet<CssAtRule>,
    pub allowed_css_properties: HashSet<CssProperty>,
    pub allowed_css_protocols: HashSet<Protocol<'static>>,
    pub allow_css_comments: bool,
    pub remove_contents_when_unwrapped: HashSet<LocalName>,
    pub whitespace_around_unwrapped_content: HashMap<LocalName, ContentWhitespace<'static>>,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Protocol<'a> {
    Scheme(&'a str),
    Relative,
}

#[derive(Debug, Clone)]
pub struct ContentWhitespace<'a> {
    before: &'a str,
    after: &'a str,
}

impl<'a> ContentWhitespace<'a> {
    pub fn space_around() -> ContentWhitespace<'a> {
        ContentWhitespace {
            before: " ",
            after: " ",
        }
    }
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
        if self.should_unwrap_node(node) {
            let sibling = node.next_sibling.get();

            if self.should_remove_contents_when_unwrapped(node) {
                node.detach();
            } else if let Some(unwrapped_node) = node.unwrap() {
                self.add_unwrapped_content_whitespace(node, unwrapped_node);
                self.traverse(unwrapped_node);
            }

            if let Some(sibling) = sibling {
                self.traverse(sibling);
            }

            return;
        }

        self.remove_attributes(node);
        self.add_attributes(node);
        self.sanitize_attribute_protocols(node);
        self.sanitize_style_tag_css(node);
        self.sanitize_style_attribute_css(node);

        for transformer in self.transformers.iter() {
            transformer(node, &self.arena);
        }

        if let Some(child) = node.first_child.get() {
            self.traverse(child);
        }

        if let Some(sibling) = node.next_sibling.get() {
            self.traverse(sibling);
        }
    }

    fn should_unwrap_node(&self, node: Ref) -> bool {
        match node.data {
            NodeData::Document
            | NodeData::Text { .. }
            | NodeData::StyleSheet { .. }
            | NodeData::ProcessingInstruction { .. } => false,
            NodeData::Comment { .. } => !self.config.allow_comments,
            NodeData::Doctype { .. } => !self.config.allow_doctype,
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
            | NodeData::StyleSheet { .. }
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
                if let Attribute::Text(attr) = &attrs[i] {
                    if !all_allowed.contains(&attr.name.local) {
                        if let Some(per_element_allowed) = per_element_allowed {
                            if per_element_allowed.contains(&attr.name.local) {
                                i += 1;
                                continue;
                            }
                        }
                        attrs.remove(i);
                        continue;
                    }
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
                attrs.push(Attribute::Text(HTML5everAttribute {
                    name: QualName::new(None, ns!(), name.clone()),
                    value: StrTendril::from(value),
                }));
            }

            if let Some(add_attributes_per_element) = add_attributes_per_element {
                for (name, &value) in add_attributes_per_element.iter() {
                    attrs.push(Attribute::Text(HTML5everAttribute {
                        name: QualName::new(None, ns!(), name.clone()),
                        value: StrTendril::from(value),
                    }));
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
                let mut i = 0;
                while i != attrs.len() {
                    if let Attribute::Text(attr) = &attrs[i] {
                        if let Some(allowed_protocols) = protocols.get(&attr.name.local) {
                            match Url::parse(&attr.value) {
                                Ok(url) => {
                                    if !allowed_protocols.contains(&Protocol::Scheme(url.scheme()))
                                    {
                                        attrs.remove(i);
                                    } else {
                                        i += 1;
                                    }
                                }
                                Err(ParseError::RelativeUrlWithoutBase) => {
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
                    } else {
                        i += 1;
                    }
                }
            }
        }
    }

    fn sanitize_css_rules(&self, rules: Vec<CssRule>) -> Vec<CssRule> {
        rules
            .into_iter()
            .filter_map(|rule| match rule {
                CssRule::StyleRule(style_rule) => Some(CssRule::StyleRule(CssStyleRule {
                    selectors: style_rule.selectors,
                    declarations: style_rule
                        .declarations
                        .into_iter()
                        .filter(|declaration| {
                            self.config
                                .allowed_css_properties
                                .contains(&CssProperty::from(declaration.property.as_str()))
                        })
                        .collect(),
                })),
                CssRule::AtRule(at_rule) => {
                    if self
                        .config
                        .allowed_css_at_rules
                        .contains(&CssAtRule::from(at_rule.name.as_str()))
                    {
                        Some(CssRule::AtRule(at_rule))
                    } else {
                        None
                    }
                }
            })
            .collect()
    }

    fn sanitize_style_tag_css(&'arena self, node: Ref<'arena>) {
        if let NodeData::Element { ref name, .. } = node.data {
            if name.local == local_name!("style") {
                // TODO: is it okay to assume <style> tags will only ever have one text node child?
                if let Some(first_child) = node.first_child.take() {
                    if let NodeData::Text { ref contents, .. } = first_child.data {
                        let rules = parse_css_stylesheet(&contents.borrow());
                        let sanitized_rules = self.sanitize_css_rules(rules);
                        first_child.detach();
                        let stylesheet = self.arena.alloc(Node::new(NodeData::StyleSheet {
                            rules: sanitized_rules,
                        }));
                        node.append(stylesheet);
                    }
                }
            }
        }
    }

    fn sanitize_style_attribute_css(&self, node: Ref<'arena>) {
        if let NodeData::Element { ref attrs, .. } = node.data {
            let mut i = 0;
            let attrs = &mut attrs.borrow_mut();

            while i != attrs.len() {
                if let Attribute::Text(attr) = &attrs[i] {
                    if attr.name.local == local_name!("style") {
                        let css_str = &attr.value;
                        let mut declarations = parse_css_style_attribute(css_str);
                        declarations.retain(|declaration| {
                            self.config
                                .allowed_css_properties
                                .contains(&CssProperty::from(declaration.property.as_str()))
                        });
                        let name = attr.name.clone();
                        attrs.remove(i);
                        attrs.insert(
                            i,
                            Attribute::Style(StyleAttribute {
                                name,
                                value: declarations,
                                serialized_value: None,
                            }),
                        );
                    }
                }
                i += 1;
            }
        }
    }

    fn add_unwrapped_content_whitespace(
        &self,
        wrapping_node: Ref<'arena>,
        unwrapped_node: Ref<'arena>,
    ) {
        if let NodeData::Element { ref name, .. } = wrapping_node.data {
            if let Some(content_whitespace) = self
                .config
                .whitespace_around_unwrapped_content
                .get(&name.local)
            {
                if let NodeData::Text { ref contents, .. } = unwrapped_node.data {
                    contents.replace_with(|current| {
                        format_tendril!(
                            "{}{}{}",
                            content_whitespace.before,
                            current,
                            content_whitespace.after
                        )
                    });
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
            allow_doctype: false,
            allowed_elements: HashSet::new(),
            allowed_attributes: HashSet::new(),
            allowed_attributes_per_element: HashMap::new(),
            add_attributes: HashMap::new(),
            add_attributes_per_element: HashMap::new(),
            allowed_protocols: HashMap::new(),
            allowed_css_at_rules: HashSet::new(),
            allowed_css_properties: HashSet::new(),
            allowed_css_protocols: HashSet::new(),
            allow_css_comments: false,
            remove_contents_when_unwrapped: HashSet::new(),
            whitespace_around_unwrapped_content: HashMap::new(),
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
    fn allow_html_comments() {
        let mut allow_comments_config = EMPTY_CONFIG.clone();
        allow_comments_config.allow_comments = true;
        allow_comments_config
            .allowed_elements
            .extend(vec![local_name!("html"), local_name!("div")]);
        let sanitizer = Sanitizer::new(&allow_comments_config, vec![]);
        let mut mock_data = MockRead::new("<div><!-- keep me --></div>");
        let mut output = vec![];
        sanitizer
            .sanitize_fragment(&mut mock_data, &mut output)
            .unwrap();
        assert_eq!(
            str::from_utf8(&output).unwrap(),
            "<html><div><!-- keep me --></div></html>"
        );
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
    fn sanitize_style_attribute_css() {
        let mut sanitize_css_config = EMPTY_CONFIG.clone();
        sanitize_css_config
            .allowed_elements
            .extend(vec![local_name!("html"), local_name!("div")]);
        sanitize_css_config
            .allowed_attributes
            .extend(vec![local_name!("style")]);
        sanitize_css_config
            .allowed_css_properties
            .extend(vec![css_property!("margin"), css_property!("color")]);
        let sanitizer = Sanitizer::new(&sanitize_css_config, vec![]);
        let mut mock_data =
            MockRead::new("<div style=\"margin: 10px; padding: 10px; color: red;\"></div>");
        let mut output = vec![];
        sanitizer
            .sanitize_fragment(&mut mock_data, &mut output)
            .unwrap();
        assert_eq!(
            str::from_utf8(&output).unwrap(),
            "<html><div style=\"margin: 10px; color: red;\"></div></html>"
        );
    }

    #[test]
    fn sanitize_stylesheet_css() {
        let mut sanitize_css_config = EMPTY_CONFIG.clone();
        sanitize_css_config
            .allowed_elements
            .extend(vec![local_name!("html"), local_name!("style")]);
        sanitize_css_config
            .allowed_css_properties
            .extend(vec![css_property!("margin"), css_property!("color")]);
        let sanitizer = Sanitizer::new(&sanitize_css_config, vec![]);
        let mut mock_data =
            MockRead::new("<style>div { margin: 10px; padding: 10px; color: red; }</style>");
        let mut output = vec![];
        sanitizer
            .sanitize_fragment(&mut mock_data, &mut output)
            .unwrap();
        assert_eq!(
            str::from_utf8(&output).unwrap(),
            "<html><style>div { margin: 10px; color: red; }</style></html>"
        );
    }

    #[test]
    fn sanitize_css_protocols() {
        let mut sanitize_css_config = EMPTY_CONFIG.clone();
        sanitize_css_config
            .allowed_elements
            .extend(vec![local_name!("html"), local_name!("style")]);
        sanitize_css_config.allowed_css_properties.extend(vec![
            css_property!("background-image"),
            css_property!("content"),
        ]);
        sanitize_css_config
            .allowed_css_protocols
            .extend(vec![Protocol::Scheme("https")]);
        let sanitizer = Sanitizer::new(&sanitize_css_config, vec![]);
        let mut mock_data = MockRead::new(
            "<style>div { background-image: url(https://example.com); \
             content: url(icon.jpg); }</style>",
        );
        let mut output = vec![];
        sanitizer
            .sanitize_fragment(&mut mock_data, &mut output)
            .unwrap();
        assert_eq!(
            str::from_utf8(&output).unwrap(),
            "<html><style>div { background-image: url(https://example.com) }</style></html>"
        );
    }

    #[test]
    fn remove_doctype() {
        let mut disallow_doctype_config = EMPTY_CONFIG.clone();
        disallow_doctype_config.allow_doctype = false;
        disallow_doctype_config
            .allowed_elements
            .extend(vec![local_name!("html"), local_name!("div")]);
        let sanitizer = Sanitizer::new(&disallow_doctype_config, vec![]);
        let mut mock_data = MockRead::new("<!DOCTYPE html><div></div>");
        let mut output = vec![];
        sanitizer
            .sanitize_document(&mut mock_data, &mut output)
            .unwrap();
        assert_eq!(str::from_utf8(&output).unwrap(), "<html><div></div></html>");
    }

    #[test]
    fn allow_doctype() {
        let mut allow_doctype_config = EMPTY_CONFIG.clone();
        allow_doctype_config.allow_doctype = true;
        allow_doctype_config
            .allowed_elements
            .extend(vec![local_name!("html"), local_name!("div")]);
        let sanitizer = Sanitizer::new(&allow_doctype_config, vec![]);
        let mut mock_data = MockRead::new("<!DOCTYPE html><div></div>");
        let mut output = vec![];
        sanitizer
            .sanitize_document(&mut mock_data, &mut output)
            .unwrap();
        assert_eq!(
            str::from_utf8(&output).unwrap(),
            "<!DOCTYPE html><html><div></div></html>"
        );
    }

    #[test]
    fn add_unwrapped_content_whitespace() {
        let mut unwrapped_whitespace_config = EMPTY_CONFIG.clone();
        unwrapped_whitespace_config
            .allowed_elements
            .extend(vec![local_name!("html"), local_name!("div")]);
        unwrapped_whitespace_config
            .whitespace_around_unwrapped_content
            .insert(local_name!("span"), ContentWhitespace::space_around());
        let sanitizer = Sanitizer::new(&unwrapped_whitespace_config, vec![]);
        let mut mock_data =
            MockRead::new("<div>div-1<span>content-1</span><span>content-2</span>div-2</div>");
        let mut output = vec![];
        sanitizer
            .sanitize_fragment(&mut mock_data, &mut output)
            .unwrap();
        assert_eq!(
            str::from_utf8(&output).unwrap(),
            "<html><div>div-1 content-1  content-2 div-2</div></html>"
        );
    }
}
