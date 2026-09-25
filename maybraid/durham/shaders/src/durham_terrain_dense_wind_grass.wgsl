//---------------------------------------------------------
// Durham terrain: procedural crack POM plus dense, vibrant, wind-animated grass.
// Procedural recessed cracks with slope-aware, refined world-space POM.
// Preserves original material bindings, palette, fog, and prepass outlines.
// Color-only relief: no silhouette change, displaced depth, or self-shadow ray.
// Local planar approximation; crossfades on cliffs and at grazing angles.
// Dense grass uses two compact blade fields, time-driven bend, and turf LOD.
// Finite stepping can miss very thin blade tips; no temporal jitter is used.
// GRASS_FAR_COVERAGE and softened blade normals are artistic LOD approximations.
//---------------------------------------------------------
#import bevy_pbr::{
    forward_io::VertexOutput,
    mesh_view_bindings::{view, lights, globals},
    prepass_utils::prepass_depth,
    pbr_types::{PbrInput, pbr_input_new, STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT},
    pbr_functions as fns,
}
#import bevy_core_pipeline::tonemapping::tone_mapping
#ifdef DISTANCE_FOG
#import bevy_pbr::mesh_view_bindings::fog
#endif

fn with_distance_fog(color: vec4<f32>, world_position: vec3<f32>, frag_xy: vec2<f32>) -> vec4<f32> {
#ifdef DISTANCE_FOG
    return fns::apply_fog(fog, color, world_position, view.world_position.xyz, frag_xy);
#else
    return color;
#endif
}

struct DurhamSwatch {
    left: vec4<f32>,
    right: vec4<f32>,
    swatch_meta: vec4<f32>,
}

struct DurhamTerrainBand {
    config: vec4<f32>,
    band_scale: vec4<f32>,
    swatches: array<DurhamSwatch, 8>,
}

