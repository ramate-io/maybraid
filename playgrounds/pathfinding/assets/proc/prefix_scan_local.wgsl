// prefix_scan_local.wgsl
// -------------------------------------------------------

#import proc::block_sum::BLOCK_SIZE

@group(0) @binding(0)
var<storage, read>  in_counts : array<u32>;      // tri_counts
@group(0) @binding(1)
var<storage, read_write> out_prefix : array<u32>; // temporary local prefix
@group(0) @binding(2)
var<storage, read_write> block_sums : array<u32>; // one sum per group

var<workgroup> shared_vals : array<u32, BLOCK_SIZE>;

@compute @workgroup_size(BLOCK_SIZE)
fn main(@builtin(global_invocation_id) gid : vec3<u32>,
        @builtin(local_invocation_id) lid : vec3<u32>,
        @builtin(workgroup_id) wid : vec3<u32>) 
{
    let global_idx = gid.x;
    let local_idx  = lid.x;
    let group_idx  = wid.x;

    // Load into shared memory (0 if OOB)
    var val = 0u;
    if (global_idx < arrayLength(&in_counts)) {
        val = in_counts[global_idx];
    }
    shared_vals[local_idx] = val;

    workgroupBarrier();

    // -------- Blelloch scan --------

    // Up-sweep (reduce) phase
    var offset = 1u;
    var d = BLOCK_SIZE >> 1u;
    while (d > 0u) {
        if (local_idx < d) {
            let ai = offset * (2u * local_idx + 1u) - 1u;
            let bi = offset * (2u * local_idx + 2u) - 1u;
            shared_vals[bi] = shared_vals[bi] + shared_vals[ai];
        }
        offset = offset * 2u;
        d = d >> 1u;
        workgroupBarrier();
    }

    // Clear the last element (exclusive scan)
    if (local_idx == 0u) {
        block_sums[group_idx] = shared_vals[BLOCK_SIZE - 1u];
        shared_vals[BLOCK_SIZE - 1u] = 0u;
    }

    workgroupBarrier();

    // Down-sweep phase
    d = 1u;
    offset = BLOCK_SIZE;
    while (d < BLOCK_SIZE) {
        offset = offset >> 1u;

        if (local_idx < d) {
            let ai = offset * (2u * local_idx + 1u) - 1u;
            let bi = offset * (2u * local_idx + 2u) - 1u;

            let t = shared_vals[ai];
            shared_vals[ai] = shared_vals[bi];
            shared_vals[bi] = shared_vals[bi] + t;
        }

        d = d * 2u;
        workgroupBarrier();
    }

    // Write back prefix result
    if (global_idx < arrayLength(&in_counts)) {
        out_prefix[global_idx] = shared_vals[local_idx];
    }
}
