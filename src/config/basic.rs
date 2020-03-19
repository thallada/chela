use html5ever::LocalName;

use std::collections::{HashMap, HashSet};

lazy_static! {
    pub static ref ELEMENTS: HashSet<LocalName> = hashset! {
        local_name!("a"),
        local_name!("abbr"),
        local_name!("blockquote"),
        local_name!("br"),
        local_name!("cite"),
        local_name!("code"),
        local_name!("dd"),
        local_name!("dfn"),
        local_name!("dl"),
        local_name!("dt"),
        local_name!("kbd"),
        local_name!("li"),
        local_name!("mark"),
        local_name!("ol"),
        local_name!("p"),
        local_name!("pre"),
        local_name!("q"),
        local_name!("s"),
        local_name!("samp"),
        local_name!("small"),
        local_name!("strike"),
        local_name!("sub"),
        local_name!("sup"),
        local_name!("time"),
        local_name!("ul"),
        local_name!("var"),
    };
    pub static ref ALL_ATTRIBUTES: HashSet<LocalName> = hashset! {};
    pub static ref ATTRIBUTES: HashMap<LocalName, HashSet<LocalName>> = hashmap! {
        local_name!("a") => hashset!{
            local_name!("href"),
        },
        local_name!("abbr") => hashset!{
            local_name!("title"),
        },
        local_name!("blockquote") => hashset!{
            local_name!("cite"),
        },
        local_name!("dfn") => hashset!{
            local_name!("title"),
        },
        local_name!("q") => hashset!{
            local_name!("cite"),
        },
        local_name!("time") => hashset!{
            local_name!("datetime"),
            LocalName::from("pubdate"),
        },
    };
    pub static ref ADD_ATTRIBUTES: HashMap<LocalName, HashMap<LocalName, &'static str>> = hashmap! {
        local_name!("a") => hashmap! {
            local_name!("rel") => "nofollow",
        },
    };
    pub static ref PROTOCOLS: HashMap<LocalName, HashMap<LocalName, HashSet<&'static str>>> = hashmap! {
        local_name!("a") => hashmap! {
            local_name!("href") => hashset!{"ftp", "http", "https", "mailto"},
        },
        local_name!("blockquote") => hashmap! {
            local_name!("cite") => hashset!{"http", "https"},
        },
        local_name!("q") => hashmap! {
            local_name!("cite") => hashset!{"http", "https"},
        },
    };
    pub static ref CSS_PROPERTIES: Vec<String> = vec![];
}
