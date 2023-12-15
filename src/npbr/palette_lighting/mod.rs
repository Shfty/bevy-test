use std::borrow::Cow;

use bevy::{
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::{
        AlphaMode, Assets, Bundle, Commands, Component, ComputedVisibility, Entity,
        GlobalTransform, Handle, HandleUntyped, Image, Material, MaterialPlugin, Mesh, Plugin,
        Query, Res, Shader, StandardMaterial, Transform, Visibility, World,
    },
    reflect::{FromReflect, Reflect, TypeUuid},
    render::{
        mesh::MeshVertexBufferLayout,
        render_resource::{
            AsBindGroup, AsBindGroupShaderType, Face, RenderPipelineDescriptor, Source,
            SpecializedMeshPipelineError,
        },
    },
};

use crate::{image_loader::ImageLoader, load_internal_asset, npbr::BaseMaterialUniform};

use super::{
    dither::DitherInput,
    palette::{HdrInput, PaletteInput, PaletteLightingInput},
    sdf::sdf_3d::Sdf3dModule,
    shader_composer::{ShaderComposer, ShaderFunction, ShaderModule},
};

pub const PALETTE_LIGHTING_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 8554004820303325000);

pub struct PaletteLightingPlugin;

impl Plugin for PaletteLightingPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        load_internal_asset!(
            app,
            PALETTE_LIGHTING_HANDLE,
            "palette_lighting.wgsl",
            Shader,
            Shader::from_wgsl
        );

        app.add_plugin(MaterialPlugin::<PaletteLightingMaterial>::default())
            .add_system(palette_lighting_material);
    }
}

