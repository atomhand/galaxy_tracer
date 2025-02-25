const pi = radians(180.0);
    
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

fn get_winding(rad : f32, wb : f32, wn : f32) -> f32 {
    let r = rad + 0.05;

    let t = atan(exp(-0.25/(0.5*r)) / wb) * 2.0 * wn;
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

// This is where the per component angular delta is added
// It's maybe a little awkward to apply it here
// But we do so in order to follow the original program
fn get_theta(p : vec3<f32>, angular_offset : f32) -> f32 {
    return atan2(p.x,p.z) + angular_offset;
}

fn arm_modifier(p : vec3<f32>, r : f32, angular_offset : f32, arm_id : i32) -> f32 {
    // .. these will be loaded from a uniform

    let wb = 0.5;
    let wn = 3.0;
    let aw = 0.1 * f32(arm_id+1);
    let disp = f32(arm_id) * radians(90.0); // angular offset

    let winding = get_winding(r,wb,wn);
    let theta = -get_theta(p,angular_offset);

    let v = abs(find_theta_difference(winding,theta+disp))/pi;

    return pow(1.0-v, aw*15.0);
}

fn all_arms_modifier(distance : f32, p : vec3<f32>, angular_offset : f32) -> f32 {
    var v = 0.0;
    let num_arms = 4;// 0 to 4
    for(var i = 0; i<4; i++) {
        if i >= num_arms { break; }
        v = max(v,arm_modifier(p,distance,angular_offset,i));
    }
    return v;
}

fn get_intensity_coefficient(p : vec3<f32>, galaxy_radius : f32, weight : f32, is_arm : bool) -> f32 {
    // component paramaters
    // These either need to be passed as function parameters
    // Or the function could be passed a component id and then read these from a uniform buffer

    let r0 = 0.5;
    let inner = 0.1; // central falloff parameter
    let angular_offset = -0.2;// per-component angular offset in radians. Called Delta in the program help
    let y0 = 0.01; // height of the component above the galaxy plane (called z0 in the program)


    let d = length(p.xz) / galaxy_radius; // distance to galactic central axis

    // this paramater is called scale in the reference codebase
    let central_falloff = pow(smoothstep(0.0,1.0 * inner, d), 4.0);

    let h = get_height_modulation(abs(p.y), y0);
    let r = get_radial_intensity(d, r0);
    let arm_mod = select(1.0,all_arms_modifier(d,p, angular_offset),is_arm); // some components don't follow the arms

    return central_falloff * arm_mod * h * r;
}