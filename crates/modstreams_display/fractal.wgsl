@group(0) @binding(0) var<uniform> zoom: f32;
@group(0) @binding(1) var<uniform> x: f32;
@group(0) @binding(2) var<uniform> y: f32;
@group(0) @binding(3) var<uniform> re_z: f32;
@group(0) @binding(4) var<uniform> im_z: f32;
@group(0) @binding(5) var<uniform> julia: f32;

fn complex_mul(a: vec2<f32>, b: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(a.x * b.x - a.y * b.y, a.x * b.y + a.y * b.x);
}

@fragment
fn main(@location(0) uv: vec2<f32>) -> @location(0) vec4f {
    let pixel_point = vec2<f32>(uv * (1. / zoom)) + vec2<f32>(x, y);
    let start_point = vec2<f32>(re_z, im_z);
    var z = pixel_point * julia + start_point * (1. - julia);
    let c = pixel_point * (1. - julia) + start_point * julia;
    let max_iters = 100;
    for(var i = 0; i < max_iters; i++) {
        z = complex_mul(z, z) + c;
        if length(z) >= 2. {
            let value = f32(i) / f32(max_iters);
            return vec4f(value, value, 0.1 + value * 0.9, 1.0);
        }
    }
    return vec4f(0.0, 0.0, 0.0, 1.0);
}