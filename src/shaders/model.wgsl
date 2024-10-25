struct VertBindings {
    perspective: mat4x4<f32>,
    modelview: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> v_bindings: VertBindings;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>, // Clip-space position
    @location(0) v_normal: vec3<f32>, // Transformed normal
    @location(1) v_position: vec3<f32>, // World-space position
}

@vertex
fn vert_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;

    // Transform the position to world space
    let world_position = v_bindings.modelview * vec4<f32>(input.position, 1.0);
    output.v_position = world_position.xyz / world_position.w;

    // Transform the normal to world space
    let normal_matrix = mat3x3<f32>(v_bindings.modelview[0].xyz, v_bindings.modelview[1].xyz, v_bindings.modelview[2].xyz);
    output.v_normal = normalize(normal_matrix * input.normal);

    // Calculate the final clip-space position
    output.position = v_bindings.perspective * world_position;

    return output;
}

struct FragBindings {
    /* @offset(0) */
    light_direction: vec3<f32>,
    /* @offset(16) */
    ambient_color: vec3<f32>,
    /* @offset(32) */
    diffuse_color: vec3<f32>,
    /* @offset(48) */
    specular_color: vec3<f32>,
}

@group(0) @binding(1) var<uniform> f_bindings: FragBindings;

// Fragment shader main function
@fragment
fn frag_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Normalize the light direction vectors
    let light_direction = normalize(f_bindings.light_direction);

    // Diffuse lighting (Lambertian reflection)
    let diffuse = max(dot(in.v_normal, light_direction), 0.0);

    // Camera direction (assuming camera is at the origin)
    let camera_dir = normalize(-in.v_position);

    // Half-vector between the light and the camera directions
    let half_direction = normalize(light_direction + camera_dir);

    // Specular reflection (Blinn-Phong model)
    let shininess = 128.0 * 32.0; // Adjust shininess for desired specular highlight
    let specular = pow(max(dot(half_direction, in.v_normal), 0.0), shininess);

    // Combine ambient, diffuse, and specular lighting
    let color = f_bindings.ambient_color + diffuse * f_bindings.diffuse_color + specular * f_bindings.specular_color;

    // Apply gamma correction
    let gamma = 0.5; // TODO: Metal requires this correction, test other backends
    let corrected_color = pow(color, vec3<f32>(1.0 / gamma));

    return vec4<f32>(clamp(corrected_color, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
