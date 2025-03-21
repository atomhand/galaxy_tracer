
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

fn FAST32_hash_3D_WRAPPED(  _gridcell : vec3<f32>, DOMAIN : f32 ) -> Hash //	generates a random number for each of the 8 cell corners
{
    var gridcell = _gridcell;
    //    gridcell is assumed to be an integer coordinate

    //	TODO: 	these constants need tweaked to find the best possible noise.
    //			probably requires some kind of brute force computational searching or something....
    let OFFSET : vec2<f32> = vec2<f32>( 50.0, 161.0 );
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

fn Perlin3D_Wrapped(  P : vec3<f32>, domain :f32) -> f32
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
    let hash = FAST32_hash_3D_WRAPPED( Pi, domain);
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

// see https://github.com/leuat/gamer/blob/ebe1b8addeac5accd4ea6d5b4918c18e99d5a6f5/source/noise/noise.cpp#L4
fn ridge_noise( in_pos : vec3<f32>, scale : f32, in_frequency : f32,octaves : i32, lacunarity : f32, offset : f32, gain : f32) -> f32 {
    var value = 0.0;
    var weight = 1.0;

    let w = - 0.05f;
    var freq = in_frequency;

    var domain = scale;

    var p = in_pos * scale;
    for(var i =0; i < octaves; i++) {
        var signal = Perlin3D_Wrapped(p,domain);

        signal = abs(signal);
        signal = offset - signal;
        signal *= signal;

        signal *= weight;

        weight = signal * gain;

        weight = saturate(signal * gain);
        
        value += signal * pow(freq,w);

        p = p * lacunarity;
        domain = domain * lacunarity;
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
        sum += Perlin3D_Wrapped(pos * frequency, frequency) * amplitude;


        frequency *= 2.0;
        amp_sum += amplitude;
        amplitude *= persistence;
    }

    return sum / amp_sum;
}