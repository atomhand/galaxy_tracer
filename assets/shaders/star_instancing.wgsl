#import bevy_pbr::{
    mesh_functions,
    view_transformations::position_world_to_clip
}
#import bevy_pbr::mesh_functions::{get_world_from_local, mesh_position_local_to_clip}
#import bevy_pbr::view_transformations::position_world_to_clip;
#import bevy_pbr::mesh_view_bindings::view
//@group(2) @binding(0) var texture: texture_2d<f32>;
//@group(2) @binding(1) var texture_sampler: sampler;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv : vec2<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    let billboard_margin_scale = 2.0;
    let galaxy_scale_factor = 0.2;

    let in_color = vec4<f32>(1.0,1.0,1.0,0.1);

    let scale_factor =  (length(in_color)/10.0+6.0)*galaxy_scale_factor * billboard_margin_scale;

    let camera_right = normalize(vec3<f32>(view.clip_from_world[0].x, view.clip_from_world[1].x, view.clip_from_world[2].x));    
    let camera_up = normalize(vec3<f32>(view.clip_from_world[0].y, view.clip_from_world[1].y, view.clip_from_world[2].y));

    var out : VertexOutput;
    out.world_position = get_world_from_local(vertex.instance_index) * vec4<f32>((camera_right * vertex.position.x + camera_up * vertex.position.y ) * scale_factor,1.0);
    out.clip_position = view.clip_from_world * vec4<f32>(out.world_position.xyz, 1.0);
    out.color = vec4<f32>(normalize(in_color.rgb),1.0);
    out.uv = vertex.position.xy * billboard_margin_scale;

    return out;
}

fn draw_star(pos : vec2<f32>, star_color : vec3<f32>, I : f32) -> vec3<f32> {
    let system_transition_factor = 0.0;

    let c = star_color;

    var d : f32 = length(pos);

    var col = I * c;
    var spectrum = I * c;

    col = spectrum / (d*d*d);

    let ARMS_SCALE = 1.0 / 1.4;

    d = length(pos * vec2<f32>(50.0,0.5)) * ARMS_SCALE;
    col += spectrum/ (d*d*d) * (1.0 - system_transition_factor);
    d = length(pos * vec2<f32>(0.5,50.0)) * ARMS_SCALE;
    col += spectrum / (d*d*d) * (1.0 - system_transition_factor);

    return col;// * (1.0 - smoothstep(0.75,1.0,length(pos)));
}

const weights_4 = array<vec2<f32>,4>(
    vec2<f32>(1.0/8.0,3.0/8.0),
    vec2<f32>(3.0/8.0,-1.0/8.0),
    vec2<f32>(-1.0/8.0,-3.0/8.0),
    vec2<f32>(-3.0/8.0,1.0/8.0)
);
const weights_8 = array<vec2<f32>,8>(
    vec2<f32>(1.0/8.0,-3.0/8.0),
    vec2<f32>(-1.0/8.0,3.0/8.0),
    vec2<f32>(5.0/8.0,1.0/8.0),
    vec2<f32>(-3.0/8.0,-5.0/8.0),
    vec2<f32>(-5.0/8.0,5.0/8.0),
    vec2<f32>(-7.0/8.0,-1.0/8.0),
    vec2<f32>(3.0/8.0,7.0/8.0),
    vec2<f32>(7.0/8.0,-7.0/8.0)
);

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let dpdx = dpdx(in.uv);//vec2(dpdx(in.uv),dpdy(in.uv));
    let dpdy = dpdy(in.uv);

    let intensity = 1.0 / 256.0;//.02*exp(-15.*rnd(1));

    var starcol = vec3<f32>(0.0);
    for(var i =0; i<8; i+=1) {
        starcol     += draw_star(in.uv + dpdx * weights_8[i].x + dpdy * weights_8[i].y, in.color.rgb, intensity);
    }

    starcol = starcol / 8.0;

    let a = (starcol.x+starcol.y+starcol.z)/3.0;

    return vec4<f32>(starcol,a);
}