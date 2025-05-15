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
    let camera_right = normalize(vec3<f32>(view.clip_from_world[0].x, view.clip_from_world[1].x, view.clip_from_world[2].x));    
    let camera_up = normalize(vec3<f32>(view.clip_from_world[0].y, view.clip_from_world[1].y, view.clip_from_world[2].y));

    let model = get_world_from_local(vertex.instance_index);

    let scaled_r = galaxy.radius * galaxy.padding_coefficient;
    //let world_space = vertex.position.xyz * vec3<f32>(scaled_r,50.0,scaled_r);
    let world_space : vec3<f32> = (camera_right * vertex.position.x + camera_up * vertex.position.y ) * scaled_r;
    let position = view.clip_from_world * model * vec4<f32>(world_space,1.0);

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
    let STEPS = galaxy.raymarch_steps;
    let exposure = 0.1;

    // LINEAR TRACE
    let start = ro + rd * max(0.0,t2);
    let end = ro + rd * max(0.0,t1);
    let step = (end-start) / STEPS;
    let step_size = length(step);
    for(var i =0; i<i32(STEPS); i++) {
        let p = start + step * f32(i);
        col = ray_step(p,col,step_size * exposure);
    }
    return col;    
}

fn jitter(p : vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(41.0, 289.0)))*45758.5453 );
}

@fragment
fn fragment(
    mesh: VertexOutput,
) -> @location(0) vec4<f32> {
    let ro = mesh.camera_origin;
    let rd = normalize(mesh.ray_dir);
    
    let t = sphIntersect(ro,rd, galaxy.radius * galaxy.padding_coefficient);

    if t.x == -1.0 && t.y == -1.0 {
        return vec4<f32>(0.0,0.0,0.0,0.0);
    }

    let start = t.x ;
    // EXPERIMENTAL RAY OFFSET JITTER
    // this is a proof of concept, 
    let end = t.y + jitter(rd.xy + rd.zz) * 100.0;

    let a = march(mesh.camera_origin, normalize(mesh.ray_dir), start,end);
    return vec4<f32>(a,1.0);        
}