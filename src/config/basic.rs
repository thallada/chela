use html5ever::LocalName;

use crate::config::restricted::RESTRICTED_CONFIG;
use crate::sanitizer::{Protocol, SanitizerConfig};

lazy_static! {
    pub static ref BASIC_CONFIG: SanitizerConfig = {
        let mut config = RESTRICTED_CONFIG.clone();
        config.allowed_elements.extend(hashset! {
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
            local_name!("time"),
            local_name!("ul"),
            local_name!("var"),
        });
        config.allowed_attributes_per_element.extend(hashmap! {
            local_name!("a") => hashset! { local_name!("href") },
            local_name!("abbr") => hashset! { local_name!("title") },
            local_name!("blockquote") => hashset! { local_name!("cite") },
            local_name!("dfn") => hashset! { local_name!("title") },
            local_name!("q") => hashset! { local_name!("cite") },
            local_name!("time") => hashset! { local_name!("datetime"), LocalName::from("pubdate") },
        });
        config.add_attributes_per_element.extend(hashmap! {
            local_name!("a") => hashmap! { local_name!("rel") => "href" },
        });
        config.allowed_protocols.extend(hashmap! {
            local_name!("a") => hashmap! { local_name!("href") => hashset! {
                Protocol::Scheme("ftp"),
                Protocol::Scheme("http"),
                Protocol::Scheme("https"),
                Protocol::Scheme("mailto"),
                Protocol::Relative,
            }},
            local_name!("blockquote") => hashmap! { local_name!("cite") => hashset! {
                Protocol::Scheme("http"),
                Protocol::Scheme("https"),
                Protocol::Relative,
            }},
            local_name!("q") => hashmap! { local_name!("cite") => hashset! {
                Protocol::Scheme("http"),
                Protocol::Scheme("https"),
                Protocol::Relative,
            }},
        });
        config
    };
}
