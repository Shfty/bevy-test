#define_import_path sdf::3d

let MAX_MARCHING_STEPS = 150u;
let MAX_MARCHING_DIST = 1000.0;
let EPSILON = 0.005;

struct SdfOutput {
    hit: bool,
    dist: f32,
    steps: u32,
}

fn sdf_3d_point(p: vec3<f32>) -> f32 {
    return length(p);
}

fn sdf_3d_sphere(p: vec3<f32>, radius: f32) -> f32 {
    return sdf_3d_point(p) - radius;
}

fn sdf_3d_torus(p: vec3<f32>, t: vec2<f32>) -> f32 {
    let q = vec2<f32>(length(p.xz) - t.x, p.y);
    return length(q) - t.y;
}

fn sdf_3d_cone(p: vec3<f32>, c: vec2<f32>, h: f32) -> f32 {
    var p = p;
    p.y -= h / 2.0;
    let q = h * vec2<f32>(c.x / c.y, -1.0);

    let w = vec2<f32>(length(p.xz), p.y);
    let a = w - q * clamp(dot(w, q)/dot(q, q), 0.0, 1.0);
    let b = w - q * vec2<f32>(clamp(w.x / q.x, 0.0, 1.0), 1.0);
    let k = sign(q.y);
    let d = min(dot(a, a), dot(b, b));
    let s = max(k * (w.x * q.y - w.y * q.x), k * (w.y - q.y));
    return sqrt(d) * sign(s);
}

fn sdf_3d_cylinder(p: vec3<f32>, h: f32, r: f32) -> f32 {
    let d = abs(vec2<f32>(length(p.xz), p.y)) - vec2<f32>(r, h);
    return min(max(d.x, d.y), 0.0) + length(max(d, vec2<f32>(0.0)));
}

fn sdf_3d_round_cone(
    p: vec3<f32>,
    r1: f32,
    r2: f32,
    h: f32
) -> f32 {
    var p = p;
    p.y += h / 2.0;

    let b = (r1 - r2) / h;
    let a = sqrt(1.0 - b * b);

    let q = vec2<f32>(length(p.xz), p.y);
    let k = dot(q, vec2<f32>(-b, a));

    if k < 0.0 {
        return length(q) - r1;
    }

    if k > a * h {
        return length(q - vec2<f32>(0.0, h)) - r2;
    }

    return dot(q, vec2<f32>(a, b)) - r1;
}

fn sdf_3d_normal_sphere(
    p: vec3<f32>,
) -> vec3<f32> {
    return normalize(p);
}

fn sdf_3d_uv_sphere(normal: vec3<f32>) -> vec2<f32> {
    return vec2<f32>(
        0.5 + (atan2(normal.z, normal.x) / PI),
        0.5 + (asin(normal.y) / PI),
    );
}

fn sdf_3d_uv_triplanar(position: vec3<f32>, normal: vec3<f32>, sharpness: f32) -> vec2<f32> {
    var weights = abs(normal);
    weights.x = pow(weights.x, sharpness);
    weights.y = pow(weights.y, sharpness);
    weights.z = pow(weights.z, sharpness);
    weights = normalize(weights);

    return (position.zy * weights.x) + (position.xz * weights.y) + (position.xy * weights.z);
}

