

// https://github.com/bevyengine/bevy/blob/c75d14586999dc1ef1ff6099adbc1f0abdb46edf/crates/bevy_render/src/view/view.wgsl
#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::mesh_functions::get_world_from_local
#import bevy_pbr::prepass_io::Vertex

@group(2) @binding(0) var<uniform> radius: f32;

// see https://github.com/kulkalkul/bevy_mod_billboard/blob/main/src/shader/billboard.wgsl

struct MyVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) camera_origin: vec3<f32>,
    @location(1) ray_dir: vec3<f32>,
}


@vertex
fn vertex(vertex: Vertex) -> MyVertexOutput {
    let model = get_world_from_local(vertex.instance_index);

    let world_space = vertex.position.xyz * vec3<f32>(radius,25.0,radius);
    let position = view.clip_from_world * model * vec4<f32>(world_space, 1.0);

    var out: MyVertexOutput;
    out.position = position;
    out.camera_origin = view.world_position;
    out.ray_dir = (model * vec4<f32>(world_space, 1.0)).xyz - view.world_position;

    return out;
}

fn sech2(x : f32) -> f32 {
    let denom = exp(x) + exp(-x);
    return 4.0 / (denom*denom);
}

//https://github.com/leuat/gamer/blob/ebe1b8addeac5accd4ea6d5b4918c18e99d5a6f5/source/galaxy/galaxycomponent.h#L156

fn get_winding(rad : f32, wb : f32, wn : f32) -> f32 {
    let r = rad + 0.05;

    let t = atan(exp(-0.25/(0.5*r)) / wb) * 2.0 * wn;
    //let t= atan(exp(1.0/r) / wb) * 2.0 * wn;
    
    return t;
}

fn find_theta_difference(t1 : f32, t2 : f32) -> f32 {
    let pi = 3.1459;

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

fn get_theta(p : vec3<f32>) -> f32 {
    return atan2(p.x,p.z);
}

// To combine arms, take max over mutliple
// p - sampling pos
// r - radial distance to centre (normalised to the galaxy scale)
// disp - angular displacement of the arm
// wb winding number b
// wn winding number n
// aw arm width (inverse)
fn arm_modifier(p : vec3<f32>, r : f32, wb : f32, wn : f32, aw : f32, disp : f32) -> f32 {
    let pi = 3.1459;

    let winding = get_winding(r,wb,wn);
    let theta = -get_theta(p);

    let v = abs(find_theta_difference(winding,theta+disp))/pi;

    return pow(1.0-v, aw*15.0);
}

fn cosh(x : f32) -> f32 {
    return (exp(x) + exp(-x))/2.0;
}

fn height_modulation(height : f32) -> f32 {
    let z0 = 10.0;
    let h = height / z0;
    if(h>2.0) {
        return 0.0;
    }

    let val = 1.0 / cosh(h);
    return val*val;
}

fn radial_intensity(rad : f32) -> f32 {
    let r0 = 0.75; // falloff radius parameter (expressed in terms of the Unit galaxy radius)
    let r = exp(-rad / (r0 * 0.5));
    return saturate(r-0.01);
}

// returns near and far intersection point
fn sphIntersect( ro : vec3<f32> , rd : vec3<f32> ,  r : f32 ) -> vec2<f32>
{
    let oc : vec3<f32> = ro;
    let b : f32 = dot( oc, rd );
    let c : f32 = dot( oc, oc ) - r*r;
    var h : f32 = b*b - c;
    if( h<0.0 ) { return vec2<f32>(-1.0); }
    h = sqrt( h );

    return vec2(-b - h, -b + h);
}

fn march(ro : vec3<f32>, rd : vec3<f32>, t1 : f32, t2 : f32) -> f32 {
    var accumulation = 0.0;

    if t1 == -1.0 {
        return accumulation;
    }

/*
    let tplane = -dot(vec3(0.0,1.0,0.0),roo) / dot(vec3(0,1.0,0.0), rd);
    let p = roo + rd * tplane;
    let r = length(p.xz)/radius;//length(p.xz);//atan2(p.z,p.x);
    return arm_modifier(p / radius,r) * disk_intensity_modifier(p,r);
    */
    
    let n = vec3(0.0,1.0,0.0);
    let d = 25.0;
    let tplane1 : f32= -(dot(n,ro)+d) / dot(n, rd);
    let tplane2 : f32 = -(dot(n,ro)-d) / dot(n, rd);

    let roo = ro + rd * max(0.0,tplane1);
    
    let t = (tplane2 - max(0.0,tplane1))/64.0;
    for(var i =0; i<64; i++) {
        let p = roo + rd * (f32(i) * t);
        let r = length(p.xz)/(radius*0.5);//length(p.xz);//atan2(p.z,p.x);

        let a1 = arm_modifier(p,r,0.5,3.0,0.2,0.0);
        let a2 = arm_modifier(p,r,0.5,3.0,1.2,3.0);

        let intensity = max(a1,a2) * height_modulation(p.y) * radial_intensity(r);

        accumulation += intensity / 64.0;
    }
    return accumulation;
    
}

@fragment
fn fragment(
    mesh: MyVertexOutput,
) -> @location(0) vec4<f32> {
    let t = sphIntersect(mesh.camera_origin, normalize(mesh.ray_dir), radius);

    //let a = (t.y-max(0.0,t.x))/(radius*2.0)*0.1;
    let a = march(mesh.camera_origin, normalize(mesh.ray_dir), max(0.0,t.x),max(0.0,t.y));
    return vec4<f32>(1.0,0.0,0.0,1.0)*a;        
}