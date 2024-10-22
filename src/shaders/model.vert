#ifdef GL_ES
precision mediump float; // Set medium precision for OpenGL ES
#endif

#ifdef GL_ES
#define ATTRIBUTE attribute
#define VARYING varying
#else
#if __VERSION__ >= 130
#define ATTRIBUTE in
#define VARYING out
#else
#define ATTRIBUTE attribute
#define VARYING varying
#endif
#endif

// Vertex attributes
ATTRIBUTE vec3 position;
ATTRIBUTE vec3 normal;

// Varyings to pass to fragment shader
VARYING vec3 v_normal;
VARYING vec3 v_position;

// Uniforms
uniform mat4 perspective;
uniform mat4 modelview;

void main() {
    // Transform the vertex position
    vec4 p = modelview * vec4(position, 1.0);
    v_position = p.xyz / p.w;

    // Transform the normal by the upper-left 3x3 part of the modelview matrix
    v_normal = mat3(modelview) * normal;

    // Final vertex position in clip space
    gl_Position = perspective * p;
}
