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

    let scaled_r = galaxy.radius * galaxy.padding_coefficient;
    let world_space = vertex.position.xyz * vec3<f32>(scaled_r,50.0,scaled_r);
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

fn march(ro : vec3<f32>, rd : vec3<f32>, t1 : f32, t2 : f32, t_c : f32) -> vec3<f32> {    
    var col = vec3<f32>(0.0,0.0,0.0);

    let o0 = ro + rd * max(0.0,t_c);
    let o1 = ro + rd * max(0.0,t1);

    let STEPS = galaxy.raymarch_steps / 2.0;

    let k = 1.5;
    let exposure = 0.1;

    // distributed from plane intersection to sphere intersection
    let step_0 = (t2-t_c);
    var sprev = 1.0;
    for(var i =0; i<i32(STEPS); i++) {
        let s = pow(1.0 - f32(i)/STEPS,k); // clustered towards 0
        let p = o0 + rd * s * step_0;
        col = ray_step(p, col, abs(s-sprev) * step_0 * exposure);
        sprev = s;
    }
    // distributed from ro to plane intersection
    sprev = 1.0 + sprev;
    let step_1 = (t_c-t1);
    for(var i =0; i<i32(STEPS); i++) {
        let s = 1.0 - pow(f32(i)/STEPS, k); // clustered towards 1
        let p = o1 + rd * s * step_1;
        col = ray_step(p, col, abs(s-sprev) * step_1 * exposure);
        sprev = s;
    }

    return col;    
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
    
    let n = vec3(0.0,1.0,0.0);
    let plane_t : f32= -dot(n,ro) / dot(n, rd);
    

    let a = march(mesh.camera_origin, normalize(mesh.ray_dir), 0.0, t.y, plane_t);
    return vec4<f32>(a,1.0);        
}