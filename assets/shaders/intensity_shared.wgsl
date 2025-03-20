const pi = radians(180.0);

// NOISE FUNCTIONS CREDIT BRIAN SHARPE (Released under open license, with request for credit only)
// https://github.com/BrianSharpe/GPU-Noise-Lib/
// ---- OF COURSE THESE SHOULD BE AN INCLUDE FILE, BUT BEVY DID NOT WANT TO PLAY NICE

fn Interpolation_C2_3D( x : vec3<f32> ) -> vec3<f32> {
    return x * x * x * (x * (x * 6.0 - 15.0) + 10.0);
}

struct Hash {
    lowz : vec4<f32>,
    highz : vec4<f32>
}

fn FAST32_hash_3D(  _gridcell : vec3<f32> ) -> Hash //	generates a random number for each of the 8 cell corners
{
    var gridcell = _gridcell;
    //    gridcell is assumed to be an integer coordinate

    //	TODO: 	these constants need tweaked to find the best possible noise.
    //			probably requires some kind of brute force computational searching or something....
    let OFFSET : vec2<f32> = vec2<f32>( 50.0, 161.0 );
    let DOMAIN: f32 = 69.0;
    let SOMELARGEFLOAT : f32 = 635.298681;
    let ZINC: f32 = 48.500388;

    //	truncate the domain
    gridcell = gridcell.xyz - floor(gridcell.xyz * ( 1.0 / DOMAIN )) * DOMAIN;
    let gridcell_inc1 : vec3<f32>  = step( gridcell, vec3<f32>( DOMAIN - 1.5 ) ) * ( gridcell + 1.0 );

    //	calculate the noise
    var P : vec4<f32> = vec4<f32>( gridcell.xy, gridcell_inc1.xy ) + OFFSET.xyxy;
    P *= P;
    P = P.xzxz * P.yyww;

    var highz_hash = vec4<f32>(1.0 / ( SOMELARGEFLOAT + vec2<f32>( gridcell.z, gridcell_inc1.z ) * ZINC ),0.0,0.0);
    let lowz_hash = fract( P * highz_hash.xxxx );
    highz_hash = fract( P * highz_hash.yyyy );

    return Hash(lowz_hash,highz_hash);
}

fn Perlin3D(  P : vec3<f32>) -> f32
{
    //	establish our grid cell and unit position
    let Pi : vec3<f32>= floor(P);
    let Pf: vec3<f32> = P - Pi;
    let Pf_min1 : vec3<f32> = Pf - 1.0;

    //
    //	improved noise.
    //	requires 1 random value per point.  Will run faster than classic noise if a slow hashing function is used
    //

    //	calculate the hash.
    //	( various hashing methods listed in order of speed )
    let hash = FAST32_hash_3D( Pi);
    var hash_lowz = hash.lowz;
    var hash_highz = hash.highz;
    //BBS_hash_3D( Pi, hash_lowz, hash_highz );
    //SGPP_hash_3D( Pi, hash_lowz, hash_highz );

    //
    //	"improved" noise using 8 corner gradients.  Faster than the 12 mid-edge point method.
    //	Ken mentions using diagonals like this can cause "clumping", but we'll live with that.
    //	[1,1,1]  [-1,1,1]  [1,-1,1]  [-1,-1,1]
    //	[1,1,-1] [-1,1,-1] [1,-1,-1] [-1,-1,-1]
    //
    hash_lowz -= 0.5;
    let grad_results_0_0 : vec4<f32> = vec2<f32>( Pf.x, Pf_min1.x ).xyxy * sign( hash_lowz );
    hash_lowz = abs( hash_lowz ) - 0.25;
    let grad_results_0_1 : vec4<f32>  = vec2<f32>( Pf.y, Pf_min1.y ).xxyy * sign( hash_lowz );
    let grad_results_0_2 : vec4<f32>  = Pf.zzzz * sign( abs( hash_lowz ) - 0.125 );
    let grad_results_0 : vec4<f32>  = grad_results_0_0 + grad_results_0_1 + grad_results_0_2;

    hash_highz -= 0.5;
    let grad_results_1_0 : vec4<f32>  = vec2<f32>( Pf.x, Pf_min1.x ).xyxy * sign( hash_highz );
    hash_highz = abs( hash_highz ) - 0.25;
    let grad_results_1_1 : vec4<f32>  = vec2<f32>( Pf.y, Pf_min1.y ).xxyy * sign( hash_highz );
    let grad_results_1_2 : vec4<f32>  = Pf_min1.zzzz * sign( abs( hash_highz ) - 0.125 );
    let grad_results_1 : vec4<f32>  = grad_results_1_0 + grad_results_1_1 + grad_results_1_2;

    //	blend the gradients and return
    let blend : vec3<f32> = Interpolation_C2_3D( Pf );
    let res0 : vec4<f32>= mix( grad_results_0, grad_results_1, blend.z );
    let blend2 : vec4<f32>  = vec4<f32>( blend.xy, vec2<f32>( 1.0 - blend.xy ) );
    return dot( res0, blend2.zxzx * blend2.wwyy ) * (2.0 / 3.0);	//	(optionally) mult by (2.0/3.0) to scale to a strict -1.0->1.0 range

}

// END BRIAN SHARPE NOISE FUNCTIONS

// NOISE UTILITIES BASED on GAMER source code
// ---- Again, ideally these would be in an include file, but they are temporarily residing here so long as Bevy stochastically fails to find nested WGSL imports

