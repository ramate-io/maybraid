//---------------------------------------------------------
// Chico canopy leaf: object-space leafy breakup on cheap-ball
// cards (planes through a centroid) + vertex sway + PBR.
//
// Holes are glued to each card (object / UV space) so they ride
// with wind. Radial density keeps the hub connected; rims take
// the swiss cheese. Coarse world-space is a weak bias only.
//---------------------------------------------------------

#import bevy_pbr::{
    mesh_bindings::mesh,
    mesh_functions,
    skinning,
    morph::{morph_position, morph_normal, morph_tangent},
    forward_io::Vertex,
    view_transformations::position_world_to_clip,
    mesh_view_bindings::{view, globals, lights},
    pbr_types::{PbrInput, pbr_input_new, STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT},
    pbr_functions as fns,
    pbr_bindings,
}
#import bevy_core_pipeline::tonemapping::tone_mapping

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> base_color: vec4<f32>;

// Interpolators match Bevy's `VertexOutput` locations 0–7, then local pos.
struct LeafVertexOutput {
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
    @location(8) local_pos: vec3<f32>,
}

// --------------------------------------------------------
// Hash / noise
// --------------------------------------------------------

fn hash13(p: vec3<f32>) -> f32 {
    let p3 = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    let d = dot(p3, p3.yzx + vec3<f32>(33.33, 33.33, 33.33));
    return fract((p3.x + p3.y) * p3.z + d);
}

fn value_noise_3d(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f0 = fract(p);
    let f = f0 * f0 * (vec3<f32>(3.0, 3.0, 3.0) - 2.0 * f0);

    let n000 = hash13(i + vec3<f32>(0.0, 0.0, 0.0));
    let n100 = hash13(i + vec3<f32>(1.0, 0.0, 0.0));
    let n010 = hash13(i + vec3<f32>(0.0, 1.0, 0.0));
    let n110 = hash13(i + vec3<f32>(1.0, 1.0, 0.0));

    let n001 = hash13(i + vec3<f32>(0.0, 0.0, 1.0));
    let n101 = hash13(i + vec3<f32>(1.0, 0.0, 1.0));
    let n011 = hash13(i + vec3<f32>(0.0, 1.0, 1.0));
    let n111 = hash13(i + vec3<f32>(1.0, 1.0, 1.0));

    let nx00 = mix(n000, n100, f.x);
    let nx10 = mix(n010, n110, f.x);
    let nx01 = mix(n001, n101, f.x);
    let nx11 = mix(n011, n111, f.x);

    let nxy0 = mix(nx00, nx10, f.y);
    let nxy1 = mix(nx01, nx11, f.y);

    return mix(nxy0, nxy1, f.z);
}

fn fbm_3d(p: vec3<f32>) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    var norm = 0.0;

    for (var i = 0; i < 4; i++) {
        value += value_noise_3d(p * frequency) * amplitude;
        norm += amplitude;
        frequency *= 2.03;
        amplitude *= 0.5;
    }

    return value / norm;
}

fn instance_scale(world_from_local: mat4x4<f32>) -> f32 {
    return max(
        length(world_from_local[0].xyz),
        max(length(world_from_local[1].xyz), length(world_from_local[2].xyz)),
    );
}

fn canopy_sway(
    local_pos: vec3<f32>,
    world_pos: vec3<f32>,
    world_normal: vec3<f32>,
    centroid: vec3<f32>,
    scale: f32,
) -> vec3<f32> {
    let r = min(length(local_pos), 1.0);
    let t = globals.time;
    let phase = dot(centroid, vec3<f32>(0.07, 0.03, 0.11));
    let gust = sin(t * 0.85 + phase) * sin(t * 0.31 + phase * 1.7);
    let flutter = sin(t * 2.15 + local_pos.x * 3.0 + phase);
    let dist = length(world_pos - view.world_position);
    let dist_fade = min(1.0, 24.0 / max(dist, 24.0));
    let wind_dir = vec3<f32>(0.92, 0.08, 0.38);
    var offset = wind_dir * (gust * r * 0.08 * scale);
    offset += world_normal * (flutter * r * 0.028 * scale);
    return offset * dist_fade;
}

