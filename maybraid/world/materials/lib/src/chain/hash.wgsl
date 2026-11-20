//---------------------------------------------------------
// Shared hash for mythical-chain deterioration.
// CPU port: hash.rs. The fragment shader must keep these
// function bodies verbatim — tests compare both files.
//---------------------------------------------------------

fn hash13(p: vec3<f32>) -> f32 {
    let p3 = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    let d = dot(p3, p3.yzx + vec3<f32>(33.33));
    return fract((p3.x + p3.y) * p3.z + d);
}

fn hash21(p: vec2<f32>) -> f32 {
    return hash13(vec3<f32>(p, 17.13));
}

fn link_weakness(id: f32, seed: f32) -> f32 {
    return hash13(vec3<f32>(id * 17.13, seed * 0.013, 4.2));
}
