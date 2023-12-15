use bevy::{
    prelude::{HandleUntyped, Plugin, Shader},
    reflect::TypeUuid,
};

use crate::load_internal_asset;

pub const UTIL_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 10097979282126633923);

pub struct UtilPlugin;

impl Plugin for UtilPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        load_internal_asset!(app, UTIL_HANDLE, "util.wgsl", Shader, Shader::from_wgsl);
    }
}
