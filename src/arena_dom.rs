// Majority of this file is from the html5ever project.
// https://github.com/servo/html5ever/blob/45b2fca5c6/html5ever/examples/arena.rs
//
// Copyright 2014-2017 The html5ever Project Developers. See the
// COPYRIGHT file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

extern crate html5ever;
extern crate typed_arena;

use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::default::Default;
use std::io;
use std::ptr;

use html5ever::interface::tree_builder::{ElementFlags, NodeOrText, QuirksMode, TreeSink};
use html5ever::serialize::TraversalScope::{ChildrenOnly, IncludeNode};
use html5ever::serialize::{Serialize, Serializer, TraversalScope};
use html5ever::tendril::{StrTendril, TendrilSink};
use html5ever::{parse_document, Attribute, ExpandedName, QualName};

pub fn html5ever_parse_slice_into_arena<'a>(bytes: &[u8], arena: Arena<'a>) -> Ref<'a> {
    let sink = Sink {
        arena: arena,
        document: arena.alloc(Node::new(NodeData::Document)),
        quirks_mode: QuirksMode::NoQuirks,
    };
    parse_document(sink, Default::default())
        .from_utf8()
        .one(bytes)
}

pub fn create_element<'arena>(arena: Arena<'arena>, name: QualName) -> Ref<'arena> {
    arena.alloc(Node::new(NodeData::Element {
        name: name,
        attrs: RefCell::new(vec![]),
        template_contents: None,
        mathml_annotation_xml_integration_point: false,
    }))
}

pub type Arena<'arena> = &'arena typed_arena::Arena<Node<'arena>>;

pub type Ref<'arena> = &'arena Node<'arena>;

pub type Link<'arena> = Cell<Option<Ref<'arena>>>;

pub struct Sink<'arena> {
    arena: Arena<'arena>,
    document: Ref<'arena>,
    quirks_mode: QuirksMode,
}

pub struct Node<'arena> {
    pub parent: Link<'arena>,
    pub next_sibling: Link<'arena>,
    pub previous_sibling: Link<'arena>,
    pub first_child: Link<'arena>,
    pub last_child: Link<'arena>,
    pub data: NodeData<'arena>,
}

pub enum NodeData<'arena> {
    Document,
    Doctype {
        name: StrTendril,
        public_id: StrTendril,
        system_id: StrTendril,
    },
    Text {
        contents: RefCell<StrTendril>,
    },
    Comment {
        contents: StrTendril,
    },
    Element {
        name: QualName,
        attrs: RefCell<Vec<Attribute>>,
        template_contents: Option<Ref<'arena>>,
        mathml_annotation_xml_integration_point: bool,
    },
    ProcessingInstruction {
        target: StrTendril,
        contents: StrTendril,
    },
}

impl<'arena> Node<'arena> {
    pub fn new(data: NodeData<'arena>) -> Self {
        Node {
            parent: Cell::new(None),
            previous_sibling: Cell::new(None),
            next_sibling: Cell::new(None),
            first_child: Cell::new(None),
            last_child: Cell::new(None),
            data: data,
        }
    }

    pub fn detach(&self) {
        let parent = self.parent.take();
        let previous_sibling = self.previous_sibling.take();
        let next_sibling = self.next_sibling.take();

        if let Some(next_sibling) = next_sibling {
            next_sibling.previous_sibling.set(previous_sibling);
        } else if let Some(parent) = parent {
            parent.last_child.set(previous_sibling);
        }

        if let Some(previous_sibling) = previous_sibling {
            previous_sibling.next_sibling.set(next_sibling);
        } else if let Some(parent) = parent {
            parent.first_child.set(next_sibling);
        }
    }

    pub fn unwrap(&self) -> Option<&'arena Self> {
        let parent = self.parent.take();
        let previous_sibling = self.previous_sibling.take();
        let next_sibling = self.next_sibling.take();
        let first_child = self.first_child.take();
        let last_child = self.last_child.take();

        if let Some(next_sibling) = next_sibling {
            if let Some(last_child) = last_child {
                next_sibling.previous_sibling.set(Some(last_child));
            } else {
                next_sibling.previous_sibling.set(previous_sibling);
            }
        } else if let Some(parent) = parent {
            parent.last_child.set(previous_sibling);
            if let Some(last_child) = last_child {
                parent.last_child.set(Some(last_child));
            } else {
                parent.last_child.set(previous_sibling);
            }
        }

        if let Some(previous_sibling) = previous_sibling {
            if let Some(first_child) = first_child {
                previous_sibling.next_sibling.set(Some(first_child));
            } else {
                previous_sibling.next_sibling.set(next_sibling);
            }
        } else if let Some(parent) = parent {
            parent.first_child.set(next_sibling);
            if let Some(first_child) = first_child {
                parent.first_child.set(Some(first_child));
            } else {
                parent.first_child.set(next_sibling);
            }
        }

