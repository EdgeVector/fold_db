use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "src/server/static-react/dist"]
pub struct Asset;
