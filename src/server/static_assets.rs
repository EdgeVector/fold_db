use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "src/server/static-react"]
#[prefix = "/"]
pub struct Asset;
