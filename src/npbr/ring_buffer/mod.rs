use bevy::{
    prelude::{HandleUntyped, Plugin, Shader},
    reflect::TypeUuid,
};

use crate::load_internal_asset;

pub const RING_BUFFER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 166189258136563402);

pub struct RingBufferPlugin;

impl Plugin for RingBufferPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        load_internal_asset!(
            app,
            RING_BUFFER_HANDLE,
            "ring_buffer.wgsl",
            Shader,
            Shader::from_wgsl
        );
    }
}
