#import bevy_pbr::{
    mesh_functions,
    view_transformations::position_world_to_clip
}
struct StarInstancingSettings {
    supersampling_offset : f32,
    padding : vec3<f32>,
}

#import bevy_pbr::mesh_functions::{get_world_from_local, mesh_position_local_to_clip}
#import bevy_pbr::view_transformations::position_world_to_clip;
#import bevy_pbr::mesh_view_bindings::view
@group(2) @binding(0) var<storage> extinction_output: array<vec4<f32>>;
@group(2) @binding(1) var<uniform> settings: StarInstancingSettings;
@group(2) @binding(2) var star_psf_texture: texture_2d<f32>;
@group(2) @binding(3) var linear_sampler: sampler;

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,

    @location(3) i_pos_scale: vec4<f32>,
    //@location(4) i_color: vec4<f32>,
};


struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv : vec2<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    let billboard_margin_scale = 4.0;
    let minor_stars_scale_factor = 0.5;

    // retrieve colour based on instance tag
    let in_color = extinction_output[i32(vertex.i_pos_scale.w)].rgb;

    let view_pos = (view.clip_from_world * vec4<f32>(vertex.i_pos_scale.xyz, 1.0)).xyz;
    let distance = length(view_pos);

    var scale =  (in_color.x+in_color.y+in_color.z) * minor_stars_scale_factor * billboard_margin_scale;
    //let min_scale = distance * 0.01;
    //scale = max(scale,min_scale);

    let camera_right = normalize(vec3<f32>(view.clip_from_world[0].x, view.clip_from_world[1].x, view.clip_from_world[2].x));    
    let camera_up = normalize(vec3<f32>(view.clip_from_world[0].y, view.clip_from_world[1].y, view.clip_from_world[2].y));

    var out : VertexOutput;
    out.world_position = vec4<f32>((camera_right * vertex.position.x + camera_up * vertex.position.y ) * scale + vertex.i_pos_scale.xyz,1.0);
    out.clip_position = view.clip_from_world * vec4<f32>(out.world_position.xyz, 1.0);
    out.uv = vertex.position.xy * billboard_margin_scale;
    out.color = vec4<f32>(in_color,1.0);

    return out;
}

fn draw_star(pos : vec2<f32>, star_color : vec3<f32>, I : f32) -> vec3<f32> {
    return textureSample(star_psf_texture, linear_sampler, pos + vec2<f32>(0.5,0.5)).r  * normalize(star_color.rgb)* I;
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
    let dpdx = dpdx(in.uv) * settings.supersampling_offset;//vec2(dpdx(in.uv),dpdy(in.uv));
    let dpdy = dpdy(in.uv) * settings.supersampling_offset;

    let intensity = 1.0;//.02*exp(-15.*rnd(1));

    var starcol = vec3<f32>(0.0);
    for(var i =0; i<8; i+=1) {
        starcol     += draw_star(in.uv + dpdx * weights_8[i].x + dpdy * weights_8[i].y, in.color.rgb, intensity) / 8.0;
    }
    let a = 1.0 * (starcol.x+starcol.y+starcol.z)/3.0;
    return vec4<f32>(starcol,a);
}