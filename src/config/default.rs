use std::collections::{HashMap, HashSet};

use crate::sanitizer::{ContentWhitespace, SanitizerConfig};

lazy_static! {
    pub static ref DEFAULT_CONFIG: SanitizerConfig = SanitizerConfig {
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
        remove_contents_when_unwrapped: hashset! {
            local_name!("iframe"),
            local_name!("noembed"),
            local_name!("noframes"),
            local_name!("noscript"),
            local_name!("script"),
            local_name!("style"),
        },
        whitespace_around_unwrapped_content: hashmap! {
            local_name!("address") => ContentWhitespace::space_around(),
            local_name!("article") => ContentWhitespace::space_around(),
            local_name!("aside") => ContentWhitespace::space_around(),
            local_name!("blockquote") => ContentWhitespace::space_around(),
            local_name!("br") => ContentWhitespace::space_around(),
            local_name!("dd") => ContentWhitespace::space_around(),
            local_name!("div") => ContentWhitespace::space_around(),
            local_name!("dl") => ContentWhitespace::space_around(),
            local_name!("footer") => ContentWhitespace::space_around(),
            local_name!("h1") => ContentWhitespace::space_around(),
            local_name!("h2") => ContentWhitespace::space_around(),
            local_name!("h3") => ContentWhitespace::space_around(),
            local_name!("h4") => ContentWhitespace::space_around(),
            local_name!("h5") => ContentWhitespace::space_around(),
            local_name!("h6") => ContentWhitespace::space_around(),
            local_name!("header") => ContentWhitespace::space_around(),
            local_name!("hgroup") => ContentWhitespace::space_around(),
            local_name!("hr") => ContentWhitespace::space_around(),
            local_name!("li") => ContentWhitespace::space_around(),
            local_name!("nav") => ContentWhitespace::space_around(),
            local_name!("ol") => ContentWhitespace::space_around(),
            local_name!("p") => ContentWhitespace::space_around(),
            local_name!("pre") => ContentWhitespace::space_around(),
            local_name!("section") => ContentWhitespace::space_around(),
            local_name!("ul") => ContentWhitespace::space_around(),
        }
    };
}
