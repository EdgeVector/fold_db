use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "src/datafold_node/static-react/dist"]
pub struct Asset;
