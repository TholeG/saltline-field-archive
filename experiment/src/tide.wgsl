struct Camera {
    view_projection: mat4x4<f32>,
    eye_time: vec4<f32>,
    settings: vec4<f32>,
}

@group(0) @binding(0) var<uniform> camera: Camera;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) tint: vec3<f32>,
    @location(2) @interpolate(flat) material: f32,
}

@vertex
fn vs_main(@location(0) position: vec3<f32>, @location(1) tint: vec3<f32>, @location(2) material: f32) -> VertexOutput {
    var result: VertexOutput;
    result.clip_position = camera.view_projection * vec4<f32>(position, 1.0);
    result.world_position = position;
    result.tint = tint;
    result.material = material;
    return result;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let haze = vec3<f32>(0.54, 0.66, 0.66);
    var color = input.tint;
    let xy = input.world_position.xz;
    if input.material == 1.0 {
        let t = camera.eye_time.w;
        let low = sin(xy.x * 0.48 + xy.y * 2.1 + t * 0.64);
        let high = sin(xy.x * 1.51 - xy.y * 3.71 + t * 0.83);
        let fine = sin(xy.x * 4.43 + xy.y * 6.15 - t * 1.16);
        let shimmer = 0.115 * smoothstep(0.65, 0.96, low * 0.55 + high * 0.30 + fine * 0.15);
        let swell = 0.034 * low + 0.018 * high + 0.007 * fine;
        let creek = smoothstep(-4.0, 21.0, xy.x);
        let facing = max(0.0, normalize(camera.eye_time.xyz - input.world_position).y);
        let reflection = (1.0 - facing) * (1.0 - facing);
        color = mix(vec3<f32>(0.23, 0.47, 0.47), vec3<f32>(0.11, 0.35, 0.43), creek);
        color = mix(color, vec3<f32>(0.56, 0.66, 0.64), reflection * 0.55);
        color += vec3<f32>(swell + shimmer * 1.0, swell + shimmer * 0.89, swell + shimmer * 0.64);
    } else if input.material == 2.0 {
        let bloom = 0.05 + sin(camera.eye_time.w * 2.0) * 0.025;
        color += vec3<f32>(bloom);
    } else if input.material == 3.0 {
        let grain = sin(xy.x * 32.0 + sin(xy.y * 2.2) * 1.7 + input.world_position.y * 3.0);
        let dark_fibers = smoothstep(0.50, 0.96, sin(xy.x * 87.0 + grain * 2.2));
        color *= 0.98 + grain * 0.04 - dark_fibers * 0.11;
    } else if input.material == 4.0 {
        let speckle = fract(sin(dot(floor(xy * 21.0), vec2<f32>(127.1, 311.7))) * 43758.5453);
        let weather = sin(xy.x * 1.16 + sin(xy.y * 0.84) * 3.0);
        color *= 0.96 + speckle * 0.06 + weather * 0.045;
    }
    let distance = length(input.world_position - camera.eye_time.xyz);
    let fog = smoothstep(17.0, 74.0, distance) * 0.78;
    color = clamp(mix(color, haze, fog), vec3<f32>(0.0), vec3<f32>(1.0));
    if camera.settings.x > 0.5 {
        let blend = step(vec3<f32>(0.04045), color);
        color = mix(color / 12.92, pow((color + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4)), blend);
    }
    return vec4<f32>(color, 1.0);
}
