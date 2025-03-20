
#import "shaders/noise_functions.wgsl"::{octave_noise_3d, ridge_noise};

@group(0) @binding(0) var octave_output: texture_storage_3d<r32float, write>;

@group(0) @binding(1) var ridge_output : texture_storage_3d<r32float, write>;



@compute @workgroup_size(8, 8, 8)
fn cache_octave_noise(@builtin(global_invocation_id) invocation_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    let location = vec3<i32>(i32(invocation_id.x), i32(invocation_id.y), i32(invocation_id.z));

    // fn octave_noise_3d(octaves: i32, persistence : f32, scale : f32, pos : vec3<f32> ) -> f3

    let pos = vec3<f32>(location) / (vec3<f32>(num_workgroups) * 8.0);

    let octaves = 10;
    let persistence = 1.0;
    let scale = 1.0;
    let noise = octave_noise_3d(octaves, persistence, scale, pos);

    // Some issues with caching an output outside the 0..1 range
    // should be resolvable
    //let tilt = 0.3;
    //let tilted = pow(noise,tilt);

    textureStore(octave_output, location,  vec4<f32>(noise,0.0,0.0,0.0));
}

@compute @workgroup_size(8, 8, 8)
fn cache_ridge_noise(@builtin(global_invocation_id) invocation_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    let location = vec3<i32>(i32(invocation_id.x), i32(invocation_id.y), i32(invocation_id.z));

    let pos = vec3<f32>(location) / (vec3<f32>(num_workgroups) * 8.0);

    let persistence = 1.0;
    let octaves = 9;
    let offset = 1.0;
    let tilt = 1.0;
    let noise =ridge_noise(pos, persistence, octaves, 2.5, offset, tilt);

    textureStore(ridge_output, location, vec4<f32>(noise,0.0,0.0,0.0));
}