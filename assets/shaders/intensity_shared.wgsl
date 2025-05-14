const pi = radians(180.0);

#import "shaders/noise_functions.wgsl"::{ridge_noise,octave_noise_3d};

// Returns position rotated by the provided winding angle and scaled to the unit galaxy
fn get_twirled_unit_pos(p : vec3<f32>, winding_angle : f32) -> vec3<f32> {
    let rot : vec2<f32> = vec2<f32>(cos(winding_angle),sin(winding_angle));
    return vec3<f32>( p.x * rot.x - p.z * rot.y, p.y,  p.x * rot.y + p.z * rot.x) / galaxy.radius;
}

fn disk_noise(p : vec3<f32>, winding_angle : f32, octaves : i32) -> f32 {
    let r = get_twirled_unit_pos(p,winding_angle);
    return octave_noise_3d(octaves,disk_params.noise_persistence,disk_params.noise_scale, r);    
}

fn dust_noise(p : vec3<f32>, winding_angle : f32, octaves : i32) -> f32 {
    let pr = get_twirled_unit_pos(p, winding_angle);
    return max(0.0,ridge_noise(pr * dust_params.noise_scale, dust_params.noise_persistence,octaves,2.5,dust_params.noise_offset, dust_params.noise_tilt));
}

// END Noise utilities

// These structs are duplicated in render.rs, so make sure to update both
struct GalaxyParams {
    arm_offsets : vec4<f32>,
    radius : f32,
    num_arms : i32,
    winding_b : f32,
    winding_n : f32,
    padding_coefficient : f32,
    exposure : f32,
    raymarch_steps : f32,
    texture_dimension : f32,
}
struct BulgeParams {
    strength : f32,
    radius : f32, // width
    intensity_mod : f32,
}
struct ComponentParams {
    strength : f32,
    arm_width : f32, // inverse
    y_thickness : f32,
    radial_extent : f32, // radial intensity start
    central_falloff : f32, // radial central falloff start
    angular_offset : f32,
    winding_factor : f32,
    noise_scale : f32,
    noise_offset : f32,
    noise_tilt : f32,
    noise_persistence : f32,
    noise_octaves : f32,
}

#ifdef COMPUTE_BINDINGS
// TODO - ADD VIEW UNIFORM HERE?
@group(0) @binding(3) var<uniform> galaxy: GalaxyParams;
@group(0) @binding(4) var<uniform> bulge_params: BulgeParams;
@group(0) @binding(5) var<uniform> disk_params: ComponentParams;
@group(0) @binding(6) var<uniform> dust_params: ComponentParams;
@group(0) @binding(7) var galaxy_xz_texture: texture_2d<f32>;
@group(0) @binding(8) var galaxy_xz_sampler: sampler; // are there texture samplers in compute shaders?
@group(0) @binding(9) var lut_texture: texture_2d_array<f32>;
@group(0) @binding(10) var lut_sampler: sampler;
// noise lookup not enabled for compute pass
#else
@group(2) @binding(0) var<uniform> galaxy: GalaxyParams;
@group(2) @binding(1) var<uniform> bulge_params: BulgeParams;
@group(2) @binding(2) var<uniform> disk_params: ComponentParams;
@group(2) @binding(3) var<uniform> dust_params: ComponentParams;
@group(2) @binding(4) var galaxy_xz_texture: texture_2d<f32>;
@group(2) @binding(5) var galaxy_xz_sampler: sampler;
@group(2) @binding(6) var lut_texture: texture_2d_array<f32>;
@group(2) @binding(7) var lut_sampler: sampler;
#endif

const LUT_ID_WINDING : i32 = 0;

fn pos_to_uv(p : vec2<f32>) -> vec2<f32> {
    return p / (galaxy.radius * 2.0 * galaxy.padding_coefficient) + 0.5 + vec2<f32>(0.5,0.5)/galaxy.texture_dimension;
}

fn lookup_winding(d : f32) -> f32 {
#ifdef FLAT_DIAGNOSTIC
    return 0.0;
#else
#ifdef COMPUTE_BINDINGS
    return textureSampleLevel(lut_texture,lut_sampler, vec2<f32>(d + 0.5 / galaxy.texture_dimension,0.5), LUT_ID_WINDING,0.0).x;
#else
    return textureSample(lut_texture,lut_sampler, vec2<f32>(d + 0.5 / galaxy.texture_dimension,0.5), LUT_ID_WINDING).x;
#endif
#endif
}

fn get_height_modulation(height : f32, y_thickness : f32) -> f32 {
    let h = abs(height / (y_thickness*galaxy.radius));
    if (h>2.0) {
        return 0.0;
    }

    let val = 1.0 / cosh(h);
    return val*val;
}

fn reconstruct_intensity(p : vec3<f32>, xz_intensity : f32, y_thickness : f32) -> f32 {
    let h = get_height_modulation(p.y, y_thickness);

    return xz_intensity * h;
}