        let mut child = first_child;
        loop {
            match child {
                Some(next_child) => {
                    next_child.parent.set(parent);
                    child = next_child.next_sibling.get();
                },
                None => break,
            }
        }

        if let Some(first_child) = first_child {
            Some(first_child)
        } else if let Some(next_sibling) = next_sibling {
            Some(next_sibling)
        } else {
            None
        }
    }

    pub fn append(&'arena self, new_child: &'arena Self) {
        new_child.detach();
        new_child.parent.set(Some(self));
        if let Some(last_child) = self.last_child.take() {
            new_child.previous_sibling.set(Some(last_child));
            debug_assert!(last_child.next_sibling.get().is_none());
            last_child.next_sibling.set(Some(new_child));
        } else {
            debug_assert!(self.first_child.get().is_none());
            self.first_child.set(Some(new_child));
        }
        self.last_child.set(Some(new_child));
    }

    pub fn insert_before(&'arena self, new_sibling: &'arena Self) {
        new_sibling.detach();
        new_sibling.parent.set(self.parent.get());
        new_sibling.next_sibling.set(Some(self));
        if let Some(previous_sibling) = self.previous_sibling.take() {
            new_sibling.previous_sibling.set(Some(previous_sibling));
            debug_assert!(ptr::eq::<Node>(
                previous_sibling.next_sibling.get().unwrap(),
                self
            ));
            previous_sibling.next_sibling.set(Some(new_sibling));
        } else if let Some(parent) = self.parent.get() {
            debug_assert!(ptr::eq::<Node>(parent.first_child.get().unwrap(), self));
            parent.first_child.set(Some(new_sibling));
        }
        self.previous_sibling.set(Some(new_sibling));
    }

    pub fn insert_after(&'arena self, new_sibling: &'arena Self) {
        new_sibling.detach();
        new_sibling.parent.set(self.parent.get());
        new_sibling.previous_sibling.set(Some(self));
        if let Some(next_sibling) = self.next_sibling.take() {
            new_sibling.next_sibling.set(Some(next_sibling));
            debug_assert!(ptr::eq::<Node>(
                next_sibling.previous_sibling.get().unwrap(),
                self
            ));
            next_sibling.previous_sibling.set(Some(new_sibling));
        } else if let Some(parent) = self.parent.get() {
            debug_assert!(ptr::eq::<Node>(parent.last_child.get().unwrap(), self));
            parent.last_child.set(Some(new_sibling));
        }
        self.next_sibling.set(Some(new_sibling));
    }
}

impl<'arena> Sink<'arena> {
    fn new_node(&self, data: NodeData<'arena>) -> Ref<'arena> {
        self.arena.alloc(Node::new(data))
    }

    fn append_common<P, A>(&self, child: NodeOrText<Ref<'arena>>, previous: P, append: A)
    where
        P: FnOnce() -> Option<Ref<'arena>>,
        A: FnOnce(Ref<'arena>),
    {
        let new_node = match child {
            NodeOrText::AppendText(text) => {
                // Append to an existing Text node if we have one.
                if let Some(&Node {
                    data: NodeData::Text { ref contents },
                    ..
                }) = previous()
                {
                    contents.borrow_mut().push_tendril(&text);
                    return;
                }
                self.new_node(NodeData::Text {
                    contents: RefCell::new(text),
                })
            }
            NodeOrText::AppendNode(node) => node,
        };

        append(new_node)
    }
}

impl<'arena> TreeSink for Sink<'arena> {
    type Handle = Ref<'arena>;
    type Output = Ref<'arena>;

    fn finish(self) -> Ref<'arena> {
        self.document
    }

    fn parse_error(&mut self, _: Cow<'static, str>) {}

    fn get_document(&mut self) -> Ref<'arena> {
        self.document
    }

    fn set_quirks_mode(&mut self, mode: QuirksMode) {
        self.quirks_mode = mode;
    }

    fn same_node(&self, x: &Ref<'arena>, y: &Ref<'arena>) -> bool {
        ptr::eq::<Node>(*x, *y)
    }

    fn elem_name<'a>(&self, target: &'a Ref<'arena>) -> ExpandedName<'a> {
        match target.data {
            NodeData::Element { ref name, .. } => name.expanded(),
            _ => panic!("not an element!"),
        }
    }

    fn get_template_contents(&mut self, target: &Ref<'arena>) -> Ref<'arena> {
        if let NodeData::Element {
            template_contents: Some(ref contents),
            ..
        } = target.data
        {
            contents
        } else {
            panic!("not a template element!")
        }
    }

    fn is_mathml_annotation_xml_integration_point(&self, target: &Ref<'arena>) -> bool {
        if let NodeData::Element {
            mathml_annotation_xml_integration_point,
            ..
        } = target.data
        {
            mathml_annotation_xml_integration_point
        } else {
            panic!("not an element!")
        }
    }

    fn create_element(
        &mut self,
        name: QualName,
        attrs: Vec<Attribute>,
        flags: ElementFlags,
    ) -> Ref<'arena> {
        self.new_node(NodeData::Element {
            name: name,
            attrs: RefCell::new(attrs),
            template_contents: if flags.template {
                Some(self.new_node(NodeData::Document))
            } else {
                None
            },
            mathml_annotation_xml_integration_point: flags.mathml_annotation_xml_integration_point,
        })
    }

    fn create_comment(&mut self, text: StrTendril) -> Ref<'arena> {
        self.new_node(NodeData::Comment { contents: text })
    }

    fn create_pi(&mut self, target: StrTendril, data: StrTendril) -> Ref<'arena> {
        self.new_node(NodeData::ProcessingInstruction {
            target: target,
            contents: data,
        })
    }

