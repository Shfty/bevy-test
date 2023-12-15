use bevy::{
    prelude::{HandleUntyped, Plugin, Shader, Vec2, Vec3},
    reflect::{FromReflect, Reflect, TypeUuid},
    render::render_resource::ShaderType,
};

use crate::load_internal_asset;

pub const DITHER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 9212124841484939528);

pub struct DitherPlugin;

impl Plugin for DitherPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        DitherInput::assert_uniform_compat();

        load_internal_asset!(app, DITHER_HANDLE, "dither.wgsl", Shader, Shader::from_wgsl);
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, FromReflect)]
pub enum DitherMode {
    None,
    Perlin,
    Radial,
    Manhattan,
    Chebyshev,
}

impl Default for DitherMode {
    fn default() -> Self {
        DitherMode::None
    }
}

impl TryFrom<DitherMode> for &'static str {
    type Error = ();

    fn try_from(value: DitherMode) -> Result<Self, Self::Error> {
        match value {
            DitherMode::None => Err(()),
            DitherMode::Perlin => Ok("DITHER_PERLIN"),
            DitherMode::Radial => Ok("DITHER_RADIAL"),
            DitherMode::Manhattan => Ok("DITHER_MANHATTAN"),
            DitherMode::Chebyshev => Ok("DITHER_CHEBYSHEV"),
        }
    }
}

#[derive(Debug, Copy, Clone, Reflect, FromReflect, ShaderType)]
pub struct DitherInput {
    pub dither_width: Vec3,
    pub dither_scale: Vec2,
    pub scroll_factor: Vec2,
}

impl Default for DitherInput {
    fn default() -> Self {
        DitherInput {
            dither_width: Vec3::new(1.0, 1.0, 0.0),
            dither_scale: Vec2::ONE,
            scroll_factor: Vec2::new(1.0, 1.0),
        }
    }
}
