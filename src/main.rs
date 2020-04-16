#![warn(clippy::all)]
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate html5ever;
#[macro_use]
extern crate maplit;
#[macro_use]
extern crate cssparser;
extern crate string_cache;
extern crate typed_arena;

use std::collections::HashSet;
use std::default::Default;
use std::io::{self, Read};

use html5ever::tendril::StrTendril;
use html5ever::{serialize, Attribute, LocalName, QualName};

use url::{ParseError, Url};

#[macro_use]
mod css_property {
    include!(concat!(env!("OUT_DIR"), "/css_property.rs"));
}
#[macro_use]
mod css_at_rule {
    include!(concat!(env!("OUT_DIR"), "/css_at_rule.rs"));
}

mod arena_dom;
mod config;
mod css_parser;
mod traverser;

use arena_dom::{create_element, html5ever_parse_slice_into_arena, Arena, NodeData, Ref};
use config::permissive::{ADD_ATTRIBUTES, ALL_ATTRIBUTES, ATTRIBUTES, ELEMENTS, PROTOCOLS};
use config::relaxed::{CSS_PROPERTIES, CSS_AT_RULES};
use css_parser::{CssRule, parse_css_style_attribute, parse_css_stylesheet};
use css_property::CssProperty;
use css_at_rule::CssAtRule;

fn main() {
    let mut bytes = Vec::new();
    io::stdin().read_to_end(&mut bytes).unwrap();
    let arena = typed_arena::Arena::new();
    let doc = html5ever_parse_slice_into_arena(&bytes, &arena);
    sanitize(doc, &arena);
    serialize(&mut io::stdout(), doc, Default::default())
        .ok()
        .expect("serialization failed")
}

fn sanitize<'arena>(node: Ref<'arena>, arena: Arena<'arena>) {
    if let Some(unwrapped) = maybe_unwrap_node(&node) {
        if let Some(unwrapped_node) = unwrapped {
            return sanitize(unwrapped_node, arena);
        } else {
            return;
        }
    }

    transform_node(&node, arena);

    if let Some(child) = node.first_child.get() {
        sanitize(child, arena);
    }

    if let Some(sibling) = node.next_sibling.get() {
        sanitize(sibling, arena);
    }
}

fn css_rules_to_string(rules: Vec<CssRule>) -> String {
    let mut sanitized_css = String::new();
    for rule in rules {
        match rule {
            CssRule::StyleRule(style_rule) => {
                sanitized_css += &style_rule.selectors.trim();
                sanitized_css += " {\n";
                for declaration in style_rule.declarations.into_iter() {
                    let declaration_string = &declaration.to_string();
                    if CSS_PROPERTIES
                        .contains(&CssProperty::from(declaration.property))
                    {
                        sanitized_css += "  ";
                        sanitized_css += declaration_string;
                        sanitized_css += " ";
                    }
                }
                sanitized_css += "\n}";
            },
            CssRule::AtRule(at_rule) => {
                dbg!(&at_rule);
                if CSS_AT_RULES
                    .contains(&CssAtRule::from(at_rule.name.clone()))
                {
                    sanitized_css += &format!("@{} ", &at_rule.name);
                    sanitized_css += &at_rule.prelude.trim();
                    if let Some(block) = at_rule.block {
                        sanitized_css += " {\n";
                        sanitized_css += &css_rules_to_string(block);
                        sanitized_css += "\n}";
                    }
                }
            }
        }
        sanitized_css += "\n";
    }
    sanitized_css.trim().to_string()
}

