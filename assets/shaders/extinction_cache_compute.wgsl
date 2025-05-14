#import "shaders/intensity_shared.wgsl"::ray_step;

@group(0) @binding(0) var<uniform> camera_pos: vec4<f32>;
//@group(0) @binding(1) var extinction_output: texture_storage_2d<rgb10a2unorm, write>;
@group(0) @binding(1) var<storage, read_write> extinction_output: array<vec4<f32>>;
@group(0) @binding(2) var<storage> positions_input: array<vec4<f32>>;

@compute @workgroup_size(64, 1, 1)
fn cache_extinction(@builtin(global_invocation_id) invocation_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    let index = invocation_id.x;

    var col = vec3<f32>(1.0);
    let start : vec3<f32> = positions_input[index].xyz;
    let end: vec3<f32> = camera_pos.xyz;

    let STEPS = 128;
    let step : vec3<f32> = (end-start)/f32(STEPS);
    let step_size : f32 = length(step);
    let exposure = 0.1;

    for(var i =0; i<STEPS; i++) {
        let pos: vec3<f32> = start + f32(i) * step;
        col = ray_step(pos, col, step_size * exposure);
    }


    // store post extinction colour
    extinction_output[index] = vec4<f32>(col,1.0);
}