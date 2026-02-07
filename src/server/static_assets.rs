#[cfg(feature = "embedded-assets")]
use rust_embed::RustEmbed;

#[cfg(feature = "embedded-assets")]
#[derive(RustEmbed)]
#[folder = "src/server/static-react/dist"]
#[prefix = "/"]
pub struct Asset;

#[cfg(not(feature = "embedded-assets"))]
#[derive(Debug, Clone)]
pub struct Asset;
