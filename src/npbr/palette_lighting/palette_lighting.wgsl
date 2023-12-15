#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::mesh_bindings

#import bevy_pbr::pbr_types
#import bevy_pbr::utils
#import bevy_pbr::clustered_forward
#import bevy_pbr::lighting
#import bevy_pbr::shadows
#import bevy_pbr::pbr_functions

#import sdf::2d
#import sdf::3d

#import npbr::bezier
#import npbr::dither
#import npbr::palette

struct BaseMaterial {
    base: StandardMaterial,
}

struct DitherUniform {
    dither_width: vec3<f32>,
    dither_scale: vec2<f32>,
    scroll_factor: vec2<f32>,
}

struct FragmentInput {
    @builtin(front_facing) is_front: bool,
    @builtin(position) frag_coord: vec4<f32>,
    #import bevy_pbr::mesh_vertex_output
};

struct FragmentOutput {
    @location(0) frag_color: vec4<f32>,
    @builtin(frag_depth) frag_depth: f32,
}

@group(1) @binding(0)
var<uniform> base_material: BaseMaterial;

@group(1) @binding(1)
var<uniform> palette_input: PaletteInput;

@group(1) @binding(2)
var<uniform> palette_lighting_input: PaletteLightingInput;

@group(1) @binding(3)
var<uniform> dither_uniform: DitherUniform;

@group(1) @binding(4)
var<uniform> hdr_input: HdrInput;

@group(1) @binding(5)
var palette_texture: texture_3d<f32>;
@group(1) @binding(6)
var palette_sampler: sampler;

@group(1) @binding(7)
var dither_texture: texture_2d<f32>;
@group(1) @binding(8)
var dither_sampler: sampler;

module!(sdf_3d)

#[function]
fn sdf_geometry_impl(
    in: ptr<function, FragmentInput>,
) -> SdfOutput {
    return SdfOutput();
}

#[function]
fn dither_coord_impl(
    in: FragmentInput,
) -> vec2<f32> {
    return vec2<f32>(0.0);
}

#[function]
fn dither_impl(
    in: FragmentInput,
    dither_uv: vec2<f32>,
) -> f32 {
    return 0.0;
}

#[function]
fn palette_coord_impl(
    in: FragmentInput,
) -> vec3<f32> {
    return vec3<f32>(0.0);
}

#[function]
fn palette_impl(
    in: FragmentInput,
    palette_uv: vec3<f32>,
    dither: vec3<f32>,
) -> vec4<f32> {
    return palette_hdr(
        palette_texture,
        palette_sampler,
        palette_uv,
        dither,
        hdr_input,
    );
}

#[function]
fn alpha_impl(
    in: FragmentInput,
    sdf_out: SdfOutput,
    a: f32,
) -> f32 {
    return 1.0;
}

@fragment
fn fragment(
    in: FragmentInput
) -> FragmentOutput {
    var in = in;
    let sdf_out = sdf_geometry_impl(&in);

    let dither_uv = dither_coord_impl(in);
    
    let dither = dither_impl(in, dither_uv);
    let dither = dither * 2.0 - 1.0;
    let dither = vec3<f32>(dither) * dither_uniform.dither_width;

    let palette_uv = palette_coord_impl(in);

    var palette_color = palette_impl(
        in,
        palette_uv,
        dither,
    );

    palette_color.a = alpha_impl(in, sdf_out, palette_color.a);

    // Tonemap if enabled
#ifdef TONEMAP_IN_SHADER
    palette_color = tone_mapping(palette_color);
#endif

#ifdef VISUALIZE_SDF_STEPS
    palette_color = vec4<f32>(f32(sdf_out.steps) / f32(MAX_MARCHING_STEPS));
#endif

    return FragmentOutput(
        alpha_discard(base_material.base, palette_color),
        in.frag_coord.z,
    );
}
