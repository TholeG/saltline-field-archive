#version 300 es
precision highp float;
uniform vec4 eye_time;
in vec3 world_position;
in vec3 surface_tint;
flat in float surface_material;
out vec4 pixel;

void main() {
    vec3 haze = vec3(0.54, 0.66, 0.66);
    vec3 color = surface_tint;
    vec2 xy = world_position.xz;
    if (surface_material == 1.0) {
        float low = sin(xy.x * 0.48 + xy.y * 2.1 + eye_time.w * 0.64);
        float high = sin(xy.x * 1.51 - xy.y * 3.71 + eye_time.w * 0.83);
        float fine = sin(xy.x * 4.43 + xy.y * 6.15 - eye_time.w * 1.16);
        float shimmer = 0.115 * smoothstep(0.65, 0.96, low * 0.55 + high * 0.30 + fine * 0.15);
        float swell = 0.034 * low + 0.018 * high + 0.007 * fine;
        float facing = max(0.0, normalize(eye_time.xyz - world_position).y);
        float reflection = (1.0 - facing) * (1.0 - facing);
        color = mix(vec3(0.23, 0.47, 0.47), vec3(0.11, 0.35, 0.43), smoothstep(-4.0, 21.0, xy.x));
        color = mix(color, vec3(0.56, 0.66, 0.64), reflection * 0.55);
        color += vec3(swell + shimmer * 1.0, swell + shimmer * 0.89, swell + shimmer * 0.64);
    } else if (surface_material == 2.0) {
        color += vec3(0.05 + sin(eye_time.w * 2.0) * 0.025);
    } else if (surface_material == 3.0) {
        float grain = sin(xy.x * 32.0 + sin(xy.y * 2.2) * 1.7 + world_position.y * 3.0);
        float dark_fibers = smoothstep(0.50, 0.96, sin(xy.x * 87.0 + grain * 2.2));
        color *= 0.98 + grain * 0.04 - dark_fibers * 0.11;
    } else if (surface_material == 4.0) {
        float speckle = fract(sin(dot(floor(xy * 21.0), vec2(127.1, 311.7))) * 43758.5453);
        float weather = sin(xy.x * 1.16 + sin(xy.y * 0.84) * 3.0);
        color *= 0.96 + speckle * 0.06 + weather * 0.045;
    }
    float fog = smoothstep(17.0, 74.0, length(world_position - eye_time.xyz)) * 0.78;
    pixel = vec4(clamp(mix(color, haze, fog), vec3(0), vec3(1)), 1.0);
}
