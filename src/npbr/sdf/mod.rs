pub mod sdf_2d;
pub mod sdf_3d;

use bevy::prelude::{Plugin, Shader};

use crate::load_internal_asset;

#[allow(unused_imports)]
use sdf_2d::SDF_2D_HANDLE;
#[allow(unused_imports)]
use sdf_3d::SDF_3D_HANDLE;
#[allow(unused_imports)]
use sdf_3d::SDF_3D_MODULE_HANDLE;

pub struct SdfPlugin;

impl Plugin for SdfPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        load_internal_asset!(
            app,
            SDF_2D_HANDLE,
            "sdf_2d/sdf_2d.wgsl",
            Shader,
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            SDF_3D_HANDLE,
            "sdf_3d/sdf_3d.wgsl",
            Shader,
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            SDF_3D_MODULE_HANDLE,
            "sdf_3d/sdf_3d_module.wgsl",
            Shader,
            Shader::from_wgsl
        );
    }
}
