//---------------------------------------------------------
// Chico bump outs: terrain-topology displacement, profile
// interpolation, and opaque stochastic fragment dropout.
//---------------------------------------------------------

#import bevy_pbr::{
    forward_io::Vertex,
    mesh_functions,
    mesh_view_bindings::view,
    pbr_functions as fns,
    pbr_types::{PbrInput, pbr_input_new},
    view_transformations::position_world_to_clip,
}
#import bevy_core_pipeline::tonemapping::tone_mapping

struct BumpOutUniform {
    colors: array<vec4<f32>, 3>,
    noise: vec4<f32>,
    style: vec4<f32>,
    density_rows: array<vec4<f32>, 3>,
    height_rows: array<vec4<f32>, 3>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> bump: BumpOutUniform;

struct BumpOutVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
#ifdef VERTEX_UVS_A
    @location(2) uv: vec2<f32>,
#endif
#ifdef VERTEX_UVS_B
    @location(3) uv_b: vec2<f32>,
#endif
#ifdef VERTEX_TANGENTS
    @location(4) world_tangent: vec4<f32>,
#endif
#ifdef VERTEX_COLORS
    @location(5) color: vec4<f32>,
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    @location(6) @interpolate(flat) instance_index: u32,
#endif
#ifdef VISIBILITY_RANGE_DITHER
    @location(7) @interpolate(flat) visibility_range_dither: i32,
#endif
    @location(8) density: f32,
}

fn saturate(x: f32) -> f32 {
    return clamp(x, 0.0, 1.0);
}

fn hash21(p: vec2<f32>) -> f32 {
    let q = vec3<f32>(p.x, p.y, p.x) * vec3<f32>(0.1031, 0.1030, 0.0973);
    let f = fract(q);
    let d = dot(f, f.yzx + vec3<f32>(33.33));
    return fract((f.x + f.y) * f.z + d);
}

fn value_noise_2d(p: vec2<f32>, seed: f32) -> f32 {
    let i = floor(p);
    let f0 = fract(p);
    let f = f0 * f0 * (vec2<f32>(3.0) - 2.0 * f0);
    let s = vec2<f32>(seed * 17.13, seed * 31.71);
    let a = hash21(i + vec2<f32>(0.0, 0.0) + s);
    let b = hash21(i + vec2<f32>(1.0, 0.0) + s);
    let c = hash21(i + vec2<f32>(0.0, 1.0) + s);
    let d = hash21(i + vec2<f32>(1.0, 1.0) + s);
    return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}

fn sample_row(row: vec4<f32>, x: f32) -> f32 {
    if (x < 1.0) {
        return mix(row.x, row.y, x);
    }
    return mix(row.y, row.z, x - 1.0);
}

fn sample_grid(rows: array<vec4<f32>, 3>, uv: vec2<f32>) -> f32 {
    // UV 0/1 lies halfway between this cell's center sample and its neighbor's center sample.
    // Adjacent cells carrying reciprocal neighborhoods therefore agree on their shared edge.
    let p = clamp(uv + vec2<f32>(0.5), vec2<f32>(0.0), vec2<f32>(2.0));
    if (p.y < 1.0) {
        return mix(sample_row(rows[0], p.x), sample_row(rows[1], p.x), p.y);
    }
    return mix(sample_row(rows[1], p.x), sample_row(rows[2], p.x), p.y - 1.0);
}

@vertex
fn vertex(vertex: Vertex) -> BumpOutVertexOutput {
    var out: BumpOutVertexOutput;
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    var profile_uv = vec2<f32>(0.5);

#ifdef VERTEX_UVS_A
    profile_uv = vertex.uv;
    out.uv = vertex.uv;
#endif
#ifdef VERTEX_UVS_B
    out.uv_b = vertex.uv_b;
#endif

    out.world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(vertex.position, 1.0),
    );
    out.density = saturate(sample_grid(bump.density_rows, profile_uv));
    let profile_height = sample_grid(bump.height_rows, profile_uv);
    let displacement_noise = value_noise_2d(
        out.world_position.xz * bump.noise.x,
        bump.noise.z,
    );
    out.world_position.y += profile_height
        + (displacement_noise - 0.5) * 2.0 * bump.noise.y * out.density;
    out.position = position_world_to_clip(out.world_position.xyz);

#ifdef VERTEX_NORMALS
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal,
        vertex.instance_index,
    );
#else
    out.world_normal = vec3<f32>(0.0, 1.0, 0.0);
#endif
#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
        world_from_local,
        vertex.tangent,
        vertex.instance_index,
    );
#endif
#ifdef VERTEX_COLORS
    out.color = vertex.color;
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex.instance_index;
#endif
#ifdef VISIBILITY_RANGE_DITHER
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        vertex.instance_index,
        world_from_local[3],
    );
#endif
    return out;
}

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: BumpOutVertexOutput,
) -> @location(0) vec4<f32> {
    let world = mesh.world_position.xyz;
    let coverage_noise = value_noise_2d(world.xz * bump.noise.x * 0.73, bump.noise.z + 91.7);
    let softness = bump.style.x;
    let reject_probability = smoothstep(
        mesh.density - softness,
        mesh.density + softness,
        coverage_noise,
    );
    let dither = hash21(floor(world.xz * bump.noise.x * 12.0) + vec2<f32>(bump.noise.z));
    if (dither < reject_probability) {
        discard;
    }

    let tint_noise = value_noise_2d(world.xz * bump.noise.x * 0.31, bump.noise.z + 37.0);
    let color01 = mix(bump.colors[0].rgb, bump.colors[1].rgb, saturate(tint_noise * 2.0));
    let color12 = mix(bump.colors[1].rgb, bump.colors[2].rgb, saturate(tint_noise * 2.0 - 1.0));
    let albedo = mix(color01, color12, step(0.5, tint_noise));

    var geometric_normal = normalize(cross(dpdx(world), dpdy(world)));
    if (dot(geometric_normal, mesh.world_normal) < 0.0) {
        geometric_normal = -geometric_normal;
    }
    let prepared = fns::prepare_world_normal(geometric_normal, true, is_front);
    let normal = normalize(mix(prepared, vec3<f32>(0.0, 1.0, 0.0), bump.style.z));

    var pbr_input: PbrInput = pbr_input_new();
    pbr_input.material.base_color = vec4<f32>(albedo, 1.0);
    pbr_input.material.metallic = 0.0;
    pbr_input.material.perceptual_roughness = bump.style.y;
    pbr_input.frag_coord = mesh.position;
    pbr_input.world_position = mesh.world_position;
    pbr_input.world_normal = normal;
    pbr_input.is_orthographic = view.clip_from_view[3].w == 1.0;
    pbr_input.N = normal;
    pbr_input.V = fns::calculate_view(mesh.world_position, pbr_input.is_orthographic);

    let lit = fns::apply_pbr_lighting(pbr_input);
    return tone_mapping(vec4<f32>(lit.rgb, 1.0), view.color_grading);
}
