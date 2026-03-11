use rust_embed::RustEmbed;
use std::borrow::Cow;

#[derive(RustEmbed)]
#[folder = "shaders-built/"]
struct Shaders;

#[derive(Clone, Copy, Debug)]
pub enum ShaderId {
    MovingShape,
    Sky,
    SolidBackground,
}

impl ShaderId {
    const fn filename(self) -> &'static str {
        match self {
            ShaderId::MovingShape => "moving-shape.comp.spv",
            ShaderId::Sky => "sky.comp.spv",
            ShaderId::SolidBackground => "solid-background.comp.spv",
        }
    }
}

pub fn get_shader_spv(id: ShaderId) -> Cow<'static, [u8]> {
    Shaders::get(id.filename())
        .expect(&format!("Missing embedded shader: {}", id.filename()))
        .data
}
