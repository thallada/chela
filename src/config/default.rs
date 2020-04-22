use std::collections::{HashMap, HashSet};

use crate::sanitizer::SanitizerConfig;

lazy_static! {
    pub static ref DEFAULT_CONFIG: SanitizerConfig = SanitizerConfig {
        allow_comments: false,
        allowed_elements: HashSet::new(),
        allowed_attributes: HashSet::new(),
        allowed_attributes_per_element: HashMap::new(),
        add_attributes: HashMap::new(),
        add_attributes_per_element: HashMap::new(),
        allowed_protocols: HashMap::new(),
        allowed_css_at_rules: HashSet::new(),
        allowed_css_properties: HashSet::new(),
        remove_contents_when_unwrapped: hashset! {
            local_name!("iframe"),
            local_name!("noembed"),
            local_name!("noframes"),
            local_name!("noscript"),
            local_name!("script"),
            local_name!("style"),
        },
    };
}
