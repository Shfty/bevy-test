use bevy::{
    prelude::{HandleUntyped, Plugin, Shader},
    reflect::TypeUuid,
};

use crate::load_internal_asset;

pub const TEXTURE_INTERPRETER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 18583457480289715);

pub struct TextureInterpreterPlugin;

impl Plugin for TextureInterpreterPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        load_internal_asset!(
            app,
            TEXTURE_INTERPRETER_HANDLE,
            "texture_interpreter.wgsl",
            Shader,
            Shader::from_wgsl
        );
    }
}
