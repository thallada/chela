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
use std::io;

use html5ever::tendril::StrTendril;
use html5ever::{Attribute, LocalName, QualName};

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
mod sanitizer;

use arena_dom::{create_element, Arena, NodeData, Ref};
use config::permissive::{ADD_ATTRIBUTES, ALL_ATTRIBUTES, ATTRIBUTES, PROTOCOLS};
use config::relaxed::{CSS_AT_RULES, CSS_PROPERTIES};
use config::default::DEFAULT_CONFIG;
use css_at_rule::CssAtRule;
use css_parser::{parse_css_style_attribute, parse_css_stylesheet, CssRule};
use css_property::CssProperty;
use sanitizer::Sanitizer;

fn main() {
    let sanitizer = Sanitizer::new(
        &DEFAULT_CONFIG,
        vec![
            &sanitize_style_tag_css,
            &sanitize_style_attribute_css,
            &remove_attributes,
            &add_attributes,
            &sanitize_attribute_protocols,
            &add_single_elements_around_ul,
        ],
    );
    sanitizer.sanitize_fragment(&mut io::stdin(), &mut io::stdout()).unwrap();
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
                    if CSS_PROPERTIES.contains(&CssProperty::from(declaration.property)) {
                        sanitized_css += "  ";
                        sanitized_css += declaration_string;
                        sanitized_css += " ";
                    }
                }
                sanitized_css += "\n}";
            }
            CssRule::AtRule(at_rule) => {
                dbg!(&at_rule);
                if CSS_AT_RULES.contains(&CssAtRule::from(at_rule.name.clone())) {
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
// DONE: add whitelist of tags, remove any not in it
// DONE: add whitelist of attributes, remove any not in it
// DONE: add map of tags to attributes, remove any on tag not in the mapped value
// DONE: add whitelist of url schemes, parse urls and remove any not in it
// DONE: strip comments
// DONE: parse style tags and attributes
// DONE: add whitelist of CSS properties, remove any not in it
// TODO: scope selectors in rich formatter
// TODO: add class attributes to elements in rich formatter
// DONE: separate this out into multiple separate transformers
// TODO: find a way to avoid passing the arena to transformer functions. It's an implementation
// detail that doesn't need to be exposed. Also, it's only needed for creating new elements.
fn sanitize_style_tag_css<'arena>(node: Ref<'arena>, _: Arena<'arena>) {
    if let NodeData::Text { ref contents } = node.data {
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
}

fn sanitize_style_attribute_css<'arena>(node: Ref<'arena>, _: Arena<'arena>) {
    if let NodeData::Element { ref attrs, .. } = node.data {
        for attr in attrs.borrow_mut().iter_mut() {
            if attr.name.local == local_name!("style") {
                let css_str = &attr.value;
                let declarations = parse_css_style_attribute(css_str);
                dbg!(&declarations);
                let mut sanitized_css = String::new();
                for declaration in declarations.into_iter() {
                    let declaration_string = &declaration.to_string();
                    if CSS_PROPERTIES.contains(&CssProperty::from(declaration.property)) {
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

fn remove_attributes<'arena>(node: Ref<'arena>, _: Arena<'arena>) {
    if let NodeData::Element {
        ref attrs,
        ref name,
        ..
    } = node.data
    {
        let attrs = &mut attrs.borrow_mut();
        let mut allowed_attrs: HashSet<LocalName> = ALL_ATTRIBUTES.clone();
        if let Some(element_attrs) = ATTRIBUTES.get(&name.local) {
            allowed_attrs = allowed_attrs.union(element_attrs).cloned().collect();
        }
        let mut i = 0;

        while i != attrs.len() {
            if !allowed_attrs.contains(&attrs[i].name.local) {
                attrs.remove(i);
            }
            i += 1;
        }
    }
}

fn add_attributes<'arena>(node: Ref<'arena>, _: Arena<'arena>) {
    if let NodeData::Element {
        ref attrs,
        ref name,
        ..
    } = node.data
    {
        let attrs = &mut attrs.borrow_mut();

        if let Some(add_attributes) = ADD_ATTRIBUTES.get(&name.local) {
            for (name, &value) in add_attributes.iter() {
                attrs.push(Attribute {
                    name: QualName::new(None, ns!(), name.clone()),
                    value: StrTendril::from(value),
                });
            }
        }
    }
}

fn sanitize_attribute_protocols<'arena>(node: Ref<'arena>, _: Arena<'arena>) {
    if let NodeData::Element {
        ref attrs,
        ref name,
        ..
    } = node.data
    {
        let attrs = &mut attrs.borrow_mut();

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
                            attrs[i].value = StrTendril::from(format!("http://{}", attrs[i].value));
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
    }
}

fn add_single_elements_around_ul<'arena>(node: Ref<'arena>, arena: Arena<'arena>) {
    if let NodeData::Element { ref name, .. } = node.data {
        if let local_name!("ul") = name.local {
            node.insert_before(create_element(arena, "single"));
            node.insert_after(create_element(arena, "single"));
        }
    }
}
