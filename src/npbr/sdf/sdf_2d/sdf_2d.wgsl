#define_import_path sdf::2d

fn sdf_2d_chebyshev(p: vec2<f32>, scale: f32) -> f32 {
    let p = p / scale;
    return max(abs(p.x), abs(p.y));
}

fn sdf_2d_manhattan(p: vec2<f32>, scale: f32) -> f32 {
    let p = p / scale;
    return abs(p.x) + abs(p.y);
}

fn sdf_2d_circle(p: vec2<f32>, radius: f32) -> f32 {
    return length(p) - radius;
}

