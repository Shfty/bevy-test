use bevy::{
    prelude::{HandleUntyped, Plugin, Shader},
    reflect::TypeUuid,
};

use crate::load_internal_asset;

pub const BEZIER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 1498276717822715012);

pub struct BezierPlugin;

impl Plugin for BezierPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        load_internal_asset!(app, BEZIER_HANDLE, "bezier.wgsl", Shader, Shader::from_wgsl);
    }
}