#ifdef MORPH_TARGETS
fn morph_vertex(vertex_in: Vertex, instance_index: u32) -> Vertex {
    var vertex = vertex_in;
    let first_vertex = mesh[instance_index].first_vertex_index;
    let vertex_index = vertex.index - first_vertex;

    let weight_count = bevy_pbr::morph::layer_count(instance_index);
    for (var i: u32 = 0u; i < weight_count; i++) {
        let weight = bevy_pbr::morph::weight_at(i, instance_index);
        if weight == 0.0 {
            continue;
        }
        vertex.position += weight * morph_position(vertex_index, i, instance_index);
#ifdef VERTEX_NORMALS
        vertex.normal += weight * morph_normal(vertex_index, i, instance_index);
#endif
#ifdef VERTEX_TANGENTS
        vertex.tangent += vec4(weight * morph_tangent(vertex_index, i, instance_index), 0.0);
#endif
    }
    return vertex;
}
#endif

// --------------------------------------------------------
// Vertex: Bevy mesh transform + rim-weighted wind
// --------------------------------------------------------

@vertex
fn vertex(vertex_no_morph: Vertex) -> LeafVertexOutput {
    var out: LeafVertexOutput;
    out.local_pos = vec3<f32>(0.0, 0.0, 0.0);
    out.world_normal = vec3<f32>(0.0, 1.0, 0.0);

#ifdef MORPH_TARGETS
    var vertex = morph_vertex(vertex_no_morph, vertex_no_morph.instance_index);
#else
    var vertex = vertex_no_morph;
#endif

    let mesh_world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);

#ifdef SKINNED
    var world_from_local = skinning::skin_model(
        vertex.joint_indices,
        vertex.joint_weights,
        vertex_no_morph.instance_index
    );
#else
    var world_from_local = mesh_world_from_local;
#endif

#ifdef VERTEX_NORMALS
#ifdef SKINNED
    out.world_normal = skinning::skin_normals(world_from_local, vertex.normal);
#else
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal,
        vertex_no_morph.instance_index
    );
#endif
#endif

#ifdef VERTEX_POSITIONS
    out.local_pos = vertex.position;
    out.world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(vertex.position, 1.0),
    );
    let centroid = mesh_world_from_local[3].xyz;
    let scale = instance_scale(mesh_world_from_local);
    out.world_position += vec4<f32>(
        canopy_sway(
            vertex.position,
            out.world_position.xyz,
            out.world_normal,
            centroid,
            scale,
        ),
        0.0,
    );
    out.position = position_world_to_clip(out.world_position.xyz);
#endif

#ifdef VERTEX_UVS_A
    out.uv = vertex.uv;
#endif
#ifdef VERTEX_UVS_B
    out.uv_b = vertex.uv_b;
#endif

#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
        world_from_local,
        vertex.tangent,
        vertex_no_morph.instance_index
    );
#endif

#ifdef VERTEX_COLORS
    out.color = vertex.color;
#endif

#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex_no_morph.instance_index;
#endif

#ifdef VISIBILITY_RANGE_DITHER
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        vertex_no_morph.instance_index,
        mesh_world_from_local[3],
    );
#endif

    return out;
}

