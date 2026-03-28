// prefix_add_offsets.wgsl
// -------------------------------------------------------

const BLOCK_SIZE : u32 = 256u;

@group(0) @binding(0)
var<storage, read_write> out_prefix : array<u32>;
@group(0) @binding(1)
var<storage, read> block_prefix : array<u32>;

@compute @workgroup_size(BLOCK_SIZE)
fn main(@builtin(global_invocation_id) gid : vec3<u32>,
        @builtin(workgroup_id) wid : vec3<u32>)
{
    let global_idx = gid.x;
    let group_idx  = wid.x;

    if (global_idx >= arrayLength(&out_prefix)) {
        return;
    }

    let offset = block_prefix[group_idx];
    out_prefix[global_idx] = out_prefix[global_idx] + offset;
}
