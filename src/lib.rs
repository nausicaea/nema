pub mod business_logic;
pub mod modrinth;
pub mod spec;

pub const USER_AGENT: &str = concat!(
    "nausicaea/minecraft/",
    env!("CARGO_PKG_VERSION"),
    " (developer@nausicaea.net)"
);
