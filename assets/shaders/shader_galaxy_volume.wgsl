

// https://github.com/bevyengine/bevy/blob/c75d14586999dc1ef1ff6099adbc1f0abdb46edf/crates/bevy_render/src/view/view.wgsl
#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::mesh_functions::get_world_from_local
#import bevy_pbr::prepass_io::Vertex

#import "shaders/intensity_shared.wgsl"::get_intensity_coefficient;

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

    let world_space = vertex.position.xyz * vec3<f32>(radius*1.25,50.0,radius*1.25);
    let position = view.clip_from_world * model * vec4<f32>(world_space, 1.0);

    var out: MyVertexOutput;
    out.position = position;
    out.camera_origin = view.world_position;
    out.ray_dir = (model * vec4<f32>(world_space, 1.0)).xyz - view.world_position;

    return out;
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
    
    let n = vec3(0.0,1.0,0.0);
    let d = 25.0;
    let tplane1 : f32= -(dot(n,ro)+d) / dot(n, rd);
    let tplane2 : f32 = -(dot(n,ro)-d) / dot(n, rd);

    let o1 = ro + rd * max(0.0,tplane1);
    
    let t = (tplane2 - max(0.0,tplane1))/64.0;
    for(var i =0; i<64; i++) {
        let p = o1 + rd * (f32(i) * t);

        let intensity = get_intensity_coefficient(p, radius, 1.0, true);

        accumulation += intensity;
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