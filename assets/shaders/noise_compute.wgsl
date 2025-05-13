
#import "shaders/wrapped_noise_functions.wgsl"::{octave_noise_3d, ridge_noise, ridge_noise_split};

struct NoiseSettingsUniform {
    persistence : f32,
    frequency : f32,
    offset : f32,
    tilt : f32,
    octaves: f32,
}

@group(0) @binding(0) var octave_output: texture_storage_3d<rg32float, write>;

@group(0) @binding(1) var ridge_output : texture_storage_3d<rg32float, write>;
@group(0) @binding(2) var ridge_detail : texture_storage_3d<rg32float, write>;

@group(0) @binding(3) var<uniform> disk_settings: NoiseSettingsUniform;
@group(0) @binding(4) var<uniform> dust_settings: NoiseSettingsUniform;

@compute @workgroup_size(8, 8, 8)
fn cache_octave_noise(@builtin(global_invocation_id) invocation_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    let location = vec3<i32>(i32(invocation_id.x), i32(invocation_id.y), i32(invocation_id.z));

    // fn octave_noise_3d(octaves: i32, persistence : f32, scale : f32, pos : vec3<f32> ) -> f3

    let pos = vec3<f32>(location) / (vec3<f32>(num_workgroups) * 8.0);

    let noise = octave_noise_3d(i32(disk_settings.octaves), disk_settings.persistence, disk_settings.frequency, pos);

    // Some issues with caching an output outside the 0..1 range
    // should be resolvable
    //let tilt = 0.3;
    //let tilted = pow(noise,tilt);

    textureStore(octave_output, location,  vec4<f32>(noise,0.0,0.0,0.0));
}

// COMBINING 2 RIDGE NOISE 

@compute @workgroup_size(8, 8, 8)
fn cache_ridge_noise(@builtin(global_invocation_id) invocation_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
    let location = vec3<i32>(i32(invocation_id.x), i32(invocation_id.y), i32(invocation_id.z));

    let pos = vec3<f32>(location) / (vec3<f32>(num_workgroups) * 8.0);

    let lacunarity = 2.0; // 2.5
    let octaves = i32(dust_settings.octaves);
    let first_octaves = octaves - octaves / 2;
    let noise =ridge_noise(pos, dust_settings.frequency, dust_settings.persistence, first_octaves, lacunarity, dust_settings.offset, dust_settings.tilt);

    textureStore(ridge_output, location, vec4<f32>(noise,0.0,0.0));

    let detail_target_freq : f32 = dust_settings.frequency * pow(lacunarity,f32(first_octaves));
    let detail_persistence = dust_settings.persistence * pow(lacunarity,f32(first_octaves));

    let detail_freq = detail_target_freq / 4.0;

    let detail =ridge_noise(pos, detail_freq, detail_persistence, octaves /2 , lacunarity, dust_settings.offset, dust_settings.tilt);

    textureStore(ridge_detail, location, vec4<f32>(detail,0.0,0.0));
}