// TODO: make separate rich and plain transformers
// TODO: add whitelist of tags, remove any not in it DONE
// TODO: add whitelist of attributes, remove any not in it DONE
// TODO: add map of tags to attributes, remove any on tag not in the mapped value DONE
// TODO: add whitelist of url schemes, parse urls and remove any not in it DONE
// TODO: strip comments DONE
// TODO: parse style tags and attributes DONE
// TODO: add whitelist of CSS properties, remove any not in it DONE
// TODO: scope selectors in rich formatter
// TODO: add class attributes to elements in rich formatter
fn transform_node<'arena>(node: Ref<'arena>, arena: Arena<'arena>) {
    match node.data {
        NodeData::Document
        | NodeData::Doctype { .. }
        | NodeData::Comment { .. }
        | NodeData::ProcessingInstruction { .. } => {}
        NodeData::Text { ref contents } => {
            // TODO: seems rather expensive to lookup the parent on every Text node. Better
            // solution would be to pass some sort of context from the parent that marks that this
            // Text node is inside a <style>.
            if let Some(parent) = node.parent.get() {
                if let NodeData::Element { ref name, .. } = parent.data {
                    if name.local == local_name!("style") {
                        let rules = parse_css_stylesheet(&contents.borrow());
                        dbg!(&rules);
                        let sanitized_css = css_rules_to_string(rules);
                        dbg!(&sanitized_css);
                        contents.replace(StrTendril::from(sanitized_css));
                    }
                }
            }
        }
        NodeData::Element {
            ref attrs,
            ref name,
            ..
        } => {
            let ref mut attrs = attrs.borrow_mut();

            let mut allowed_attrs: HashSet<LocalName> = ALL_ATTRIBUTES.clone();
            if let Some(element_attrs) = ATTRIBUTES.get(&name.local) {
                allowed_attrs = allowed_attrs.union(element_attrs).cloned().collect();
            }
            let mut i = 0;
            while i != attrs.len() {
                if !allowed_attrs.contains(&attrs[i].name.local) {
                    attrs.remove(i);
                } else {
                    if attrs[i].name.local == local_name!("style") {
                        let css_str = &attrs[i].value;
                        let declarations = parse_css_style_attribute(css_str);
                        dbg!(&declarations);
                        let mut sanitized_css = String::new();
                        for declaration in declarations.into_iter() {
                            let declaration_string = &declaration.to_string();
                            if CSS_PROPERTIES
                                .contains(&CssProperty::from(declaration.property))
                            {
                                sanitized_css += declaration_string;
                                sanitized_css += " ";
                            }
                        }
                        let sanitized_css = sanitized_css.trim();
                        dbg!(&sanitized_css);
                        attrs[i].value = StrTendril::from(sanitized_css);
                    }
                    i += 1;
                }
            }

            if let Some(add_attributes) = ADD_ATTRIBUTES.get(&name.local) {
                for (name, &value) in add_attributes.iter() {
                    attrs.push(Attribute {
                        name: QualName::new(None, ns!(), name.clone()),
                        value: StrTendril::from(value),
                    });
                }
            }

            if let Some(protocols) = PROTOCOLS.get(&name.local) {
                let mut i = 0;
                while i != attrs.len() {
                    if let Some(allowed_protocols) = protocols.get(&attrs[i].name.local) {
                        match Url::parse(&attrs[i].value) {
                            Ok(url) => {
                                if !allowed_protocols.contains(url.scheme()) {
                                    attrs.remove(i);
                                } else {
                                    i += 1;
                                }
                            }
                            Err(ParseError::RelativeUrlWithoutBase) => {
                                attrs[i].value =
                                    StrTendril::from(format!("http://{}", attrs[i].value));
                                i += 1;
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

            match name.local {
                local_name!("ul") => {
                    node.insert_before(create_element(
                        arena,
                        QualName::new(None, ns!(), LocalName::from("single")),
                    ));
                    node.insert_after(create_element(
                        arena,
                        QualName::new(None, ns!(), LocalName::from("single")),
                    ));
                }
                _ => {}
            }
        }
    }
}

fn maybe_unwrap_node(node: Ref) -> Option<Option<Ref>> {
    match node.data {
        NodeData::Document
        | NodeData::Doctype { .. }
        | NodeData::Text { .. }
        | NodeData::ProcessingInstruction { .. } => None,
        NodeData::Comment { .. } => Some(node.unwrap()),
        NodeData::Element { ref name, .. } => {
            if !ELEMENTS.contains(&name.local) {
                Some(node.unwrap())
            } else {
                None
            }
        }
    }
}
