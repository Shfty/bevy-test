#[function]
fn sdf_3d_position(
    p: vec3<f32>,
) -> f32 {
    return sdf_3d_point(p);
}

// Naive sphere trace
fn sdf_3d_sphere_trace(
    start: f32,
    end: f32,
    eye: vec3<f32>,
    dir: vec3<f32>,
    epsilon: f32,
) -> SdfOutput {
    var out = SdfOutput();
    var t = start;

    for(var i = 0u; i < MAX_MARCHING_STEPS; i++) {
        let p = eye + dir * t;
        let dist = sdf_3d_position(p);

        out.steps += 1u;

        if dist < 0.0 {
            out.hit = true;
            out.dist = t;
            break;
        }

        t += max(epsilon, abs(dist));

        if t > end {
            out.dist = end;
            break;
        }
    }

    return out;
}

// Sphere trace with respect to lipschitz bound
fn sdf_3d_sphere_trace_k(
    start: f32,
    end: f32,
    eye: vec3<f32>,
    dir: vec3<f32>,
    epsilon: f32,
    k: f32,
) -> SdfOutput {
    var out = SdfOutput();

    var t = start;

    for(var i = 0u; i < MAX_MARCHING_STEPS; i++) {
        let p = eye + dir * t;
        let dist = sdf_3d_position(p);

        out.steps += 1u;

        if dist < 0.0 {
            out.hit = true;
            out.dist = t;
            break;
        }

        t += max(epsilon, abs(dist) / k);

        if t > end {
            out.dist = end;
            break;
        }
    }

    return out;
}

// Computes the global lipschitz bound of the falloff function
// e: energy
// R: radius
fn falloff_k(e: f32, r: f32) -> f32
{
    return 1.72 * abs(e) / r;
}

#[function]
fn sdf_3d_raymarch(
    start: f32,
    end: f32,
    eye: vec3<f32>,
    dir: vec3<f32>,
) -> SdfOutput {
    return sdf_3d_sphere_trace_k(
        start,
        end,
        eye,
        dir,
        EPSILON,
        falloff_k(1.0, 3.0) * 3.0,
    );
}

fn sdf_3d_normal_estimate(
    p: vec3<f32>,
    epsilon: f32,
) -> vec3<f32> {
    let k = vec2(1.0, -1.0);
    return normalize(
        k.xyy * sdf_3d_position(p + k.xyy * epsilon) +
        k.yyx * sdf_3d_position(p + k.yyx * epsilon) +
        k.yxy * sdf_3d_position(p + k.yxy * epsilon) +
        k.xxx * sdf_3d_position(p + k.xxx * epsilon)
    );
}

fn sdf_3d_normal_estimate_center_diff(
    p: vec3<f32>,
    epsilon: f32,
) -> vec3<f32> {
    return normalize(
        vec3<f32>(
            sdf_3d_position(vec3<f32>(p.x + epsilon, p.y, p.z)) - sdf_3d_position(vec3<f32>(p.x - epsilon, p.y, p.z)),
            sdf_3d_position(vec3<f32>(p.x, p.y + epsilon, p.z)) - sdf_3d_position(vec3<f32>(p.x, p.y - epsilon, p.z)),
            sdf_3d_position(vec3<f32>(p.x, p.y, p.z + epsilon)) - sdf_3d_position(vec3<f32>(p.x, p.y, p.z - epsilon)),
        )
    );
}

#[function]
fn sdf_3d_normal(
    p: vec3<f32>,
) -> vec3<f32> {
    return sdf_3d_normal_estimate(p, EPSILON);
}

#[function]
fn sdf_3d_uv(
    p: vec3<f32>,
    n: vec3<f32>,
) -> vec2<f32> {
    return vec2<f32>(0.0, 0.0);
}

fn sdf_geometry(
    in: ptr<function, FragmentInput>,
    object_local: bool,
) -> SdfOutput {
    let camera = view.view.w;
    let ray_delta = (*in).world_position - camera;
    let ray_dist = length(ray_delta);
    let ray_direction = normalize(ray_delta);
    var object = mesh.model.w;
    if !object_local {
        object = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    let inv_model_rot = transpose(
        mat3x3<f32>(
            mesh.model.x.xyz,
            mesh.model.y.xyz,
            mesh.model.z.xyz,
        )
    );

    var near = 0.0;
    var far = MAX_MARCHING_DIST;

    if (*in).is_front {
        near = ray_dist;
    } else {
        far = ray_dist;
    }

    let sdf_out = sdf_3d_raymarch(
        near,
        far,
        inv_model_rot * (camera.xyz - object.xyz),
        inv_model_rot * ray_direction.xyz,
    );

    if sdf_out.hit {
        // Update position
        (*in).world_position = camera + ray_direction * max(sdf_out.dist, EPSILON);

        // Calculate normal-friendly rotation matrix
        let inverse_transpose_rot = mat3x3<f32>(
            mesh.inverse_transpose_model.x.xyz,
            mesh.inverse_transpose_model.y.xyz,
            mesh.inverse_transpose_model.z.xyz,
        );

        // Update normal
        (*in).world_normal = inverse_transpose_rot * sdf_3d_normal(
            (*in).world_position.xyz - object.xyz
        );

        // TODO: Update tangent

        // Update UV
        (*in).uv = sdf_3d_uv(
            (*in).world_position.xyz - object.xyz,
            (*in).world_normal,
        );

        // Update depth
        let pos = view.view_proj * (*in).world_position;
        (*in).frag_coord.z = pos.z / pos.w;
    }

    return sdf_out;
}

