

fn get_intensity_coefficient(p : vec3<f32>, angular_offset : f32, weight : f32) -> f32 {
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
    let arm_mod = all_arms_modifier(d,p.xz, angular_offset); // some components don't follow the arms

    return central_falloff * arm_mod * h * r * weight;
}

fn find_theta_difference_old(t1 : f32, t2 : f32) -> f32 {
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

fn arm_modifier(p_theta : f32, r : f32, angular_offset : f32, arm_id : i32) -> f32 {
    // .. these will be loaded from a uniform

    let aw = 0.1 * f32(arm_id+1);
    let disp = galaxy.arm_offsets[arm_id]; // angular offset

    let winding = get_winding(r);
    let theta = -(p_theta+angular_offset);

    let v = find_theta_difference(winding,theta+disp);

    return pow(1.0-v, aw*15.0);
}

fn all_arms_modifier(distance : f32, p : vec2<f32>, angular_offset : f32) -> f32 {
    var v = 0.0;
    let p_theta = atan2(p.x,p.y);
    for(var i = 0; i<4; i++) {
        if i >= galaxy.num_arms { break; }
        v = max(v,arm_modifier(p_theta,distance,angular_offset,i));
    }
    return v;
}

// SHOULD PROBABLY BE REPLACED BY A LUT
fn get_winding(rad : f32) -> f32 {
    let r = rad + 0.05;

    let t = atan(exp(-0.25/(0.5*r)) / galaxy.winding_b) * 2.0 * galaxy.winding_n;
    //let t= atan(exp(1.0/r) / wb) * 2.0 * wn;
    
    return t;
}

fn find_theta_difference(t1 : f32, t2 : f32) -> f32 {

    let diff: f32  = abs(t1 - t2) / pi;
    let normalized_diff : f32 = ((diff + 1.0) % 2.0) - 1.0;
    return abs(normalized_diff);
}

fn get_xz_intensity_old(p : vec2<f32>, angular_offset : f32) -> f32 {
    let r0 = 0.5;
    let inner = 0.1; // central falloff parameter

    let d = length(p) / galaxy.radius; // distance to galactic central axis

    // this paramater is called scale in the reference codebase
    let central_falloff = pow(smoothstep(0.0,1.0 * inner, d), 4.0);
    let r = get_radial_intensity(d, r0);
    let arm_mod = all_arms_modifier(d,p, angular_offset); // some components don't follow the arms

    return central_falloff * arm_mod * r;
}

fn get_radial_intensity(distance : f32, r0 : f32) -> f32 {
    let r = exp(-distance / (r0 * 0.5f));
    return saturate(r-0.01f);
}