pub mod bezier;
pub mod dither;
pub mod noise;
pub mod palette;
pub mod palette_lighting;
pub mod ring_buffer;
pub mod sdf;
pub mod stack_machine;
pub mod shader_composer;
pub mod texture_interpreter;
pub mod util;

use bevy::{prelude::*, pbr::StandardMaterialUniform, render::render_resource::ShaderType};

use bezier::BezierPlugin;
use dither::DitherPlugin;
use noise::NoisePlugin;
use palette::PalettePlugin;
use ring_buffer::RingBufferPlugin;
use sdf::SdfPlugin;
use texture_interpreter::TextureInterpreterPlugin;
use util::UtilPlugin;

pub struct NpbrPlugin;

impl Plugin for NpbrPlugin {
    fn build(&self, app: &mut App) {
        BaseMaterialUniform::assert_uniform_compat();

        app.add_plugin(NoisePlugin)
            .add_plugin(BezierPlugin)
            .add_plugin(UtilPlugin)
            .add_plugin(PalettePlugin)
            .add_plugin(DitherPlugin)
            .add_plugin(SdfPlugin)
            .add_plugin(RingBufferPlugin)
            .add_plugin(TextureInterpreterPlugin);
    }
}

#[derive(Default, ShaderType)]
pub struct BaseMaterialUniform {
    base: StandardMaterialUniform,
}