#[derive(Debug, Clone)]
pub struct SdfGeometryFunction(pub Cow<'static, str>);

impl ShaderFunction for SdfGeometryFunction {
    fn name(&self) -> &str {
        "sdf_geometry_impl"
    }

    fn inputs(&self) -> Vec<(&str, &str)> {
        vec![("in", "ptr<function, FragmentInput>")]
    }

    fn output(&self) -> Option<&str> {
        Some("SdfOutput")
    }

    fn body(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct DitherCoordFunction(pub Cow<'static, str>);

impl ShaderFunction for DitherCoordFunction {
    fn name(&self) -> &str {
        "dither_coord_impl"
    }

    fn inputs(&self) -> Vec<(&str, &str)> {
        vec![("in", "FragmentInput")]
    }

    fn output(&self) -> Option<&str> {
        Some("vec2<f32>")
    }

    fn body(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct DitherFunction(pub Cow<'static, str>);

impl ShaderFunction for DitherFunction {
    fn name(&self) -> &str {
        "dither_impl"
    }

    fn inputs(&self) -> Vec<(&str, &str)> {
        vec![("in", "FragmentInput"), ("dither_uv", "vec2<f32>")]
    }

    fn output(&self) -> Option<&str> {
        Some("f32")
    }

    fn body(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct PaletteCoordFunction(pub Cow<'static, str>);

impl ShaderFunction for PaletteCoordFunction {
    fn name(&self) -> &str {
        "palette_coord_impl"
    }

    fn inputs(&self) -> Vec<(&str, &str)> {
        vec![("in", "FragmentInput")]
    }

    fn output(&self) -> Option<&str> {
        Some("vec3<f32>")
    }

    fn body(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct PaletteFunction(pub Cow<'static, str>);

impl ShaderFunction for PaletteFunction {
    fn name(&self) -> &str {
        "palette_impl"
    }

    fn inputs(&self) -> Vec<(&str, &str)> {
        vec![
            ("in", "FragmentInput"),
            ("palette_uv", "vec2<f32>"),
            ("dither", "vec2<f32>"),
        ]
    }

    fn output(&self) -> Option<&str> {
        Some("vec4<f32>")
    }

    fn body(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct AlphaFunction(pub Cow<'static, str>);

impl ShaderFunction for AlphaFunction {
    fn name(&self) -> &str {
        "alpha_impl"
    }

    fn inputs(&self) -> Vec<(&str, &str)> {
        vec![
            ("in", "FragmentInput"),
            ("sdf_out", "SdfOutput"),
            ("a", "f32"),
        ]
    }

    fn output(&self) -> Option<&str> {
        Some("f32")
    }

    fn body(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Default, Clone)]
pub struct PaletteLightingShader {
    module_sdf: Option<Sdf3dModule>,
    function_sdf_geometry: Option<SdfGeometryFunction>,
    function_dither_coord: Option<DitherCoordFunction>,
    function_dither: Option<DitherFunction>,
    function_palette_coord: Option<PaletteCoordFunction>,
    function_palette: Option<PaletteFunction>,
    function_alpha: Option<AlphaFunction>,
}

impl PaletteLightingShader {
    pub fn with_sdf_3d_module(mut self, m: Sdf3dModule) -> Self {
        self.module_sdf = Some(m);
        self
    }

    pub fn with_sdf_geometry_function(mut self, f: SdfGeometryFunction) -> Self {
        self.function_sdf_geometry = Some(f);
        self
    }

    pub fn with_dither_coord_function(mut self, f: DitherCoordFunction) -> Self {
        self.function_dither_coord = Some(f);
        self
    }

    pub fn with_dither_function(mut self, f: DitherFunction) -> Self {
        self.function_dither = Some(f);
        self
    }

    pub fn with_palette_coord_function(mut self, f: PaletteCoordFunction) -> Self {
        self.function_palette_coord = Some(f);
        self
    }

    pub fn with_palette_function(mut self, f: PaletteFunction) -> Self {
        self.function_palette = Some(f);
        self
    }

    pub fn with_alpha_function(mut self, f: AlphaFunction) -> Self {
        self.function_alpha = Some(f);
        self
    }
}

impl ShaderComposer for PaletteLightingShader {
    fn base(&self) -> Source {
        Source::Wgsl(include_str!("palette_lighting.wgsl").into())
    }

    fn emplace(&self, source: &mut Source) {
        if let Some(module_sdf) = self.module_sdf.as_ref() {
            ShaderModule::emplace(module_sdf, source);
        }

        if let Some(function_sdf_geometry) = self.function_sdf_geometry.as_ref() {
            function_sdf_geometry.emplace(source);
        }

        if let Some(function_dither_coord) = self.function_dither_coord.as_ref() {
            function_dither_coord.emplace(source);
        }

        if let Some(function_dither) = self.function_dither.as_ref() {
            function_dither.emplace(source);
        }

        if let Some(function_palette_coord) = self.function_palette_coord.as_ref() {
            function_palette_coord.emplace(source);
        }

        if let Some(function_palette) = self.function_palette.as_ref() {
            function_palette.emplace(source);
        }

        if let Some(function_alpha) = self.function_alpha.as_ref() {
            function_alpha.emplace(source);
        }
    }
}

#[derive(Debug, Default, Clone, AsBindGroup, TypeUuid, Component, Reflect, FromReflect)]
#[uuid = "4a933d92-9cb6-4184-93b4-b3e928997995"]
#[bind_group_data(PaletteLightingMaterialKey)]
#[uniform(0, BaseMaterialUniform)]
pub struct PaletteLightingMaterial {
    #[uniform(1)]
    pub palette_input: PaletteInput,

    #[uniform(2)]
    pub palette_lighting_input: PaletteLightingInput,

    #[uniform(3)]
    pub dither_input: DitherInput,

    #[uniform(4)]
    pub hdr_input: HdrInput,

    #[texture(5, dimension = "3d")]
    #[sampler(6)]
    pub palette_texture: Handle<Image>,

    #[texture(7)]
    #[sampler(8)]
    pub dither_texture: Handle<Image>,

    pub base: StandardMaterial,
    pub shader_handle: Option<Handle<Shader>>,
}

impl AsBindGroupShaderType<BaseMaterialUniform> for PaletteLightingMaterial {
    fn as_bind_group_shader_type(
        &self,
        images: &bevy::render::render_asset::RenderAssets<Image>,
    ) -> BaseMaterialUniform {
        BaseMaterialUniform {
            base: self.base.as_bind_group_shader_type(images),
        }
    }
}

impl Material for PaletteLightingMaterial {
    fn alpha_mode(&self) -> AlphaMode {
        self.base.alpha_mode()
    }

    fn depth_bias(&self) -> f32 {
        self.base.depth_bias()
    }

    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayout,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let fragment_descriptor = descriptor.fragment.as_mut().unwrap();

        if let Some(shader) = key.bind_group_data.shader_handle {
            fragment_descriptor.shader = shader.clone();
        }

        descriptor.primitive.cull_mode = key.bind_group_data.cull_mode;

        if let Some(label) = &mut descriptor.label {
            *label = format!("palette_lighting_{}", *label).into();
        }

        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct PaletteLightingMaterialKey {
    cull_mode: Option<Face>,
    shader_handle: Option<Handle<Shader>>,
}

impl<'a> From<&'a PaletteLightingMaterial> for PaletteLightingMaterialKey {
    fn from(value: &'a PaletteLightingMaterial) -> Self {
        PaletteLightingMaterialKey {
            cull_mode: value.base.cull_mode,
            shader_handle: value.shader_handle.clone(),
        }
    }
}

/// A component bundle for entities with a [`Mesh`] and a [`Material`].
#[derive(Clone, Default, Bundle)]
pub struct PaletteLightingMeshBundle {
    pub mesh: Handle<Mesh>,
    pub material: Handle<PaletteLightingMaterial>,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    /// User indication of whether an entity is visible
    pub visibility: Visibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub computed_visibility: ComputedVisibility,
}

impl PaletteLightingMeshBundle {
    pub fn mesh(&self) -> &Handle<Mesh> {
        &self.mesh
    }
}

pub fn palette_lighting_material(
    mut commands: Commands,
    query: Query<(Entity, &PaletteLightingMaterial)>,
    image_loader: Res<ImageLoader>,
) {
    for (entity, material) in query.iter() {
        if image_loader.is_loaded(&material.palette_texture)
            && image_loader.is_loaded(&material.dither_texture)
        {
            commands.add(move |world: &mut World| {
                let material = world
                    .entity_mut(entity)
                    .remove::<PaletteLightingMaterial>()
                    .unwrap();
                let mut dither_materials = world.resource_mut::<Assets<PaletteLightingMaterial>>();
                let handle = dither_materials.add(material);
                world.entity_mut(entity).insert(handle);
            })
        }
    }
}
