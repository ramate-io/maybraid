#import bevy_pbr::view_transformations::position_world_to_clip

struct Vertex {
	@location(0) position: vec3<f32>,
	@location(1) normal: vec3<f32>,
	@location(8) i_col0: vec4<f32>,
	@location(9) i_col1: vec4<f32>,
	@location(10) i_col2: vec4<f32>,
	@location(11) i_col3: vec4<f32>,
	@location(12) i_color: vec4<f32>,
};

struct VertexOutput {
	@builtin(position) clip_position: vec4<f32>,
	@location(0) color: vec4<f32>,
	@location(1) world_normal: vec3<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
	let instance = mat4x4<f32>(vertex.i_col0, vertex.i_col1, vertex.i_col2, vertex.i_col3);
	let world = instance * vec4<f32>(vertex.position, 1.0);
	var out: VertexOutput;
	// Instance is already world (grove host is identity). Do not use
	// `get_world_from_local(0u)`: that indexes `mesh[0]`, some other entity.
	out.clip_position = position_world_to_clip(world.xyz);
	out.world_normal = normalize((instance * vec4<f32>(vertex.normal, 0.0)).xyz);
	out.color = vertex.i_color;
	return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
	let light = normalize(vec3<f32>(0.35, 0.9, 0.25));
	let ndotl = max(dot(in.world_normal, light), 0.18);
	return vec4<f32>(in.color.rgb * ndotl, in.color.a);
}
