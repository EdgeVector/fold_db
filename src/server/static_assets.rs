use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "src/server/static-react/dist"]
#[prefix = "/"]
pub struct Asset;