// --------------------------------------------------------
// Fragment
// --------------------------------------------------------

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: LeafVertexOutput,
) -> @location(0) vec4<f32> {
    var pbr_input: PbrInput = pbr_input_new();

    let world_pos = mesh.world_position.xyz;
    let local_pos = mesh.local_pos;
    let r = saturate(length(local_pos));

    // ----------------------------------------------------
    // Object-space swiss cheese + radial hub
    // ----------------------------------------------------
    // Medium/fine live in kit space so overlapping cheap-ball
    // cards do not share tunnels. UV adds per-plane bites.
    // World coarse is a weak shared bias only — it cannot
    // open a hole by itself.

    let medium = fbm_3d(local_pos * 3.25);
    let fine = fbm_3d(local_pos * 8.50);
#ifdef VERTEX_UVS_A
    let uv_bite = fbm_3d(vec3<f32>(mesh.uv * 6.0, 0.17));
    let hole = medium * 0.48 + fine * 0.22 + uv_bite * 0.30;
#else
    let hole = medium * 0.65 + fine * 0.35;
#endif
    let local_coarse = (fbm_3d(local_pos * 1.15) - 0.5) * 0.10;
    let world_bias = (fbm_3d(world_pos * 0.45) - 0.5) * 0.05;
    let field = hole + local_coarse + world_bias;

    // Hub stays mostly solid; cheese starts well inside the kit so the
    // quad / ico outline is not the silhouette.
    let hub = 1.0 - smoothstep(0.22, 0.62, r);
    let threshold = mix(0.18, 0.46, 1.0 - hub);

    let fw = max(fwidth(field) * 1.35, 0.01);
    let hole_alpha = smoothstep(threshold - fw, threshold + fw, field);

    // Noisy blot radius: thick object-space rim, not a 1px mesh edge.
    let blot = 0.52 + 0.38 * fbm_3d(local_pos * 2.4);
    let rim_w = max(fwidth(r) * 2.0, 0.08);
    let radial_alpha = smoothstep(0.0, rim_w, blot - r);

    let alpha = hole_alpha * radial_alpha;
    if (alpha < 0.08) {
        discard;
    }

    // ----------------------------------------------------
    // Color variation + pigment thinning at hole edges
    // ----------------------------------------------------

    let tint_noise = fbm_3d(local_pos * 1.75);
    let speckle = fbm_3d(local_pos * 12.0);

    let warm_cool = mix(
        vec3<f32>(0.82, 0.95, 0.72),
        vec3<f32>(1.12, 1.04, 0.78),
        tint_noise,
    );

    let brightness = mix(0.78, 1.18, speckle);
    let wash = mix(0.72, 1.0, saturate(min(alpha, radial_alpha) * 1.25));

    let base_rgb = vec3<f32>(
        base_color.x,
        base_color.y,
        base_color.z,
    );

    pbr_input.material.base_color = vec4<f32>(
        base_rgb * warm_cool * brightness * wash,
        alpha,
    );

    // Matte dielectric — default roughness 0.5 reads as plastic in shadow.
    pbr_input.material.perceptual_roughness = 1.0;
    pbr_input.material.metallic = 0.0;
    pbr_input.material.reflectance = vec3<f32>(0.12, 0.12, 0.12);

    // ----------------------------------------------------
    // PBR setup — both sides light toward the camera
    // ----------------------------------------------------

    pbr_input.material.flags = pbr_input.material.flags | STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT;

    pbr_input.frag_coord = mesh.position;
    pbr_input.world_position = mesh.world_position;
    pbr_input.is_orthographic = view.clip_from_view[3].w == 1.0;
    pbr_input.V = fns::calculate_view(mesh.world_position, pbr_input.is_orthographic);

    let prepared_normal = fns::prepare_world_normal(
        mesh.world_normal,
        true,
        is_front,
    );

    // Soften ico/card faceting; wrap keeps N·L from going fully black.
    let n = normalize(mix(prepared_normal, vec3<f32>(0.0, 1.0, 0.0), 0.4));
    pbr_input.world_normal = n;
    pbr_input.N = n;

    let lit_color = fns::apply_pbr_lighting(pbr_input);

    var wrap = 0.5;
    var back = 0.0;
    if (lights.n_directional_lights > 0u) {
        let L = lights.directional_lights[0].direction_to_light;
        wrap = saturate(dot(n, L) * 0.5 + 0.5);
        back = saturate(-dot(n, L));
    }
    let lifted = lit_color.rgb + base_rgb * (0.16 + 0.22 * wrap + 0.28 * back);

    return tone_mapping(vec4<f32>(lifted, alpha), view.color_grading);
}
