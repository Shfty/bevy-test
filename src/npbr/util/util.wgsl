#define_import_path npbr::util

let I32_MIN = -2147483648i;
let I32_MAX = 2147483647i;

let U32_MIN = 0u;
let U32_MAX = 4294967295u;

let F32_MIN = -0x1.fffffep+127f;
let F32_MAX = 0x1.fffffep+127f;
let F32_EPSILON = 0x1.0p-126f;

struct Range {
    start: f32,
    end: f32,
    _pad0: f32,
    _pad1: f32,
}

fn rgb2luma(rgb: vec3<f32>) -> f32 {
    return sqrt(dot(rgb, vec3<f32>(0.299, 0.587, 0.114)));
}

fn nonlinear_srgb_to_hsl(rgb: vec3<f32>) -> vec3<f32> {
    let red = rgb.r;
    let green = rgb.g;
    let blue = rgb.b;

    // https://en.wikipedia.org/wiki/HSL_and_HSV#From_RGB
    let x_max = max(red, max(green, blue));
    let x_min = min(red, min(green, blue));
    let chroma = x_max - x_min;
    let lightness = (x_max + x_min) / 2.0;

    var hue = 0.0;
    if chroma == 0.0 {
        hue = 0.0;
    } else if red == x_max {
        hue = 60.0 * (green - blue) / chroma;
    } else if green == x_max {
        hue = 60.0 * (2.0 + (blue - red) / chroma);
    } else {
        hue = 60.0 * (4.0 + (red - green) / chroma);
    }

    if hue < 0.0 {
        hue = 360.0 + hue;
    }

    var saturation = 0.0;
    if lightness <= 0.0 || lightness >= 1.0 {
        saturation = 0.0;
    } else {
        saturation = (x_max - lightness) / min(lightness, 1.0 - lightness);
    }

    return vec3<f32>(hue, saturation, lightness);
}

fn map_range(value: f32, ain1: f32, ain2: f32, aout1: f32, aout2: f32) -> f32 {
    return aout1 + (aout2 - aout1) * ((value - ain1) / (ain2 - ain1));
}

fn map_range_vec(value: f32, a: vec2<f32>, b: vec2<f32>) -> f32 {
    return b.x + (b.y - b.x) * ((value - a.x) / (a.y - a.x));
}

fn screen_position(mvp: mat4x4<f32>, offset: vec3<f32>) -> vec3<f32> {
    let screen_ofs = vec4<f32>(
        offset.x,
        offset.y,
        offset.z,
        1.0
    );

    var screen_pos = mvp * screen_ofs;
    screen_pos.y *= -1.0;
    var screen_pos = screen_pos.xyz / screen_pos.w;
    screen_pos = (screen_pos + 1.0) / 2.0;

    return screen_pos;
}

fn pbr_lighting(
    material: StandardMaterial,
    frag_coord: vec4<f32>,
    world_position: vec4<f32>,
    world_normal: vec3<f32>,
    uv: vec2<f32>,
#ifdef VERTEX_TANGENTS
    world_tangent: vec4<f32>,
#endif
    is_front: bool,
) -> vec4<f32> {
    // Calculate lighting via PBR
    var pbr_input: PbrInput = pbr_input_new();

    pbr_input.material = material;
    pbr_input.frag_coord = frag_coord;
    pbr_input.world_position = world_position;
    pbr_input.world_normal = prepare_world_normal(
        world_normal,
        (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u,
        is_front,
    );
    pbr_input.is_orthographic = view.projection[3].w == 1.0;

    pbr_input.N = apply_normal_mapping(
        pbr_input.material.flags,
        pbr_input.world_normal,
#ifdef VERTEX_TANGENTS
#ifdef STANDARDMATERIAL_NORMAL_MAP
        world_tangent,
#endif
#endif
        uv,
    );
    pbr_input.V = calculate_view(world_position, pbr_input.is_orthographic);

    return pbr(pbr_input);
}

fn fresnel_linear(world_position: vec4<f32>, world_normal: vec3<f32>, view: mat4x4<f32>) -> f32 {
    let camera_to_object = normalize(view.w - world_position).xyz;
    return 1.0 - (acos(dot(camera_to_object, world_normal)) / (PI * 0.5));
}

