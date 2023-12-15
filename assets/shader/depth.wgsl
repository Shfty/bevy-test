@fragment
fn fragment(
    #import bevy_pbr::mesh_vertex_output
) -> @location(0) vec4<f32> {
    let color = vec3(world_position.z);
    return vec4(color, 1.0);
}