    fn append(&mut self, parent: &Ref<'arena>, child: NodeOrText<Ref<'arena>>) {
        self.append_common(
            child,
            || parent.last_child.get(),
            |new_node| parent.append(new_node),
        )
    }

    fn append_before_sibling(&mut self, sibling: &Ref<'arena>, child: NodeOrText<Ref<'arena>>) {
        self.append_common(
            child,
            || sibling.previous_sibling.get(),
            |new_node| sibling.insert_before(new_node),
        )
    }

    fn append_based_on_parent_node(
        &mut self,
        element: &Ref<'arena>,
        prev_element: &Ref<'arena>,
        child: NodeOrText<Ref<'arena>>,
    ) {
        if element.parent.get().is_some() {
            self.append_before_sibling(element, child)
        } else {
            self.append(prev_element, child)
        }
    }

    fn append_doctype_to_document(
        &mut self,
        name: StrTendril,
        public_id: StrTendril,
        system_id: StrTendril,
    ) {
        self.document.append(self.new_node(NodeData::Doctype {
            name: name,
            public_id: public_id,
            system_id: system_id,
        }))
    }

    fn add_attrs_if_missing(&mut self, target: &Ref<'arena>, attrs: Vec<Attribute>) {
        let mut existing = if let NodeData::Element { ref attrs, .. } = target.data {
            attrs.borrow_mut()
        } else {
            panic!("not an element")
        };

        let existing_names = existing
            .iter()
            .map(|e| e.name.clone())
            .collect::<HashSet<_>>();
        existing.extend(
            attrs
                .into_iter()
                .filter(|attr| !existing_names.contains(&attr.name)),
        );
    }

    fn remove_from_parent(&mut self, target: &Ref<'arena>) {
        target.detach()
    }

    fn reparent_children(&mut self, node: &Ref<'arena>, new_parent: &Ref<'arena>) {
        let mut next_child = node.first_child.get();
        while let Some(child) = next_child {
            debug_assert!(ptr::eq::<Node>(child.parent.get().unwrap(), *node));
            next_child = child.next_sibling.get();
            new_parent.append(child)
        }
    }
}

// Implementation adapted from implementation for RcDom:
// https://github.com/servo/html5ever/blob/45b2fca5c6/markup5ever/rcdom.rs#L410
impl<'arena> Serialize for Node<'arena> {
    fn serialize<S>(&self, serializer: &mut S, traversal_scope: TraversalScope) -> io::Result<()>
    where
        S: Serializer,
    {
        match (&traversal_scope, &self.data) {
            (
                _,
                &NodeData::Element {
                    ref name,
                    ref attrs,
                    ..
                },
            ) => {
                if traversal_scope == IncludeNode {
                    serializer.start_elem(
                        name.clone(),
                        attrs.borrow().iter().map(|at| (&at.name, &at.value[..])),
                    )?;
                }

                if let Some(child) = self.first_child.get() {
                    child.serialize(serializer, IncludeNode)?;
                }

                if traversal_scope == IncludeNode {
                    serializer.end_elem(name.clone())?;
                }
            }

            (&ChildrenOnly(_), &NodeData::Document) => {
                if let Some(child) = self.first_child.get() {
                    child.serialize(serializer, IncludeNode)?;
                }
            }

            (&ChildrenOnly(_), _) => {},

            (&IncludeNode, &NodeData::Doctype { ref name, .. }) => serializer.write_doctype(&name)?,
            (&IncludeNode, &NodeData::Text { ref contents }) => {
                serializer.write_text(&contents.borrow())?
            }
            (&IncludeNode, &NodeData::Comment { ref contents }) => {
                serializer.write_comment(&contents)?
            }
            (
                &IncludeNode,
                &NodeData::ProcessingInstruction {
                    ref target,
                    ref contents,
                },
            ) => serializer.write_processing_instruction(target, contents)?,
            (&IncludeNode, &NodeData::Document) => panic!("Can't serialize Document node itself"),
        }

        if let Some(sibling) = self.next_sibling.get() {
            sibling.serialize(serializer, IncludeNode)?
        }

        Ok(())
    }
}
