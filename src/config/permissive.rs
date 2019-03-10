use html5ever::LocalName;

use std::collections::{HashMap, HashSet};

use super::relaxed::{
    ADD_ATTRIBUTES as RELAXED_ADD_ATTRIBUTES, ALL_ATTRIBUTES as RELAXED_ALL_ATTRIBUTES,
    ATTRIBUTES as RELAXED_ATTRIBUTES, ELEMENTS as RELAXED_ELEMENTS,
};

lazy_static! {
    pub static ref ELEMENTS: HashSet<LocalName> = RELAXED_ELEMENTS
        .union(&hashset!(
            local_name!("acronym"),
            local_name!("basefont"),
            local_name!("big"),
            local_name!("blink"),
            local_name!("center"),
            LocalName::from("command"),
            local_name!("dir"),
            local_name!("font"),
            local_name!("marquee"),
            local_name!("strike"),
            local_name!("tt"),
            local_name!("form"),
            local_name!("input"),
            local_name!("button"),
            LocalName::from("single"),
            LocalName::from("double"),
        ))
        .into_iter()
        .cloned()
        .collect();
    pub static ref ALL_ATTRIBUTES: HashSet<LocalName> = RELAXED_ALL_ATTRIBUTES
        .union(&hashset! {
            local_name!("bgcolor"),
            local_name!("width"),
            local_name!("height"),
            local_name!("border"),
            local_name!("color"),
            local_name!("background"),
        })
        .into_iter()
        .cloned()
        .collect();
    // Can't figure out how to merge HashMaps :(
    pub static ref ATTRIBUTES: HashMap<LocalName, HashSet<LocalName>> = hashmap! {
        local_name!("a") => hashset!{
            local_name!("href"),
            local_name!("hreflang"),
            local_name!("name"),
            local_name!("rel"),
        },
        local_name!("abbr") => hashset!{
            local_name!("title"),
        },
        local_name!("blockquote") => hashset!{
            local_name!("cite"),
        },
        local_name!("button") => hashset!{
            local_name!("type"),
        },
        local_name!("col") => hashset!{
            local_name!("span"),
            local_name!("width"),
        },
        local_name!("colgroup") => hashset!{
            local_name!("span"),
            local_name!("width"),
        },
        local_name!("data") => hashset!{
            local_name!("value"),
        },
        local_name!("del") => hashset!{
            local_name!("cite"),
            local_name!("datetime"),
        },
        local_name!("dfn") => hashset!{
            local_name!("title"),
        },
        local_name!("img") => hashset!{
            local_name!("align"),
            local_name!("alt"),
            local_name!("border"),
            local_name!("height"),
            local_name!("src"),
            local_name!("srcset"),
            local_name!("width"),
        },
        local_name!("input") => hashset!{
            local_name!("type"),
            local_name!("name"),
            local_name!("value"),
        },
        local_name!("ins") => hashset!{
            local_name!("cite"),
            local_name!("datetime"),
        },
        local_name!("li") => hashset!{
            local_name!("value"),
        },
        local_name!("ol") => hashset!{
            LocalName::from("reversed"),
            local_name!("start"),
            local_name!("type"),
        },
        local_name!("q") => hashset!{
            local_name!("cite"),
        },
        local_name!("style") => hashset!{
            local_name!("media"),
            local_name!("scoped"),
            local_name!("type"),
        },
        local_name!("table") => hashset!{
            local_name!("align"),
            local_name!("bgcolor"),
            local_name!("border"),
            local_name!("cellpadding"),
            local_name!("cellspacing"),
            local_name!("frame"),
            local_name!("rules"),
            LocalName::from("sortable"),
            local_name!("summary"),
            local_name!("width"),
        },
        local_name!("td") => hashset!{
            local_name!("abbr"),
            local_name!("align"),
            local_name!("axis"),
            local_name!("colspan"),
            local_name!("headers"),
            local_name!("rowspan"),
            local_name!("valign"),
            local_name!("width"),
        },
        local_name!("th") => hashset!{
            local_name!("abbr"),
            local_name!("align"),
            local_name!("axis"),
            local_name!("colspan"),
            local_name!("headers"),
            local_name!("rowspan"),
            local_name!("scope"),
            LocalName::from("sorted"),
            local_name!("valign"),
            local_name!("width"),
        },
        local_name!("time") => hashset!{
            local_name!("datetime"),
            LocalName::from("pubdate"),
        },
        local_name!("ul") => hashset!{
            local_name!("type"),
        },
    };
    pub static ref ADD_ATTRIBUTES: HashMap<LocalName, HashMap<LocalName, &'static str>> = RELAXED_ADD_ATTRIBUTES.clone();
    pub static ref PROTOCOLS: HashMap<LocalName, HashMap<LocalName, HashSet<&'static str>>> = hashmap! {
        local_name!("a") => hashmap! {
            local_name!("href") => hashset!{"ftp", "http", "https", "mailto"},
        },
        local_name!("blockquote") => hashmap! {
            local_name!("cite") => hashset!{"http", "https"},
        },
        local_name!("del") => hashmap! {
            local_name!("cite") => hashset!{"http", "https"},
        },
        local_name!("img") => hashmap! {
            local_name!("src") => hashset!{"http", "https"},
        },
        local_name!("ins") => hashmap! {
            local_name!("cite") => hashset!{"http", "https"},
        },
        local_name!("q") => hashmap! {
            local_name!("cite") => hashset!{"http", "https"},
        },
    };
}
