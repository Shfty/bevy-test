use std::borrow::Cow;

use bevy::{
    prelude::{HandleUntyped, Shader},
    reflect::TypeUuid,
    render::render_resource::Source,
};

use crate::npbr::shader_composer::{ShaderComposer, ShaderFunction, ShaderModule};

pub const SDF_3D_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 677430762715419032);

pub const SDF_3D_MODULE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 302754411503600884);

#[derive(Debug, Default, Clone)]
pub struct Sdf3dModule {
    pub function_position: Option<PositionFunction>,
    pub function_normal: Option<NormalFunction>,
    pub function_uv: Option<UvFunction>,
    pub function_raymarch: Option<RaymarchFunction>,
}
impl ShaderComposer for Sdf3dModule {
    fn base(&self) -> Source {
        Source::Wgsl(include_str!("sdf_3d_module.wgsl").into())
    }

    fn emplace(&self, source: &mut Source) {
        if let Some(function_position) = self.function_position.as_ref() {
            function_position.emplace(source);
        }

        if let Some(function_normal) = self.function_normal.as_ref() {
            function_normal.emplace(source);
        }

        if let Some(function_uv) = self.function_uv.as_ref() {
            function_uv.emplace(source);
        }

        if let Some(function_raymarch) = self.function_raymarch.as_ref() {
            function_raymarch.emplace(source);
        }
    }
}

impl ShaderModule for Sdf3dModule {
    fn name(&self) -> Cow<'static, str> {
        "sdf_3d".into()
    }

    fn module(&self) -> Cow<'static, str> {
        let Source::Wgsl(source) = self.compose() else {
            unreachable!();
        };

        source
    }
}

#[derive(Debug, Clone)]
pub struct PositionFunction(pub Cow<'static, str>);

impl ShaderFunction for PositionFunction {
    fn name(&self) -> &str {
        "sdf_3d_position"
    }

    fn inputs(&self) -> Vec<(&str, &str)> {
        vec![("p", "vec3<f32>")]
    }

    fn output(&self) -> Option<&str> {
        Some("f32")
    }

    fn body(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct NormalFunction(pub Cow<'static, str>);

impl ShaderFunction for NormalFunction {
    fn name(&self) -> &str {
        "sdf_3d_normal"
    }

    fn inputs(&self) -> Vec<(&str, &str)> {
        vec![("p", "vec3<f32>")]
    }

    fn output(&self) -> Option<&str> {
        Some("vec3<f32>")
    }

    fn body(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct UvFunction(pub Cow<'static, str>);

impl ShaderFunction for UvFunction {
    fn name(&self) -> &str {
        "sdf_3d_uv"
    }

    fn inputs(&self) -> Vec<(&str, &str)> {
        vec![("p", "vec3<f32>"), ("n", "vec3<f32>")]
    }

    fn output(&self) -> Option<&str> {
        Some("vec2<f32>")
    }

    fn body(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct RaymarchFunction(pub Cow<'static, str>);

impl ShaderFunction for RaymarchFunction {
    fn name(&self) -> &str {
        "sdf_3d_raymarch"
    }

    fn inputs(&self) -> Vec<(&str, &str)> {
        vec![
            ("start", "f32"),
            ("end", "f32"),
            ("eye", "vec3<f32>"),
            ("dir", "vec3<f32>"),
        ]
    }

    fn output(&self) -> Option<&str> {
        Some("SdfOutput")
    }

    fn body(&self) -> &str {
        &self.0
    }
}
