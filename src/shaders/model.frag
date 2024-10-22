#ifdef GL_ES
precision mediump float; // Set medium precision for OpenGL ES
#endif

#if __VERSION__ >= 130
#define VARYING in
out vec4 fragColor;
#else
#define VARYING varying
#endif

VARYING vec3 v_normal;
VARYING vec3 v_position;

// Uniforms
uniform vec3 u_light;
uniform vec3 ambient_color;
uniform vec3 diffuse_color;
uniform vec3 specular_color;

void main() {
    // Normalize inputs
    vec3 N = normalize(v_normal); // Normal vector
    vec3 L = normalize(u_light); // Light direction

    // Diffuse lighting: Lambertian reflectance
    float diffuse = max(dot(N, L), 0.0);

    // Specular lighting (Phong reflection model)
    vec3 V = normalize(-v_position); // View direction
    vec3 H = normalize(L + V); // Halfway vector
    float specular = pow(max(dot(N, H), 0.0), 16.0);

    // Alternative specular method (commented out)
    // vec3 R = reflect(-L, N);                  // Reflection vector
    // float cosAlpha = max(dot(V, R), 0.0);
    // float specular = pow(cosAlpha, 4.0);

    // Final color output
    vec3 final_color = ambient_color + diffuse * diffuse_color + specular * specular_color;

    #if __VERSION__ >= 130
    fragColor = vec4(final_color, 1.0);
    #else
    gl_FragColor = vec4(final_color, 1.0);
    #endif
}
