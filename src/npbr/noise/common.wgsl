#define_import_path npbr::noise::common

fn _fade(t: vec2<f32>) -> vec2<f32> {
    return t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
}

fn gln_rand4(p: vec4<f32>) -> vec4<f32> {
    return (((p * 34.0) + 1.0) * p) % 289.0;
}