struct DurhamTerrainNoise {
    regional_blend: vec4<f32>,
    global_seed: vec4<f32>,
    bands: array<DurhamTerrainBand, 4>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> terrain_noise: DurhamTerrainNoise;

@group(#{MATERIAL_BIND_GROUP}) @binding(1)
var<uniform> style_params: vec4<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(2)
var<uniform> base_color: vec4<f32>;

fn saturate(x: f32) -> f32 {
    return clamp(x, 0.0, 1.0);
}

fn spread_noise(t: f32, amount: f32) -> f32 {
    let x = saturate(t);
    return saturate(0.5 + (x - 0.5) * amount);
}

fn hash21(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453123);
}

fn value_noise_2d(p: vec2<f32>, seed: f32) -> f32 {
    let i = floor(p);
    let nf = fract(p);
    let u = nf * nf * (3.0 - 2.0 * nf);
    let s = vec2<f32>(seed * 17.13, seed * 31.71);

    let a = hash21(i + vec2<f32>(0.0, 0.0) + s);
    let b = hash21(i + vec2<f32>(1.0, 0.0) + s);
    let c = hash21(i + vec2<f32>(0.0, 1.0) + s);
    let d = hash21(i + vec2<f32>(1.0, 1.0) + s);

    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn fbm_2d(p: vec2<f32>, seed: f32, amp: f32, freq: f32) -> f32 {
    var sum = 0.0;
    var a = amp;
    var f = freq;

    for (var octave = 0; octave < 2; octave = octave + 1) {
        sum += value_noise_2d(p * f, seed + f32(octave) * 19.17) * a;
        f *= 2.03;
        a *= 0.5;
    }

    // Preserve approximately the four-octave amplitude while evaluating half
    // as many frequencies.
    return saturate(sum * 1.25);
}

fn domain_warp_offset_2d(p: vec2<f32>, seed: f32, amp: f32, freq: f32) -> vec2<f32> {
    let qx = fbm_2d(p + vec2<f32>(17.1, 3.7), seed + 10.0, amp, freq * 0.35);
    let qy = fbm_2d(p + vec2<f32>(8.3, 29.4), seed + 20.0, amp, freq * 0.35);

    return (vec2<f32>(qx, qy) - vec2<f32>(0.5)) * 260.0;
}

fn palette_noise_2d(p: vec2<f32>, seed: f32, amp: f32, freq: f32) -> f32 {
    let warp = domain_warp_offset_2d(p, seed, amp, freq);
    let n = fbm_2d(p + warp, seed + 3.0, amp, freq);

    return spread_noise(n, 1.75);
}

fn swatch_linear(sw: DurhamSwatch, along: f32) -> vec3<f32> {
    return mix(sw.left.xyz, sw.right.xyz, saturate(along));
}

/// Half-width in **continuous** `u = t*8` around each integer seam for joint-only blending.
const SWATCH_INDEX_SOFTNESS: f32 = 0.12;

fn swatch_sample(t_noise: f32, band: DurhamTerrainBand) -> vec3<f32> {
    let u = min(saturate(t_noise) * 8.0, 8.0 - 1e-4);
    let e = SWATCH_INDEX_SOFTNESS;
    let i = min(i32(floor(u)), 7);
    let f = fract(u);

    // Only the two seams adjacent to this cell can be inside the soft interval.
    if (i > 0 && f <= e) {
        let t = smoothstep(0.0, 1.0, (f + e) / (2.0 * e));
        return mix(
            swatch_linear(band.swatches[i - 1], 1.0),
            swatch_linear(band.swatches[i], 0.0),
            t
        );
    }
    if (i < 7 && f >= 1.0 - e) {
        let t = smoothstep(0.0, 1.0, (f - 1.0 + e) / (2.0 * e));
        return mix(
            swatch_linear(band.swatches[i], 1.0),
            swatch_linear(band.swatches[i + 1], 0.0),
            t
        );
    }

    return swatch_linear(band.swatches[i], f);
}

fn band_color(p: vec2<f32>, band: DurhamTerrainBand) -> vec3<f32> {
    let seed = band.config.x;
    let t = palette_noise_2d(p, seed, band.band_scale.y, band.band_scale.x);
    return swatch_sample(t, band);
}

const WORLD_COLOR_SHIFT: vec2<f32> = vec2<f32>(1298.0, 18229.0);

fn ground_color_at(p: vec2<f32>) -> vec3<f32> {
    var acc = vec3<f32>(0.0);
    var wsum = 0.0;

    for (var bi = 0; bi < 4; bi = bi + 1) {
        let band = terrain_noise.bands[bi];
        let w = spread_noise(max(band.band_scale.z, 0.0), 1.35);

        acc += band_color(p, band) * w;
        wsum += w;
    }

    return acc / max(wsum, 1e-6);
}

// Relief is recessed below the original mesh. All distances are metres.
// World XZ is suitable for terrain, not cliffs/overhangs. With floating-origin
// rendering, add a stable absolute-world offset to relief coordinates.
const CRACK_CELL_M: f32 = 1.8;
const CRACK_WIDTH_M: f32 = 0.10;
const CRACK_DEPTH_M: f32 = 0.18;
const CRACK_DARK: f32 = 0.55;
const CRACK_RANGE_START_M: f32 = 35.0;
const CRACK_RANGE_END_M: f32 = 50.0;
const CRACK_NORMAL_EPS_M: f32 = 0.008;
const CRACK_MIN_STEPS: i32 = 24;
const CRACK_MAX_STEPS: i32 = 96;
const CRACK_REFINE_STEPS: i32 = 5;

// Integer lattice hashing avoids the sin-based hash used by the original
// palette. Negative cell coordinates are reinterpreted, not clamped to zero.
fn relief_hash(p: vec2<i32>, seed: u32) -> u32 {
    var v = bitcast<u32>(p.x) * 0x8da6b343u
          ^ bitcast<u32>(p.y) * 0xd8163841u ^ seed;
    v = (v ^ (v >> 16u)) * 0x7feb352du;
    v = (v ^ (v >> 15u)) * 0x846ca68bu;
    return v ^ (v >> 16u);
}

fn relief_random(p: vec2<i32>, seed: u32) -> f32 {
    return f32(relief_hash(p, seed) >> 8u) * (1.0 / 16777216.0);
}

// F2-F1 provides a continuous approximate cell-boundary distance.
// Broad plateaus, narrow recessed boundaries; no texture required.
fn relief_height(p: vec2<f32>) -> f32 {
    let domain = p / CRACK_CELL_M;
    let cell = vec2<i32>(floor(domain));
    let local = fract(domain);
    var first = 1e10;
    var second = 1e10;
    for (var z = -1; z <= 1; z = z + 1) {
        for (var x = -1; x <= 1; x = x + 1) {
            let offset = vec2<i32>(x, z);
            let id = cell + offset;
            // Limited jitter keeps the local neighborhood well behaved.
            let jitter = vec2<f32>(relief_random(id, 71u),
                                   relief_random(id, 193u));
            let feature = vec2<f32>(offset) + vec2<f32>(0.3) + 0.4 * jitter;
            let delta = feature - local;
            let d2 = dot(delta, delta);
            if (d2 < first) {
                second = first;
                first = d2;
            } else {
                second = min(second, d2);
            }
        }
    }
    let edge = (sqrt(second) - sqrt(first)) * CRACK_CELL_M;
    return smoothstep(0.0, CRACK_WIDTH_M, edge);
}

struct ReliefHit {
    offset: vec3<f32>,
    height: f32,
}

// Solve N.dot(Q-P) + D*(1-h(Q.xz)) = 0 on Q=P-V*s.
// N is the unsoftened macro normal. D is fixed, never distance-scaled.
// Grazing rays are clamped; the caller fades before dot(N,V) reaches zero.
fn trace_relief(p: vec2<f32>, N: vec3<f32>, V: vec3<f32>) -> ReliefHit {
    // Limit ray travel at extreme grazing angles.
    let ray = -V * (
        CRACK_DEPTH_M / max(dot(N, V), 0.06)
    );
    let horizontal_travel = length(ray.xz);
    // Attempt at least four samples across the nominal crack width.
    // The finite maximum remains a quality/performance tradeoff.
    let requested = ceil(horizontal_travel / (CRACK_WIDTH_M * 0.25));
    let steps = i32(clamp(requested, f32(CRACK_MIN_STEPS), f32(CRACK_MAX_STEPS)));
    let h0 = relief_height(p);
    if (h0 >= 1.0) {
        return ReliefHit(vec3<f32>(0.0), h0);
    }

    var lo = 0.0;
    var hi = 1.0;
    var f_lo = h0 - 1.0;
    var f_hi = 0.0;
    for (var i = 1; i <= CRACK_MAX_STEPS; i = i + 1) {
        if (i > steps) { break; }
        let t = f32(i) / f32(steps);
        let f = relief_height(p + ray.xz * t) - (1.0 - t);
        if (f >= 0.0) {
            hi = t;
            f_hi = f;
            break;
        }
        lo = t;
        f_lo = f;
    }
    for (var i = 0; i < CRACK_REFINE_STEPS; i = i + 1) {
        let mid = 0.5 * (lo + hi);
        let f = relief_height(p + ray.xz * mid) - (1.0 - mid);
        if (f >= 0.0) {
            hi = mid;
            f_hi = f;
        } else {
            lo = mid;
            f_lo = f;
        }
    }
    let fraction = clamp(-f_lo / max(f_hi - f_lo, 1e-7), 0.0, 1.0);
    let t = mix(lo, hi, fraction);
    let offset = ray * t;
    return ReliefHit(offset, relief_height(p + offset.xz));
}

fn relief_gradient(p: vec2<f32>) -> vec2<f32> {
    let e = CRACK_NORMAL_EPS_M;
    let dx = relief_height(p + vec2<f32>(e, 0.0))
           - relief_height(p - vec2<f32>(e, 0.0));
    let dz = relief_height(p + vec2<f32>(0.0, e))
           - relief_height(p - vec2<f32>(0.0, e));
    return vec2<f32>(dx, dz) / (2.0 * e);
}

// --------------------------------------------------------
// Dense stylized turf with two offset blade fields and root-fixed bending.
// POM still cannot create a silhouette outside the terrain raster footprint.
// All length values are metres. Two blades per 0.12m square (~139/m^2).
const GRASS_PATCH_SCALE_M: f32 = 7.0;
const GRASS_PATCH_LOW: f32 = 0.12;
const GRASS_PATCH_HIGH: f32 = 0.30;
const GRASS_CELL_M: f32 = 0.12;
const GRASS_HEIGHT_M: f32 = 0.095;
const GRASS_TURF_HEIGHT_M: f32 = 0.010;
const GRASS_HALF_WIDTH_M: f32 = 0.022;
const GRASS_POM_START_M: f32 = 10.0;
const GRASS_POM_END_M: f32 = 24.0;
const GRASS_MIN_STEPS: i32 = 24;
const GRASS_MAX_STEPS: i32 = 80;
const GRASS_REFINE_STEPS: i32 = 5;
const GRASS_RAY_DENOM_MIN: f32 = 0.10;
const GRASS_NORMAL_EPS_M: f32 = 0.002;
const GRASS_FAR_COVERAGE: f32 = 0.96;
const GRASS_NORMAL_STRENGTH: f32 = 0.35;
const GRASS_WIND_SPEED: f32 = 1.6;
// Keep <= .009 for the compact support / cell-boundary guarantee below.
const GRASS_SWAY_M: f32 = 0.009;
const GRASS_WIND_COLOR: f32 = 0.12;
// Linear RGB. Grass only inherits 15% of the material tint, avoiding brown turf.
const GRASS_ROOT_COLOR: vec3<f32> = vec3<f32>(0.045, 0.23, 0.018);
const GRASS_MID_COLOR: vec3<f32> = vec3<f32>(0.13, 0.48, 0.032);
const GRASS_TIP_COLOR: vec3<f32> = vec3<f32>(0.38, 0.70, 0.085);

fn grass_patch_noise(p: vec2<f32>) -> f32 {
    let q = p / GRASS_PATCH_SCALE_M;
    let cell = vec2<i32>(floor(q));
    let f = fract(q);
    let u = f * f * (3.0 - 2.0 * f);
    let a = relief_random(cell, 701u);
    let b = relief_random(cell + vec2<i32>(1, 0), 701u);
    let c = relief_random(cell + vec2<i32>(0, 1), 701u);
    let d = relief_random(cell + vec2<i32>(1, 1), 701u);
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn grass_patch(p: vec2<f32>) -> f32 {
    return smoothstep(GRASS_PATCH_LOW, GRASS_PATCH_HIGH, grass_patch_noise(p));
}

// Traveling waves in world space. The mask and root positions never move.
// Bevy globals.time supplies seconds; no new material binding required.
fn grass_wind(p: vec2<f32>) -> f32 {
    let time = globals.time * GRASS_WIND_SPEED;
    let phase = dot(p, vec2<f32>(0.58, 0.34));
    return 0.72 * sin(phase - time)
         + 0.28 * sin(phase * 2.3 - time * 1.7);
}

// One compact curved blade. Neighbor-cell search is unnecessary: maximum
// support radius sqrt(.047^2 + (.022+.009)^2) + .0029 < .060.
// Do not increase width/length/sway beyond that bound without neighbor search.
fn grass_blade_height(p: vec2<f32>, shift: vec2<f32>, seed: u32) -> f32 {
    let q = (p + shift) / GRASS_CELL_M;
    let cell = vec2<i32>(floor(q));
    let r0 = relief_random(cell, seed);
    let r1 = relief_random(cell, seed + 12u);
    let r2 = relief_random(cell, seed + 28u);
    let center = (vec2<f32>(r1, r2) - vec2<f32>(0.5)) * 0.004;
    let local = (fract(q) - vec2<f32>(0.5)) * GRASS_CELL_M - center;
    let angle = r0 * 6.28318530718;
    let axis = vec2<f32>(cos(angle), sin(angle));
    let across = vec2<f32>(-axis.y, axis.x);
    let half_length = mix(0.040, 0.047, r1);
    let along = (dot(local, axis) + half_length) / (2.0 * half_length);
    if (along <= 0.0 || along >= 1.0) { return 0.0; }
    let root = (vec2<f32>(cell) + vec2<f32>(0.5)) * GRASS_CELL_M
        - shift + center - axis * half_length;
    let wind = grass_wind(root);
    let bend = GRASS_SWAY_M * wind * along * along;
    let width = GRASS_HALF_WIDTH_M * mix(1.0, 0.16, along);
    let lateral = 1.0 - smoothstep(0.0, width, abs(dot(local, across) - bend));
    let rise = smoothstep(0.0, 0.36, along);
    let tip = 1.0 - smoothstep(0.70, 1.0, along);
    return GRASS_HEIGHT_M * mix(0.70, 1.0, r2) * rise * tip * lateral;
}

fn grass_height(p: vec2<f32>) -> f32 {
    let meadow = grass_patch(p);
    if (meadow <= 0.0) { return 0.0; }
    let a = grass_blade_height(p, vec2<f32>(0.0), 811u);
    let b = grass_blade_height(p, vec2<f32>(0.061, 0.053), 1201u);
    // Turf closes the brown gaps; blade relief rises above it.
    return meadow * max(GRASS_TURF_HEIGHT_M, max(a, b));
}

struct GrassHit {
    offset: vec3<f32>,
    height: f32,
    found: bool,
}

// March from ABOVE the terrain toward the original surface, so blades rise
// out of the ground. Empty space is not a solid cap at the top of the slab.
// Below the denominator limit, effective height is compressed consistently
// for tracing and normals. The exact grazing horizon still fades out.
fn trace_grass(p: vec2<f32>, N: vec3<f32>, V: vec3<f32>) -> GrassHit {
    let denom = max(dot(N, V), GRASS_RAY_DENOM_MIN);
    let top = V * (GRASS_HEIGHT_M / denom);
    let requested = ceil(length(top.xz) / (GRASS_HALF_WIDTH_M * 0.5));
    let steps = i32(clamp(requested, f32(GRASS_MIN_STEPS), f32(GRASS_MAX_STEPS)));
    var lo = 0.0;
    var hi = 1.0;
    var crossed = false;
    for (var i = 1; i <= GRASS_MAX_STEPS; i = i + 1) {
        if (i > steps) { break; }
        let t = f32(i) / f32(steps);
        let offset = top * (1.0 - t);
        let height = grass_height(p + offset.xz);
        let f = height - GRASS_HEIGHT_M * (1.0 - t);
        // Height zero is empty ground, not a grass hit.
        if (f >= 0.0 && height > 1e-6) {
            hi = t;
            crossed = true;
            break;
        }
        lo = t;
    }
    if (!crossed) { return GrassHit(vec3<f32>(0.0), 0.0, false); }
    for (var i = 0; i < GRASS_REFINE_STEPS; i = i + 1) {
        let t = 0.5 * (lo + hi);
        let offset = top * (1.0 - t);
        let f = grass_height(p + offset.xz) - GRASS_HEIGHT_M * (1.0 - t);
        if (f >= 0.0) { hi = t; } else { lo = t; }
    }
    let offset = top * (1.0 - hi);
    return GrassHit(offset, grass_height(p + offset.xz), true);
}

fn grass_gradient(p: vec2<f32>) -> vec2<f32> {
    let e = GRASS_NORMAL_EPS_M;
    return vec2<f32>(
        grass_height(p + vec2<f32>(e, 0.0)) - grass_height(p - vec2<f32>(e, 0.0)),
        grass_height(p + vec2<f32>(0.0, e)) - grass_height(p - vec2<f32>(0.0, e))
    ) / (2.0 * e);
}

fn depth_at(pos: vec4<f32>) -> f32 {
    return prepass_depth(pos, 0);
}

fn depth_edge_laplacian(pos: vec4<f32>, strength: f32) -> f32 {
    let d0 = depth_at(pos);
    let dR = depth_at(pos + vec4<f32>( 1.0,  0.0, 0.0, 0.0));
    let dL = depth_at(pos + vec4<f32>(-1.0,  0.0, 0.0, 0.0));
    let dU = depth_at(pos + vec4<f32>( 0.0,  1.0, 0.0, 0.0));
    let dD = depth_at(pos + vec4<f32>( 0.0, -1.0, 0.0, 0.0));

    let lap = abs((dR + dL + dU + dD) - (4.0 * d0));
    let scale = max(0.05, abs(1000.0 * d0));

    return saturate((lap / scale) * strength);
}

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: VertexOutput
) -> @location(0) vec4<f32> {
    var pbr_input: PbrInput = pbr_input_new();

    let double_sided =
        (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u;

    pbr_input.world_normal = fns::prepare_world_normal(
        normalize(mesh.world_normal),
        double_sided,
        is_front,
    );

    // Blend toward up so low-poly N·L faceting is less sharp; 0 = mesh, 1 = flat lit.
    let soften = saturate(style_params.x);
    let soft_n = normalize(mix(
        pbr_input.world_normal,
        vec3<f32>(0.0, 1.0, 0.0),
        soften,
    ));

    pbr_input.is_orthographic = view.clip_from_view[3].w == 1.0;
    let V = fns::calculate_view(mesh.world_position, pbr_input.is_orthographic);

    let P = mesh.world_position.xyz;
    let macro_n = normalize(pbr_input.world_normal);
    // No WORLD_COLOR_SHIFT here: relief has its own stable world domain.
    let relief_p = P.xz;

    // Evaluate derivatives before any nonuniform ray-marching control flow.
    // Fade unresolved relief instead of changing the height field per pixel.
    let footprint = max(length(dpdx(relief_p)), length(dpdy(relief_p)));
    let resolution_w = 1.0 - smoothstep(
        CRACK_WIDTH_M * 0.5,
        CRACK_WIDTH_M * 1.25,
        footprint
    );
    let distance_w = 1.0 - smoothstep(
        CRACK_RANGE_START_M, CRACK_RANGE_END_M, length(P - view.world_position.xyz));
    // Keep relief visible closer to the horizon.
    let facing_w = smoothstep(0.025, 0.10, dot(macro_n, V));
    let slope_w = smoothstep(0.15, 0.35, macro_n.y);
    let relief_w = distance_w * resolution_w * facing_w * slope_w;

    let base_palette = ground_color_at(relief_p + WORLD_COLOR_SHIFT);
    var ground = base_palette * base_color.rgb;
    var n = soft_n;

    let turf_cover = grass_patch(relief_p) * smoothstep(0.40, 0.70, macro_n.y);
    if (relief_w > 1e-4 && turf_cover < 0.98) {
        let hit = trace_relief(relief_p, macro_n, V);
        let q = relief_p + hit.offset.xz;
        let gradient = relief_gradient(q);
        // Exact local implicit-surface gradient is macro_n - D*(hx,0,hz).
        // Substitute soft_n only for final artistic lighting, never tracing.
        let shading_gradient = soft_n - CRACK_DEPTH_M
            * vec3<f32>(gradient.x, 0.0, gradient.y);
        let relief_n = normalize(shading_gradient);
        n = normalize(mix(soft_n, relief_n, relief_w));

        let hit_palette = ground_color_at(q + WORLD_COLOR_SHIFT);
        // Artistic cavity tint, not a light-direction self-shadow test.
        let cavity = mix(CRACK_DARK, 1.0, hit.height);
        let hit_ground = hit_palette * base_color.rgb * cavity;
        // Fade the appearance, not the physical relief depth / ray length.
        ground = mix(ground, hit_ground, relief_w);
    }

    // Persistent green underlayer, including gaps between resolved blades.
    let grass_slope = smoothstep(0.40, 0.70, macro_n.y);
    let grass_patch_w = grass_patch(relief_p) * grass_slope;
    let grass_tint = mix(vec3<f32>(1.0), base_color.rgb, 0.15);
    let gust = grass_wind(relief_p);
    // Fade travelling color bands when their wavelength becomes subpixel.
    let gust_resolved = 1.0 - smoothstep(0.8, 2.0, footprint);
    let grass_far_color = mix(GRASS_MID_COLOR, GRASS_TIP_COLOR,
        0.18 + GRASS_WIND_COLOR * gust * gust_resolved) * grass_tint;
    ground = mix(ground, grass_far_color, grass_patch_w * GRASS_FAR_COVERAGE);
    n = normalize(mix(n, soft_n, grass_patch_w * GRASS_FAR_COVERAGE));
    var grass_roughness = mix(1.0, 0.90, grass_patch_w);

    let grass_resolution = 1.0 - smoothstep(
        GRASS_HALF_WIDTH_M * 0.6, GRASS_HALF_WIDTH_M * 1.8, footprint);
    let grass_distance = 1.0 - smoothstep(
        GRASS_POM_START_M, GRASS_POM_END_M, length(P - view.world_position.xyz));
    let grass_facing = smoothstep(0.025, 0.10, dot(macro_n, V));
    let grass_pom_w = grass_resolution * grass_distance * grass_facing * grass_slope;

    let grass_reach = GRASS_HEIGHT_M * length(V.xz)
        / max(dot(macro_n, V), GRASS_RAY_DENOM_MIN);
    let meadow_upper_bound = grass_patch_noise(relief_p)
        + 2.122 * grass_reach / GRASS_PATCH_SCALE_M;
    if (grass_pom_w > 1e-4 && meadow_upper_bound > GRASS_PATCH_LOW) {
        let hit = trace_grass(relief_p, macro_n, V);
        if (hit.found) {
            let q = relief_p + hit.offset.xz;
            let height_ratio = saturate(hit.height / GRASS_HEIGHT_M);
            let cell_color = grass_patch_noise(q + vec2<f32>(43.1, 9.7));
            let blade_color = mix(GRASS_MID_COLOR * 0.78, GRASS_TIP_COLOR,
                smoothstep(0.10, 0.85, height_ratio));
            let root_shade = mix(GRASS_ROOT_COLOR, blade_color,
                smoothstep(0.0, 0.14, height_ratio));
            let detailed_ground = root_shade * grass_tint
                * (0.92 + 0.16 * cell_color + 0.06 * grass_wind(q));
            let gradient = grass_gradient(q);
            let compression = max(dot(macro_n, V), 0.0)
                / max(dot(macro_n, V), GRASS_RAY_DENOM_MIN);
            let detailed_n = normalize(soft_n - GRASS_NORMAL_STRENGTH * compression
                * vec3<f32>(gradient.x, 0.0, gradient.y));
            ground = mix(ground, detailed_ground, grass_pom_w);
            n = normalize(mix(n, detailed_n, grass_pom_w));
            grass_roughness = mix(grass_roughness, 0.90, grass_pom_w);
        }
    }

    pbr_input.material.base_color = vec4<f32>(ground, base_color.a);
    pbr_input.material.metallic = 0.0;
    pbr_input.material.perceptual_roughness = grass_roughness;
    pbr_input.frag_coord = mesh.position;
    pbr_input.world_position = mesh.world_position;
    pbr_input.world_normal = n;
    pbr_input.N = n;
    pbr_input.V = V;

    let lit_color = fns::apply_pbr_lighting(pbr_input);

    let edge = depth_edge_laplacian(mesh.position, style_params.y);
    let edge_ink = saturate(edge * 1000.0);
    let edge_intensity = mix(1.0, saturate(style_params.z), edge_ink);

    let shaded = lit_color.rgb * edge_intensity;
    let lit = with_distance_fog(
        vec4<f32>(shaded, 1.0),
        mesh.world_position.xyz,
        mesh.position.xy,
    );
    let toned = tone_mapping(lit, view.color_grading);
    let unlit = with_distance_fog(
        vec4<f32>(ground * edge_intensity, 1.0),
        mesh.world_position.xyz,
        mesh.position.xy,
    );
    let lit_mix = saturate(style_params.w);
    // Unlit albedo is a daylight stamp. Scale it with ambient so night
    // ground follows the key instead of staying readable at range.
    let day = saturate((dot(lights.ambient_color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722)) - 15.0) / 500.0);
    let final_color = mix(unlit.rgb * mix(0.16, 1.0, day), toned.rgb, lit_mix);

    return vec4<f32>(final_color, 1.0);
}