// Returns position rotated by the provided winding angle and scaled to the unit galaxy
fn get_twirled_unit_pos(p : vec3<f32>, winding_angle : f32) -> vec3<f32> {
    let rot : vec2<f32> = vec2<f32>(cos(winding_angle),sin(winding_angle));
    return vec3<f32>( p.x * rot.x - p.z * rot.y, p.y,  p.x * rot.y + p.z * rot.x) / galaxy.radius;
}

// see https://github.com/leuat/gamer/blob/ebe1b8addeac5accd4ea6d5b4918c18e99d5a6f5/source/noise/noise.cpp#L4
fn ridge_noise( in_pos : vec3<f32>,in_frequency : f32,octaves : i32, lacunarity : f32, offset : f32, gain : f32) -> f32 {
    var value = 0.0;
    var weight = 1.0;

    let w = - 0.05f;
    var freq = in_frequency;

    var p = in_pos;
    for(var i =0; i < octaves; i++) {
        var signal = Perlin3D(p);

        signal = abs(signal);
        signal = offset - signal;
        signal *= signal;

        signal *= weight;

        weight = signal * gain;

        weight = saturate(signal * gain);
        
        value += signal * pow(freq,w);

        p = p * lacunarity;
        freq *= lacunarity;
    }
    return (value * 1.25) - 1.0;
}

fn octave_noise_3d(octaves: i32, persistence : f32, scale : f32, pos : vec3<f32> ) -> f32 {
    var sum = 0.0;
    var frequency = scale;
    var amplitude = 1.0;

    var amp_sum = 0.0;
    for(var i =0; i < octaves; i++) {
        sum += Perlin3D(pos * frequency) * amplitude;

        frequency *= 2.0;
        amp_sum += amplitude;
        amplitude *= persistence;
    }

    return sum / amp_sum;
}

fn perlin_cloud_noise(p : vec3<f32>, winding_angle : f32, octaves : i32, scale : f32, persistence : f32) -> f32 {
    let r = get_twirled_unit_pos(p,winding_angle);
    return octave_noise_3d(octaves,persistence,scale, r);
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
    padding : f32,
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

@group(2) @binding(0) var<uniform> galaxy: GalaxyParams;
@group(2) @binding(1) var<uniform> bulge_params: BulgeParams;
@group(2) @binding(2) var<uniform> disk_params: ComponentParams;
@group(2) @binding(3) var<uniform> dust_params: ComponentParams;
@group(2) @binding(4) var<uniform> stars_params: ComponentParams;
@group(2) @binding(5) var galaxy_xz_texture: texture_2d<f32>;
@group(2) @binding(6) var galaxy_xz_sampler: sampler;
@group(2) @binding(7) var lut_texture: texture_2d_array<f32>;
@group(2) @binding(8) var lut_sampler: sampler;

const LUT_ID_WINDING : i32 = 0;

fn pos_to_uv(p : vec2<f32>) -> vec2<f32> {
    return p / (galaxy.radius * 2.0 * galaxy.padding_coefficient) + 0.5;
}

fn lookup_winding(d : f32) -> f32 {
    return textureSample(lut_texture,lut_sampler, vec2<f32>(d,0.5), LUT_ID_WINDING).x;
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
        p2 = abs(perlin_cloud_noise(p, winding_angle, octaves, disk_params.noise_scale, disk_params.noise_persistence));
    }

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
        p2 = perlin_cloud_noise(p, winding_angle, octaves, dust_params.noise_scale, dust_params.noise_persistence);
    }

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
        let pr = get_twirled_unit_pos(p, winding_angle);
        p2 = max(0.0,ridge_noise(pr * dust_params.noise_scale, dust_params.noise_persistence,octaves,2.5,dust_params.noise_offset, dust_params.noise_tilt));
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

    let xz_sample : vec4<f32> = textureSample(galaxy_xz_texture, galaxy_xz_sampler, uv);

    // It's feasible to calculate this live, but caching it to the texture/LUT gets a very acceptable result and seems to be faster
    let base_winding : f32 = -lookup_winding(d);//-xz_sample.w;//-get_winding(d);

    let disk_xz: f32  = reconstruct_intensity(p, xz_sample.x, disk_params.y_thickness);
    let disk_winding_angle : f32 = base_winding * disk_params.winding_factor;//-disk_sample.y;

    //  blue
    let disk_col = vec3<f32>(0.4,0.6,1.0);
    let disk_intensity : f32 = get_disk_intensity(p, disk_winding_angle, disk_xz) * stepsize;

    let dust_xz = reconstruct_intensity(p, xz_sample.y, dust_params.y_thickness);
    let dust_winding_angle : f32 = base_winding * dust_params.winding_factor;
    let dust_intensity : f32 = get_dust_intensity_ridged(p, dust_winding_angle, dust_xz) * stepsize;

    let bulge_intensity = get_bulge_intensity(p) * stepsize * galaxy.exposure;
    // yellow
    let bulge_col = vec3<f32>(1.,0.9,0.45);

    // yellow absorption spectra = appears red
    let dust_col = vec3<f32>(0.4,0.6,1.0);
    let extinction : vec3<f32> = exp(-dust_intensity * dust_col );

#ifdef DIAGNOSTIC
    return in_col + vec3<f32>((disk_intensity + dust_intensity), 0.0, 0.0);
#else
    let col = in_col + disk_col * disk_intensity * galaxy.exposure + bulge_col * bulge_intensity;
    return col * extinction;
#endif
}