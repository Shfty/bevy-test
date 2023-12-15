#define_import_path npbr::ring_buffer

struct RingBuffer {
    buf: array<f32, 16>,
    write_ptr: u32,
    read_ptr: u32,
}

fn ring_buffer_write_pointer(buf: ptr<function, RingBuffer>) -> u32 {
    return (*buf).write_ptr % 32u;
}

fn ring_buffer_read_pointer(buf: ptr<function, RingBuffer>) -> u32 {
    return (*buf).read_ptr % 32u;
}

fn ring_buffer_available(buf: ptr<function, RingBuffer>) -> i32 {
    return i32((*buf).write_ptr) - i32((*buf).read_ptr);
}

fn ring_buffer_write(buf: ptr<function, RingBuffer>, v: f32) {
    (*buf).buf[ring_buffer_write_pointer(buf)] = v;
    (*buf).write_ptr += 1u;
}

fn ring_buffer_read(buf: ptr<function, RingBuffer>) -> f32 {
    if ring_buffer_available(buf) < 1 {
        return 0.0;
    }

    let v = (*buf).buf[ring_buffer_read_pointer(buf)];
    (*buf).read_ptr += 1u;
    return v;
}

fn ring_buffer_write_vec2(buf: ptr<function, RingBuffer>, v: vec2<f32>) {
    for(var i = 0; i < 2; i++) {
        ring_buffer_write(buf, v[i]);
    }
}

fn ring_buffer_read_vec2(buf: ptr<function, RingBuffer>) -> vec2<f32> {
    var v = vec2<f32>();
    for(var i = 0; i < 2; i++) {
        v[i] = ring_buffer_read(buf);
    }
    return v;
}

fn ring_buffer_write_vec3(buf: ptr<function, RingBuffer>, v: vec3<f32>) {
    for(var i = 0; i < 3; i++) {
        ring_buffer_write(buf, v[i]);
    }
}

fn ring_buffer_read_vec3(buf: ptr<function, RingBuffer>) -> vec3<f32> {
    var v = vec3<f32>();
    for(var i = 0; i < 3; i++) {
        v[i] = ring_buffer_read(buf);
    }
    return v;
}

fn ring_buffer_write_vec4(buf: ptr<function, RingBuffer>, v: vec4<f32>) {
    for(var i = 0; i < 4; i++) {
        ring_buffer_write(buf, v[i]);
    }
}

fn ring_buffer_read_vec4(buf: ptr<function, RingBuffer>) -> vec4<f32> {
    var v = vec4<f32>();
    for(var i = 0; i < 4; i++) {
        v[i] = ring_buffer_read(buf);
    }
    return v;
}

