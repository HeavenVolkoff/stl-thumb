#ifdef GL_ES
precision mediump float; // Precision qualifier for OpenGL ES 2.0
#endif

#if __VERSION__ >= 130
#define VARYING in
#define TEXTURE texture
out vec4 fragColor;
#else
#define VARYING varying
#define TEXTURE texture2D
#endif

// Varyings
VARYING vec2 v_tex_coords;

// Uniforms
uniform vec2 resolution;
uniform sampler2D tex;
uniform int enabled; // Toggle FXAA

// FXAA constants
#define FXAA_REDUCE_MIN   (1.0 / 128.0)
#define FXAA_REDUCE_MUL   (1.0 / 8.0)
#define FXAA_SPAN_MAX     8.0

// FXAA implementation
vec4 fxaa(sampler2D tex, vec2 fragCoord, vec2 resolution,
    vec2 v_rgbNW, vec2 v_rgbNE,
    vec2 v_rgbSW, vec2 v_rgbSE,
    vec2 v_rgbM) {
    // Sample neighboring pixels
    vec2 inverseVP = vec2(1.0 / resolution.x, 1.0 / resolution.y);
    vec3 rgbNW = TEXTURE(tex, v_rgbNW).xyz;
    vec3 rgbNE = TEXTURE(tex, v_rgbNE).xyz;
    vec3 rgbSW = TEXTURE(tex, v_rgbSW).xyz;
    vec3 rgbSE = TEXTURE(tex, v_rgbSE).xyz;
    vec4 texColor = TEXTURE(tex, v_rgbM);
    vec3 rgbM = texColor.xyz;

    // Luminance calculation
    vec3 luma = vec3(0.299, 0.587, 0.114);
    float lumaNW = dot(rgbNW, luma);
    float lumaNE = dot(rgbNE, luma);
    float lumaSW = dot(rgbSW, luma);
    float lumaSE = dot(rgbSE, luma);
    float lumaM = dot(rgbM, luma);
    float lumaMin = min(lumaM, min(min(lumaNW, lumaNE), min(lumaSW, lumaSE)));
    float lumaMax = max(lumaM, max(max(lumaNW, lumaNE), max(lumaSW, lumaSE)));

    // Calculate direction for edge detection
    vec2 dir;
    dir.x = -((lumaNW + lumaNE) - (lumaSW + lumaSE));
    dir.y = ((lumaNW + lumaSW) - (lumaNE + lumaSE));

    float dirReduce = max((lumaNW + lumaNE + lumaSW + lumaSE) *
                (0.25 * FXAA_REDUCE_MUL), FXAA_REDUCE_MIN);

    float rcpDirMin = 1.0 / (min(abs(dir.x), abs(dir.y)) + dirReduce);
    dir = min(vec2(FXAA_SPAN_MAX),
            max(vec2(-FXAA_SPAN_MAX), dir * rcpDirMin)) * inverseVP;

    // Perform blending based on calculated direction
    vec4 rgbA = TEXTURE(tex, fragCoord * inverseVP + dir * (1.0 / 3.0 - 0.5));
    vec4 rgbB = TEXTURE(tex, fragCoord * inverseVP + dir * (2.0 / 3.0 - 0.5));

    // Final color selection based on luminance
    vec3 finalColor = mix(rgbA.rgb, rgbB.rgb, step(lumaMin, dot(rgbA.rgb, luma)));
    return vec4(finalColor, texColor.a);
}

void main() {
    vec4 color;
    if (enabled != 0) {
        vec2 fragCoord = v_tex_coords * resolution;
        vec2 inverseVP = 1.0 / resolution.xy;
        vec2 v_rgbNW = (fragCoord + vec2(-1.0, -1.0)) * inverseVP;
        vec2 v_rgbNE = (fragCoord + vec2(1.0, -1.0)) * inverseVP;
        vec2 v_rgbSW = (fragCoord + vec2(-1.0, 1.0)) * inverseVP;
        vec2 v_rgbSE = (fragCoord + vec2(1.0, 1.0)) * inverseVP;
        vec2 v_rgbM = fragCoord * inverseVP;

        color = fxaa(tex, fragCoord, resolution, v_rgbNW, v_rgbNE, v_rgbSW, v_rgbSE, v_rgbM);
    } else {
        // Bypass FXAA if not enabled
        color = TEXTURE(tex, v_tex_coords);
    }

    #if __VERSION__ >= 130
    fragColor = color;
    #else
    gl_FragColor = color;
    #endif
}
