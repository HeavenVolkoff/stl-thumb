#ifdef GL_ES
precision mediump float; // Precision qualifier for OpenGL ES 2.0
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

// Vertex attribute inputs
ATTRIBUTE vec2 position;
ATTRIBUTE vec2 i_tex_coords;

// Varying to pass texture coordinates to fragment shader
VARYING vec2 v_tex_coords;

void main() {
    // Pass through texture coordinates
    v_tex_coords = i_tex_coords;

    // Calculate the final position of the vertex in clip space
    gl_Position = vec4(position, 0.0, 1.0);
}
