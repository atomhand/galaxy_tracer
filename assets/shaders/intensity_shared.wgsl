const pi = radians(180.0);

#import "shaders/common/noise_functions.wgsl"::perlin_cloud_noise;

// These structs are duplicated in render.rs, so make sure to update both
struct GalaxyParams {
    arm_offsets : vec4<f32>,
    radius : f32,
    num_arms : i32,
    winding_b : f32,
    winding_n : f32,
    padding_coefficient : f32,
    padding : vec3<f32>,
}
struct BulgeParams {
    strength : f32,
    r0 : f32, // (inverse) width
}
struct ComponentParams {
    strength : f32,
    arm_width : f32, // inverse
    y0 : f32,
    r0 : f32, // radial intensity start
    r1 : f32, // radial falloff start
    angular_offset : f32,
    winding : f32,
    noise_scale : f32,
    noise_offset : f32,
    tilt : f32,
    ks : f32
}

@group(2) @binding(0) var<uniform> galaxy: GalaxyParams;
@group(2) @binding(1) var<uniform> bulge: BulgeParams;
@group(2) @binding(2) var material_galaxy_texture: texture_2d<f32>;
@group(2) @binding(3) var material_galaxy_sampler: sampler;

fn pos_to_uv(p : vec2<f32>) -> vec2<f32> {
    return p / (galaxy.radius * 2.0 * galaxy.padding_coefficient) + 0.5;
}

fn get_height_modulation(height : f32, y0 : f32) -> f32 {
    // only overwritten by Bulge

    let h = abs(height / y0);
    if (h>2.0) {
        return 0.0;
    }

    let val = 1.0 / cosh(h);
    return val*val;
}

fn get_radial_intensity(distance : f32, r0 : f32) -> f32 {
    // Altho this is a virtual function in the reference codebase, I don't think anything overwrites it

    let r = exp(-distance / (r0 * 0.5f));
    return saturate(r-0.01f);
}

// SHOULD PROBABLY BE REPLACED BY A LUT
fn get_winding(rad : f32) -> f32 {
    let r = rad + 0.05;

    let t = atan(exp(-0.25/(0.5*r)) / galaxy.winding_b) * 2.0 * galaxy.winding_n;
    //let t= atan(exp(1.0/r) / wb) * 2.0 * wn;
    
    return t;
}

fn find_theta_difference(t1 : f32, t2 : f32) -> f32 {
    let v1 = abs(t1-t2);
    let v2 = abs(t1-t2-2.0*pi);
    let v3 = abs(t1-t2+2.0*pi);
    let v4 = abs(t1-t2-2.0*pi*2.0);
    let v5 = abs(t1-t2+2.0*pi*2.0);

    var v = min(v1,v2);
    v = min(v,v3);
    v = min(v,v4);
    v = min(v,v5);

    return v;
}

fn arm_modifier(p : vec2<f32>, r : f32, angular_offset : f32, arm_id : i32) -> f32 {
    // .. these will be loaded from a uniform

    let aw = 0.1 * f32(arm_id+1);
    let disp = galaxy.arm_offsets[arm_id]; // angular offset

    let winding = get_winding(r);
    let theta = -(atan2(p.x,p.y)+angular_offset);

    let v = abs(find_theta_difference(winding,theta+disp))/pi;

    return pow(1.0-v, aw*15.0);
}

fn all_arms_modifier(distance : f32, p : vec2<f32>, angular_offset : f32) -> f32 {
    var v = 0.0;
    for(var i = 0; i<4; i++) {
        if i >= galaxy.num_arms { break; }
        v = max(v,arm_modifier(p,distance,angular_offset,i));
    }
    return v;
}
fn get_xz_intensity(p : vec2<f32>, angular_offset : f32, is_arm : bool) -> f32 {
    return textureSample(material_galaxy_texture, material_galaxy_sampler, pos_to_uv(p)).x;
}

