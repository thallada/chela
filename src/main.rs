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

use std::io;

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
use config::basic::BASIC_CONFIG;
use sanitizer::Sanitizer;

fn main() {
    let sanitizer = Sanitizer::new(&BASIC_CONFIG, vec![&add_spacer_elements_around_ul]);
    sanitizer
        .sanitize_fragment(&mut io::stdin(), &mut io::stdout())
        .unwrap();
}

// DONE: add whitelist of tags, remove any not in it
// DONE: add whitelist of attributes, remove any not in it
// DONE: add map of tags to attributes, remove any on tag not in the mapped value
// DONE: add whitelist of url schemes, parse urls and remove any not in it
// DONE: strip comments
// DONE: parse style tags and attributes
// DONE: add whitelist of CSS properties, remove any not in it
// DONE: separate this out into multiple separate transformers
// TODO: find a way to avoid passing the arena to transformer functions. It's an implementation
// detail that doesn't need to be exposed. Also, it's only needed for creating new elements.
fn add_spacer_elements_around_ul<'arena>(node: Ref<'arena>, arena: Arena<'arena>) {
    if let NodeData::Element { ref name, .. } = node.data {
        if let local_name!("ul") = name.local {
            node.insert_before(create_element(arena, "spacer"));
            node.insert_after(create_element(arena, "spacer"));
        }
    }
}
