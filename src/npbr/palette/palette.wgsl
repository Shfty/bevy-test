#define_import_path npbr::palette
#import npbr::util

struct PaletteInput {
    color: f32,
    brightness: f32,
}

struct HdrInput {
    range: vec2<f32>,
    bezier: vec4<f32>,
    factor: f32,
}

struct PaletteLightingInput {
    luminance_range: vec2<f32>,
    color_bezier: vec4<f32>,
}

fn palette_dim(palette_texture: texture_3d<f32>) -> vec3<f32> {
    let palette_size = textureDimensions(palette_texture);
    return vec3<f32>(palette_size - vec3<u32>(1));
}

// Generate a nearest-neighbour palette UV with a given palette-space offset
fn palette_coord(
    palette_texture: texture_3d<f32>,
    uv: vec3<f32>,
    offset: vec3<f32>
) -> vec3<f32> {
    let dim = palette_dim(palette_texture);

    var uv = uv;

    // Transform to palette space
    uv *= dim;

    // Apply offset
    uv += offset;

    // Round to nearest texel
    uv = round(uv);

    // Clamp to palette size
    uv = max(uv, vec3<f32>(0.0));
    uv = min(uv, dim);

    // Transform to unit space
    uv /= dim;

    return uv;
}

fn sample_palette(
    palette_texture: texture_3d<f32>,
    palette_sampler: sampler,
    coord: vec3<f32>
) -> vec4<f32> {
    let dim = palette_dim(palette_texture);

    // Copy for palette lookup
    var palette_uv = coord;

    // Remap from 0..palette_dim to 0..palette_dim - 1
    palette_uv *= vec3<f32>(1.0) - (vec3<f32>(1.0) / dim);

    // Apply half-pixel offset
    palette_uv += 0.5 / dim;

    // Sample palette
    return textureSampleLevel(
        palette_texture,
        palette_sampler,
        palette_uv,
        0.0,
        vec3<i32>(0, 0, 0),
    );
}

fn hdr_multiplier(
    coord: vec3<f32>,
    hdr: HdrInput,
) -> f32 {
    return 1.0 +
        bezier_easing(
            map_range_vec(
                coord.y,
                hdr.range,
                vec2<f32>(0.0, 1.0),
            ),
            hdr.bezier
        ) * hdr.factor;
}

fn apply_hdr(
    coord: vec3<f32>,
    palette_color: vec4<f32>,
    hdr: HdrInput,
) -> vec4<f32> {
    var rgb = palette_color.rgb;

    rgb *= hdr_multiplier(
        coord,
        hdr,
    );

    return vec4<f32>(rgb.r, rgb.g, rgb.b, palette_color.a);
}

fn palette(
    palette_texture: texture_3d<f32>,
    palette_sampler: sampler,
    uv: vec3<f32>,
    offset: vec3<f32>,
) -> vec4<f32> {
    let coord = palette_coord(
        palette_texture,
        uv,
        offset
    );

    let palette_color = sample_palette(
        palette_texture,
        palette_sampler,
        coord
    );

    return palette_color;
}

fn palette_hdr(
    palette_texture: texture_3d<f32>,
    palette_sampler: sampler,
    uv: vec3<f32>,
    offset: vec3<f32>,
    hdr: HdrInput,
) -> vec4<f32> {
    let coord = palette_coord(
        palette_texture,
        uv,
        offset
    );

    let palette_color = sample_palette(
        palette_texture,
        palette_sampler,
        coord
    );

    let palette_color = apply_hdr(
        coord,
        palette_color,
        hdr
    );

    return palette_color;
}

fn palette_lighting(
    palette_texture: texture_3d<f32>,
    pbr: vec4<f32>,
    palette_input: PaletteInput,
    lighting_input: PaletteLightingInput,
) -> vec3<f32> {
    let dim = palette_dim(palette_texture);

    // Map lighting into HSL, extract saturation and luminosity
    let hsl = nonlinear_srgb_to_hsl(pbr.rgb);
    let hue = hsl.x;
    let saturation = hsl.y;
    let luminosity = hsl.z;

    // Create palette UV from saturation and remapped luminosity
    return vec3<f32>(
        palette_input.brightness,
        bezier_easing(
            map_range_vec(
                luminosity,
                lighting_input.luminance_range,
                vec2<f32>(0.0, 1.0),
            ),
            lighting_input.color_bezier
        ),
        palette_input.color / dim.z,
    );

}

