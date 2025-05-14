#import "shaders/intensity_shared.wgsl"::ray_step;

@group(0) @binding(0) var<uniform> camera_pos: vec4<f32>;
@group(0) @binding(1) var extinction_output: texture_storage_2d<rgb10a2unorm, write>;
@group(0) @binding(2) var<storage> positions_input: array<vec4<f32>>;

@compute @workgroup_size(8, 8, 1)
fn cache_extinction(@builtin(global_invocation_id) invocation_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));

    // fn octave_noise_3d(octaves: i32, persistence : f32, scale : f32, pos : vec3<f32> ) -> f3

    let dims = vec2<i32>(num_workgroups.xy) * 8;

    //let uv = vec2<f32>(location) / vec2<f32>(dims);

    var col = vec3<f32>(1.0);
    let start : vec3<f32> = positions_input[location.x + location.y * dims.x].xyz;
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
    textureStore(extinction_output, location,  vec4<f32>(col,1.0));
}