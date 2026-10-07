#import bevy_pbr::forward_io::VertexOutput

// Bevy 0.19.1 Material bindings: https://github.com/bevyengine/bevy/blob/v0.19.1/examples/shader/shader_material.rs
// x: brightness (dimensionless, 0.2..2); y: scanlines (dimensionless, 0..0.4).
// z: effective curvature (dimensionless, 0..0.16); w: effect mode (0 bypass, 1 enabled).
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> settings: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> tint: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var screen: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var screen_sampler: sampler;

// Display UV uses u=0 left, v=0 top. This exact radial sampling map is shared with mapping.rs.
fn sample_uv(uv: vec2<f32>) -> vec2<f32> {
    let centered = uv * 2.0 - vec2<f32>(1.0);
    return vec2<f32>(0.5) + centered * 0.5 * (1.0 + settings.z * dot(centered, centered));
}
fn on_glass(uv: vec2<f32>) -> bool {
    let radius = 0.045;
    let nearest = clamp(uv, vec2<f32>(radius), vec2<f32>(1.0 - radius));
    return distance(uv, nearest) <= radius;
}
@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let uv = sample_uv(mesh.uv);
    if !on_glass(mesh.uv) || any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) {
        return vec4<f32>(0.002, 0.004, 0.003, 1.0);
    }
    let original = textureSample(screen, screen_sampler, uv);
    if settings.w < 0.5 { return vec4<f32>(original.rgb, 1.0); }
    // Texture dimensions are source pixels; the static RGB shift is less than one source pixel.
    let dimensions = vec2<f32>(textureDimensions(screen));
    let offset = vec2<f32>(0.65 / dimensions.x, 0.0);
    let red = textureSample(screen, screen_sampler, uv + offset).r;
    let blue = textureSample(screen, screen_sampler, uv - offset).b;
    let row = uv.y * dimensions.y;
    let scan = 1.0 - settings.y * (0.5 + 0.5 * cos(row * 3.14159265));
    let center = mesh.uv * 2.0 - vec2<f32>(1.0);
    let vignette = 1.0 - 0.12 * dot(center, center);
    let color = vec3<f32>(red, original.g, blue) * tint.rgb * settings.x * scan * vignette;
    return vec4<f32>(color, 1.0);
}
