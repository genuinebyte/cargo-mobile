pub mod cli;
pub(crate) mod config;
mod deps;
mod device;
mod ios_deploy;
pub(crate) mod project;
mod system_profile;
mod target;
mod teams;

use crate::util::{
    self,
    cli::{Report, TextWrapper},
};

pub static NAME: &str = "apple";

// These checks will have to be refined when this is resolved upstream...
pub fn rust_version_check(wrapper: &TextWrapper) -> Result<(), util::RustVersionError> {
    util::RustVersion::check().map(|version| {
        const MAX: (u32, u32, u32) = (1, 45, 2);
        if version.triple > MAX {
            Report::action_request(
                format!("iOS linking is currently broken on Rust versions later than 1.45.2, and you're on {}!", version),
                "Until this is resolved upstream, switch back to 1.45.2 by running `rustup install stable-2020-08-03 && rustup default stable-2020-08-03`",
            ).print(wrapper);
        }
    })
}
