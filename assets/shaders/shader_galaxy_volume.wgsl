

// https://github.com/bevyengine/bevy/blob/c75d14586999dc1ef1ff6099adbc1f0abdb46edf/crates/bevy_render/src/view/view.wgsl
#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::mesh_functions::get_world_from_local
#import bevy_pbr::prepass_io::Vertex


#import "shaders/intensity_shared.wgsl"::{galaxy, ray_step, get_xz_intensity};

// see https://github.com/kulkalkul/bevy_mod_billboard/blob/main/src/shader/billboard.wgsl

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) camera_origin: vec3<f32>,
    @location(1) ray_dir: vec3<f32>,
}


@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    let model = get_world_from_local(vertex.instance_index);

    let world_space = vertex.position.xyz * vec3<f32>(galaxy.radius*1.25,50.0,galaxy.radius*1.25);
    let position = view.clip_from_world * model * vec4<f32>(world_space, 1.0);

    var out: VertexOutput;
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

fn march(ro : vec3<f32>, rd : vec3<f32>, t1 : f32, t2 : f32) -> vec3<f32> {
    
    var col = vec3<f32>(0.0,0.0,0.0);
    if t1 == -1.0 && t2 == -1.0 {
        return col;
    }

    let o1 = ro + rd * max(0.0,t1);

    let STEPS = galaxy.raymarch_steps;

    
    let t = (t2 - max(0.0,t1))/STEPS;
    let step_weight = abs(t) / 10.0;

    for(var i =0; i<i32(STEPS); i++) {
        let p = o1 + rd * (STEPS-f32(i)) * t;
        col = ray_step(p, col, step_weight);
    }
    return col;
    
}

@fragment
fn fragment(
    mesh: VertexOutput,
) -> @location(0) vec4<f32> {
    //let t = sphIntersect(mesh.camera_origin, normalize(mesh.ray_dir), galaxy.radius);

    let ro = mesh.camera_origin;
    let rd = normalize(mesh.ray_dir);
    let n = vec3(0.0,1.0,0.0);
    let d = 25.0;
    let t1 : f32= -(dot(n,ro)+d) / dot(n, rd);
    let t2 : f32 = -(dot(n,ro)-d) / dot(n, rd);

    //let a = (t.y-max(0.0,t.x))/(radius*2.0)*0.1;
    let a = march(mesh.camera_origin, normalize(mesh.ray_dir), t1, t2);
    return vec4<f32>(a,1.0);        
}