fn get_disk_intensity(p : vec3<f32>, winding_angle : f32, base_intensity : f32) -> f32 {
    if(base_intensity < 0.0005 || disk_params.strength == 0.0) {
        return 0.0;
    }

    var p2 = 0.5;
    let octaves = i32(disk_params.noise_octaves);
#ifdef DIAGNOSTIC
    return f32(octaves) / 10.0;
#else
    if octaves > 0 {
        p2 = abs(disk_noise(p, winding_angle, octaves));
    }

    // These should be folded into the cached noise texture
    // (BUt I need to sort the tex format to deal with values outside 0..1 )
    p2 = max(p2, 0.01);
    p2 = pow(p2,disk_params.noise_tilt);
    p2 += disk_params.noise_offset;
    
    return base_intensity * p2 * disk_params.strength;
#endif
}

fn get_dust_intensity(p : vec3<f32>, winding_angle : f32, base_intensity : f32) -> f32 {
    if(base_intensity < 0.0005 || dust_params.strength == 0.0) {
        return 0.0;
    }

    var p2 = 0.5;
    let octaves = i32(dust_params.noise_octaves);
#ifdef DIAGNOSTIC
    return f32(octaves) / 10.0;
#else
    if octaves > 0 {
        p2 = dust_noise(p, winding_angle, octaves);
    }

    // These should be folded into the cached noise texture
    // (BUt I need to sort the tex format to deal with values outside 0..1 )
    p2 = max(p2-dust_params.noise_offset,0.0);
    p2 = clamp(pow(5*p2, dust_params.noise_tilt), -10.0, 10.0);

    let s : f32 = 0.01;
    return base_intensity * p2 * s * dust_params.strength;
#endif
}

fn get_dust_intensity_ridged(p : vec3<f32>, winding_angle : f32, base_intensity : f32) -> f32 {
    if(base_intensity < 0.0005 || dust_params.strength == 0.0) {
        return 0.0;
    }

    var p2 = 0.5;
    let octaves = i32(dust_params.noise_octaves);
#ifdef DIAGNOSTIC
    return f32(octaves) / 10.0;
#else
    if octaves > 0 {
        p2 = dust_noise(p,winding_angle,octaves);
    }

    let s : f32 = 0.01;
    return p2 * base_intensity * s * dust_params.strength;
#endif
}

fn get_bulge_intensity(p : vec3<f32>) -> f32 {
    let rho_0: f32 = bulge_params.strength;
    let rad : f32 = (length(p)/galaxy.radius+0.01)*bulge_params.radius + 0.01;
    var i : f32 = rho_0 * (pow(rad,-0.855)*exp(-pow(rad,1.0/4.0f)) - 0.05f);
    return max(0.0,i);
}

fn ray_step(p: vec3<f32>, in_col : vec3<f32>, stepsize : f32) -> vec3<f32> {

    let d : f32 = length(p.xz) / galaxy.radius;
    let uv : vec2<f32> = pos_to_uv(p.xz);

#ifdef COMPUTE_BINDINGS
    let xz_sample : vec4<f32> = textureSampleLevel(galaxy_xz_texture, galaxy_xz_sampler, uv,0.0);
#else
    let xz_sample : vec4<f32> = textureSample(galaxy_xz_texture, galaxy_xz_sampler, uv);
#endif

    // It's feasible to calculate this live, but caching it to the texture/LUT gets a very acceptable result and seems to be faster
    let base_winding : f32 = -lookup_winding(d);//-xz_sample.w;//-get_winding(d);

    let dust_xz = reconstruct_intensity(p, xz_sample.y, dust_params.y_thickness);
    let dust_winding_angle : f32 = base_winding * dust_params.winding_factor;
    let dust_intensity : f32 = get_dust_intensity_ridged(p, dust_winding_angle, dust_xz) * stepsize;
    // yellow absorption spectra = appears red
    let dust_col = vec3<f32>(0.4,0.6,1.0);
    let extinction : vec3<f32> = exp(-dust_intensity * dust_col );

#ifdef EXTINCTION_ONLY
    return in_col * extinction;
#else
    let disk_xz: f32  = reconstruct_intensity(p, xz_sample.x, disk_params.y_thickness);
    let disk_winding_angle : f32 = base_winding * disk_params.winding_factor;//-disk_sample.y;

    //  blue
    let disk_col = vec3<f32>(0.4,0.6,1.0);
    let disk_intensity : f32 = get_disk_intensity(p, disk_winding_angle, disk_xz) * stepsize;
    
    let bulge_intensity = get_bulge_intensity(p) * stepsize * galaxy.exposure * 0.1;
    // yellow
    let bulge_col = vec3<f32>(1.,0.9,0.45);

#ifdef FLAT_DIAGNOSTIC
    return vec3<f32>(1.0 - dust_intensity);
#else
#ifdef DIAGNOSTIC
    return in_col + vec3<f32>((disk_intensity + dust_intensity), 0.0, 0.0);
#else
    let col = in_col + disk_col * disk_intensity * galaxy.exposure + bulge_col * bulge_intensity;
    return col * extinction;
#endif
#endif
#endif
}