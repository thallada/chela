use crate::config::default::DEFAULT_CONFIG;
use crate::sanitizer::SanitizerConfig;

lazy_static! {
    pub static ref RESTRICTED_CONFIG: SanitizerConfig = {
        let mut config = DEFAULT_CONFIG.clone();
        config.allowed_elements.extend(hashset! {
            local_name!("b"),
            local_name!("em"),
            local_name!("i"),
            local_name!("strong"),
            local_name!("u"),
        });
        config
    };
}
