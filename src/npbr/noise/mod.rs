use bevy::{
    prelude::{HandleUntyped, Plugin, Shader},
    reflect::TypeUuid,
};

use crate::load_internal_asset;

pub const NOISE_COMMON_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 6470095711402881527);

pub const NOISE_PERLIN_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 2110594331872704138);

pub struct NoisePlugin;

impl Plugin for NoisePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        load_internal_asset!(
            app,
            NOISE_COMMON_HANDLE,
            "common.wgsl",
            Shader,
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            NOISE_PERLIN_HANDLE,
            "perlin.wgsl",
            Shader,
            Shader::from_wgsl
        );
    }
}
