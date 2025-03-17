

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