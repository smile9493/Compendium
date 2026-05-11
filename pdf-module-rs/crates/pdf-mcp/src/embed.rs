use rust_embed::Embed;

/// Embedded SPA assets from the Vue3 build output.
/// Files are embedded at compile time via rust-embed.
#[derive(Embed)]
#[folder = "pdf-web-ui/dist/"]
pub struct Assets;
