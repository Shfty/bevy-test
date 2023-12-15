#define_import_path npbr::texture_interpreter

struct TextureInterpreter {
    tex_head: vec2<i32>,
    tex_lod: i32,
}

fn texture_interpreter_load(
    input: ptr<function, TextureInterpreter>,
    buf: ptr<function, RingBuffer>,
    data: texture_2d<f32>,
) {
    /*
    let t = textureLoad(data, (*input).tex_head, (*input).tex_lod);
    ring_buffer_write_vec4(buf, t);
    */
    let t = textureLoad(data, (*input).tex_head, (*input).tex_lod).r;
    ring_buffer_write(buf, t);
    (*input).tex_head.x += 1;
}

fn texture_interpreter_try_load(
    input: ptr<function, TextureInterpreter>,
    buf: ptr<function, RingBuffer>,
    data: texture_2d<f32>,
    needed: u32,
) {
    loop {
        if ring_buffer_available(buf) >= i32(needed) {
            break;
        }
        texture_interpreter_load(input, buf, data);
    }
}

fn texture_interpreter_f32(
    input: ptr<function, TextureInterpreter>,
    buf: ptr<function, RingBuffer>,
    data: texture_2d<f32>,
) -> f32 {
    texture_interpreter_try_load(input, buf, data, 1u);
    return ring_buffer_read(buf);
}

fn texture_interpreter_u32(
    input: ptr<function, TextureInterpreter>,
    buf: ptr<function, RingBuffer>,
    data: texture_2d<f32>,
) -> u32 {
    texture_interpreter_try_load(input, buf, data, 1u);
    return u32(ring_buffer_read(buf));
}

fn texture_interpreter_vec2f(
    input: ptr<function, TextureInterpreter>,
    buf: ptr<function, RingBuffer>,
    data: texture_2d<f32>,
) -> vec2<f32> {
    texture_interpreter_try_load(input, buf, data, 2u);
    return vec2<f32>(
        ring_buffer_read(buf),
        ring_buffer_read(buf),
    );
}

fn texture_interpreter_vec2u(
    input: ptr<function, TextureInterpreter>,
    buf: ptr<function, RingBuffer>,
    data: texture_2d<f32>,
) -> vec2<u32> {
    let v = texture_interpreter_vec2f(input, buf, data);
    return vec2<u32>(
        u32(v.x),
        u32(v.y),
    );
}

fn texture_interpreter_vec3f(
    input: ptr<function, TextureInterpreter>,
    buf: ptr<function, RingBuffer>,
    data: texture_2d<f32>,
) -> vec3<f32> {
    texture_interpreter_try_load(input, buf, data, 3u);
    return vec3<f32>(
        ring_buffer_read(buf),
        ring_buffer_read(buf),
        ring_buffer_read(buf),
    );
}

fn texture_interpreter_vec3u(
    input: ptr<function, TextureInterpreter>,
    buf: ptr<function, RingBuffer>,
    data: texture_2d<f32>,
) -> vec3<u32> {
    let v = texture_interpreter_vec3f(input, buf, data);
    return vec3<u32>(
        u32(v.x),
        u32(v.y),
        u32(v.z),
    );
}

fn texture_interpreter_vec4f(
    input: ptr<function, TextureInterpreter>,
    buf: ptr<function, RingBuffer>,
    data: texture_2d<f32>,
) -> vec4<f32> {
    texture_interpreter_try_load(input, buf, data, 4u);
    return vec4<f32>(
        ring_buffer_read(buf),
        ring_buffer_read(buf),
        ring_buffer_read(buf),
        ring_buffer_read(buf),
    );
}

fn texture_interpreter_vec4u(
    input: ptr<function, TextureInterpreter>,
    buf: ptr<function, RingBuffer>,
    data: texture_2d<f32>,
) -> vec4<u32> {
    let v = texture_interpreter_vec4f(input, buf, data);
    return vec4<u32>(
        u32(v.x),
        u32(v.y),
        u32(v.z),
        u32(v.w),
    );
}
