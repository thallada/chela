use html5ever::LocalName;

use std::collections::{HashMap, HashSet};

use super::basic::{
    ADD_ATTRIBUTES as BASIC_ADD_ATTRIBUTES, ALL_ATTRIBUTES as BASIC_ALL_ATTRIBUTES,
    ATTRIBUTES as BASIC_ATTRIBUTES, ELEMENTS as BASIC_ELEMENTS,
};

lazy_static! {
    pub static ref ELEMENTS: HashSet<LocalName> = BASIC_ELEMENTS
        .union(&hashset!(
            local_name!("address"),
            local_name!("article"),
            local_name!("aside"),
            local_name!("bdi"),
            local_name!("bdo"),
            local_name!("body"),
            local_name!("caption"),
            local_name!("col"),
            local_name!("colgroup"),
            local_name!("data"),
            local_name!("del"),
            local_name!("div"),
            local_name!("figcaption"),
            local_name!("figure"),
            local_name!("footer"),
            local_name!("h1"),
            local_name!("h2"),
            local_name!("h3"),
            local_name!("h4"),
            local_name!("h5"),
            local_name!("h6"),
            local_name!("head"),
            local_name!("header"),
            local_name!("hgroup"),
            local_name!("hr"),
            local_name!("html"),
            local_name!("img"),
            local_name!("ins"),
            local_name!("main"),
            local_name!("nav"),
            local_name!("rp"),
            local_name!("rt"),
            local_name!("ruby"),
            local_name!("section"),
            local_name!("span"),
            local_name!("style"),
            local_name!("summary"),
            local_name!("sup"),
            local_name!("table"),
            local_name!("tbody"),
            local_name!("td"),
            local_name!("tfoot"),
            local_name!("th"),
            local_name!("thead"),
            local_name!("title"),
            local_name!("tr"),
            local_name!("wbr"),
        ))
        .into_iter()
        .cloned()
        .collect();
    pub static ref ALL_ATTRIBUTES: HashSet<LocalName> = BASIC_ALL_ATTRIBUTES.union(&hashset! {
        local_name!("class"),
        local_name!("dir"),
        local_name!("hidden"),
        local_name!("id"),
        local_name!("lang"),
        local_name!("style"),
        local_name!("tabindex"),
        local_name!("title"),
        LocalName::from("translate"),
    }).into_iter().cloned().collect();
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
    pub static ref ADD_ATTRIBUTES: HashMap<LocalName, HashMap<LocalName, &'static str>> = BASIC_ADD_ATTRIBUTES.clone();
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
