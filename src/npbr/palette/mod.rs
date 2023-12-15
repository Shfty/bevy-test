use bevy::{
    prelude::{App, HandleUntyped, Plugin, Shader, Vec2, Vec4},
    reflect::{FromReflect, Reflect, TypeUuid},
    render::render_resource::ShaderType,
};

use crate::load_internal_asset;

pub const PALETTE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 138171658569322273);

pub struct PalettePlugin;

impl Plugin for PalettePlugin {
    fn build(&self, app: &mut App) {
        PaletteLightingInput::assert_uniform_compat();
        HdrInput::assert_uniform_compat();

        load_internal_asset!(
            app,
            PALETTE_HANDLE,
            "palette.wgsl",
            Shader,
            Shader::from_wgsl
        );
    }
}

#[derive(Debug, Copy, Clone, Reflect, FromReflect, ShaderType)]
pub struct PaletteInput {
    pub color: f32,
    pub brightness: f32,
}

impl Default for PaletteInput {
    fn default() -> Self {
        PaletteInput {
            color: 0.0,
            brightness: 0.5,
        }
    }
}

#[derive(Debug, Copy, Clone, Reflect, FromReflect, ShaderType)]
pub struct PaletteLightingInput {
    pub luminance_range: Vec2,
    pub color_bezier: Vec4,
}

impl Default for PaletteLightingInput {
    fn default() -> Self {
        PaletteLightingInput {
            luminance_range: Vec2::new(0.0, 13.85),
            color_bezier: Vec4::new(0.0, 1.0, 0.33, 0.33),
        }
    }
}

#[derive(Debug, Copy, Clone, Reflect, FromReflect, ShaderType)]
pub struct HdrInput {
    pub range: Vec2,
    pub bezier: Vec4,
    factor: f32,
}

impl Default for HdrInput {
    fn default() -> Self {
        Self {
            range: Vec2::new(0.33, 1.0),
            bezier: Vec4::new(1.0, 0.0, 1.0, 1.0),
            factor: 13.85,
        }
    }
}
