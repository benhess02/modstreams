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