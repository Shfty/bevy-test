#define_import_path npbr::dither

#import npbr::noise::perlin

fn dithering_perlin(
    viewport_uv: vec2<f32>,
    viewport_size: vec2<f32>,
    dither_scale: f32,
) -> f32 {
    var dither_uv = viewport_uv;
    dither_uv.x *= viewport_size.x / viewport_size.y;
    dither_uv *= dither_scale;
    dither_uv.x += globals.time;

    return gln_perlin(dither_uv);
}

fn dithering_radial(
    viewport_uv: vec2<f32>,
    viewport_size: vec2<f32>,
    dither_scale: f32,
) -> f32 {
    var f = viewport_uv;
    f = (f + 1.0) / 2.0;
    f.x *= viewport_size.x / viewport_size.y;

    let period = 1.0 / dither_scale;
    f = ((f % period) / period) % 1.0;

    f = (f - 0.5) * 2.0;
    var f = length(f);
    f = (f + 1.0) / 2.0;

    return f;
}

fn dithering_manhattan(
    viewport_uv: vec2<f32>,
    viewport_size: vec2<f32>,
    dither_scale: f32,
) -> f32 {
    var f = viewport_uv;

    // Transform to 0..1 range
    f = (f + 1.0) / 2.0;

    // Apply aspect
    f.x *= viewport_size.x / viewport_size.y;

    // Repeat
    let period = 1.0 / dither_scale;
    f = ((f % period) / period) % 1.0;

    // Transform to -1..1 range
    f = (f - 0.5) * 2.0;

    // Evaluate dither
    var f = sdf_2d_manhattan(f, 1.0);

    // Transform to 0..1 range
    f = (f + 1.0) / 2.0;

    return f;
}

fn dithering_chebyshev(
    viewport_uv: vec2<f32>,
    viewport_size: vec2<f32>,
    dither_scale: f32,
) -> f32 {
    var f = viewport_uv;
    f = (f + 1.0) / 2.0;
    f.x *= viewport_size.x / viewport_size.y;

    let period = 1.0 / dither_scale;
    f = ((f % period) / period) % 1.0;

    f = (f - 0.5) * 2.0;
    var f = sdf_2d_chebyshev(f, 1.0);
    f = (f + 1.0) / 2.0;

    return f;
}

fn dithering_texture(
    dither_texture: texture_2d<f32>,
    dither_sampler: sampler,
    viewport_uv: vec2<f32>,
    viewport_size: vec2<f32>,
) -> vec4<f32> {
    // Calculate viewport:dither texture ratio
    let dither_dim = vec2<f32>(textureDimensions(dither_texture).xy);
    let screen_dither_ratio = viewport_size / dither_dim;

    // Sample dither texture
    var screen_uv = viewport_uv;
    screen_uv += 0.5 / dither_dim;
    screen_uv *= screen_dither_ratio;// / input.dither_scale;
    return textureSampleLevel(dither_texture, dither_sampler, screen_uv, 0.0);
}

