const VERTECIES = array<vec4<f32>, 6>(
    vec4<f32>(-1., 1., 0., 1.),
    vec4<f32>(-1., -1., 0., 1.),
    vec4<f32>(1., -1., 0., 1.),
    vec4<f32>(1., -1, 0., 1.),
    vec4<f32>(1., 1., 0., 1.),
    vec4<f32>(-1., 1., 0., 1.),
);

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    let pos = VERTECIES[in_vertex_index];
    var output: VertexOutput;
    output.uv = vec2<f32>(pos.x * (8. / 6.), pos.y);
    output.position = pos;
    return output;
}

@group(0) @binding(0) var<uniform> zoom: f32;
@group(0) @binding(1) var<uniform> re_z: f32;
@group(0) @binding(2) var<uniform> im_z: f32;
@group(0) @binding(3) var<uniform> julia: f32;

fn complex_mul(a: vec2<f32>, b: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(a.x * b.x - a.y * b.y, a.x * b.y + a.y * b.x);
}

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4f {
    let pixel_point = vec2<f32>(uv * (1. / zoom));
    let start_point = vec2<f32>(re_z, im_z);
    var z = pixel_point * julia + start_point * (1. - julia);
    let c = pixel_point * (1. - julia) + start_point * julia;
    let max_iters = 100;
    for(var i = 0; i < max_iters; i++) {
        z = complex_mul(z, z) + c;
        if length(z) >= 2. {
            let value = f32(i) / f32(max_iters);
            return vec4f(0.1 + value * 0.9, value, value, 1.0);
        }
    }
    return vec4f(0.0, 0.0, 0.0, 1.0);
}