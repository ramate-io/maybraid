// prefix_scan_blocks.wgsl
// -------------------------------------------------------

#import proc::block_prefix::BLOCK_SIZE

@group(0) @binding(0)
var<storage, read_write> block_sums : array<u32>;
@group(0) @binding(1)
var<storage, read_write> block_prefix : array<u32>;

var<workgroup> temp : array<u32, BLOCK_SIZE>;

@compute @workgroup_size(BLOCK_SIZE)
fn main(@builtin(local_invocation_id) lid : vec3<u32>) 
{
    let i = lid.x;

    // Load
    if (i < arrayLength(&block_sums)) {
        temp[i] = block_sums[i];
    } else {
        temp[i] = 0u;
    }

    workgroupBarrier();

    // Blelloch again (same as before)
    // ... identical up-sweep + down-sweep ...

    // Write result
    if (i < arrayLength(&block_sums)) {
        block_prefix[i] = temp[i];
    }
}