fn get_xz_intensity_old(p : vec2<f32>, angular_offset : f32, is_arm : bool) -> f32 {
    let r0 = 0.5;
    let inner = 0.1; // central falloff parameter
    let y0 = 0.01; // height of the component above the galaxy plane (called z0 in the program)


    let d = length(p) / galaxy.radius; // distance to galactic central axis

    // this paramater is called scale in the reference codebase
    let central_falloff = pow(smoothstep(0.0,1.0 * inner, d), 4.0);
    let r = get_radial_intensity(d, r0);
    let arm_mod = select(1.0,all_arms_modifier(d,p, angular_offset),is_arm); // some components don't follow the arms

    return central_falloff * arm_mod * r;
}

fn reconstruct_intensity(p : vec3<f32>, xz_intensity : f32, weight : f32) -> f32 {
    let y0 = 0.01; // height of the component above the galaxy plane (called z0 in the program)
    let h = get_height_modulation(abs(p.y), y0);

    return xz_intensity * h * weight;
}

fn get_intensity_coefficient(p : vec3<f32>, angular_offset : f32, weight : f32, is_arm : bool) -> f32 {
    // component paramaters
    // These either need to be passed as function parameters
    // Or the function could be passed a component id and then read these from a uniform buffer

    let r0 = 0.5;
    let inner = 0.1; // central falloff parameter
    let y0 = 0.01; // height of the component above the galaxy plane (called z0 in the program)


    let d = length(p.xz) / galaxy.radius; // distance to galactic central axis

    // this paramater is called scale in the reference codebase
    let central_falloff = pow(smoothstep(0.0,1.0 * inner, d), 4.0);

    let h = get_height_modulation(abs(p.y), y0);
    let r = get_radial_intensity(d, r0);
    let arm_mod = select(1.0,all_arms_modifier(d,p.xz, angular_offset),is_arm); // some components don't follow the arms

    return central_falloff * arm_mod * h * r * weight;
}

fn disk_intensity(p : vec3<f32>, winding_angle : f32, base_intensity : f32) -> f32 {
    if(base_intensity < 0.0005) {
        return 0.0;
    }

    let octaves : i32 = 10;
    let scale : f32 = 1.0 / 20.0;
    let persistence : f32 = 0.5;
    var p2 = abs(perlin_cloud_noise(p, winding_angle, octaves, scale, persistence));
    p2 = max(p2, 0.01);

    p2 = pow(p2,1.0); // pow(p2,noiseTilt)
    // p2 += componentParams.noiseOffset

    return base_intensity * p2;
}

fn dust_intensity(p : vec3<f32>, winding_angle : f32, base_intensity : f32) -> f32 {
    if(base_intensity < 0.0005) {
        return 0.0;
    }
    let octaves = 9;
    let scale = 1.0 / 10.0;
    let persistence = 0.5;
    var p2 = perlin_cloud_noise(p, winding_angle, octaves, scale, persistence);
    let noiseOffset = 0.0;
    p2 = max(p2-noiseOffset,0.0);

    let noiseTilt = 1.0;
    p2 = clamp(pow(5*p2, noiseTilt), -10.0, 10.0);

    let s : f32 = 0.01;

    return base_intensity * p2 * s;
}

fn ray_step(p: vec3<f32>, in_col : vec3<f32>, stepsize : f32) -> vec3<f32> {

    let d : f32 = length(p.xz) / galaxy.radius;
    let uv : vec2<f32> = pos_to_uv(p.xz);

    // disk components
    //for(var i = 0; i<4; i++) {
    //    if i >= galaxy.num_arms { break; }

    let disk_sample = textureSample(material_galaxy_texture, material_galaxy_sampler, uv).x;
    let disk_xz: f32  = reconstruct_intensity(p, disk_sample, 1.0);
    let winding_angle : f32 = -get_winding(d);//-disk_sample.y;

    let disk_col = vec3<f32>(3.54387,3.44474,3.448229);
    let disk_intensity : f32 = disk_intensity(p, winding_angle, disk_xz);

    let dust_xz = disk_xz;//textureSample(material_galaxy_texture, material_galaxy_sampler, uv, i + galaxy.num_arms).x;
    let dust_intensity : f32 = dust_intensity(p, winding_angle, dust_xz);
    //}

    let dust_col : vec3<f32> = vec3<f32>(1.0,1.0,1.0);
    let extinction : vec3<f32> = exp(-dust_intensity * dust_col );

    let col = in_col + disk_col * disk_intensity;
    return col * extinction;
}