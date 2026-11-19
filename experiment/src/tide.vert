#version 300 es
layout(location = 0) in vec3 position;
layout(location = 1) in vec3 tint;
layout(location = 2) in float material;
uniform mat4 view_projection;
out vec3 world_position;
out vec3 surface_tint;
flat out float surface_material;

void main() {
    vec4 clip = view_projection * vec4(position, 1.0);
    gl_Position = vec4(clip.xy, 2.0 * clip.z - clip.w, clip.w);
    world_position = position;
    surface_tint = tint;
    surface_material = material;
}
