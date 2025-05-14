#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_render::view::View
#import bevy_pbr::prepass_bindings::PreviousViewUniforms
#import bevy_pbr::view_transformations::{uv_to_ndc,ndc_to_uv}
#import bevy_pbr::utils::coords_to_viewport_uv

struct BackgroundUpscaleSettings {
    current_pixel : f32,
    dimensions : vec2<f32>
}

//@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var<uniform> previous_view: PreviousViewUniforms;
@group(0) @binding(2) var<uniform> upscale_settings : BackgroundUpscaleSettings;
@group(0) @binding(3) var history_input_texture: texture_2d<f32>;
@group(0) @binding(4) var background_input_texture: texture_2d<f32>;
@group(0) @binding(5) var nearest_sampler: sampler;
@group(0) @binding(6) var linear_sampler: sampler;

struct Output {
    @location(0) view_target: vec4<f32>,
    @location(1) history: vec4<f32>,
};

fn coords_to_ray_direction(position: vec2<f32>, viewport: vec4<f32>) -> vec3<f32> {
    // Using world positions of the fragment and camera to calculate a ray direction
    // breaks down at large translations. This code only needs to know the ray direction.
    // The ray direction is along the direction from the camera to the fragment position.
    // In view space, the camera is at the origin, so the view space ray direction is
    // along the direction of the fragment position - (0,0,0) which is just the
    // fragment position.
    // Use the position on the near clipping plane to avoid -inf world position
    // because the far plane of an infinite reverse projection is at infinity.
    let view_position_homogeneous = view.view_from_clip * vec4(
        coords_to_viewport_uv(position, viewport) * vec2(2.0, -2.0) + vec2(-1.0, 1.0),
        1.0,
        1.0,
    );

    // Transforming the view space ray direction by the skybox transform matrix, it is 
    // equivalent to rotating the skybox itself.
    var view_ray_direction = view_position_homogeneous.xyz / view_position_homogeneous.w;
    view_ray_direction = (view.world_from_view * vec4(view_ray_direction, 0.0)).xyz;

    // Transforming the view space ray direction by the view matrix, transforms the
    // direction to world space. Note that the w element is set to 0.0, as this is a
    // vector direction, not a position, That causes the matrix multiplication to ignore
    // the translations from the view matrix.

    // transform is identity
    let ray_direction = view_ray_direction.xyz; // (uniforms.transform * vec4(view_ray_direction, 0.0)).xyz;

    return normalize(ray_direction);
}

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

@fragment
fn fragment(in: FullscreenVertexOutput) -> Output {   
    let rd  : vec3<f32>= coords_to_ray_direction(in.position.xy, view.viewport);


    let clip_pos = uv_to_ndc(in.uv); // Convert from uv to clip space
    let ro : vec3<f32> = view.world_position;//(view.world_from_clip * vec4(clip_pos, 0.0, 1.0)).xyz;
    let galaxy_radius : f32 = 500.0;


    // sphere intersect
    /*
    var t : f32 = sphIntersect(ro,rd, galaxy_radius).x;
    if( t < 0.0) {
        t = -dot(n,ro) / dot(n, rd);
    }
    */
    // galactic XZ plane
    let n =vec3(0.0,1.0,0.0);
    // "billboard plane"
    //let n =normalize(ro);

    // intersect plane
    let t : f32= -dot(n,ro) / dot(n, rd);
    
    let world_pos : vec4<f32> = vec4<f32>(ro + rd * t,1.0);
    let prev_clip_pos = (previous_view.clip_from_world * world_pos);
    let old_uv = ndc_to_uv(prev_clip_pos.xy/prev_clip_pos.w);

    let dimensions = vec2<f32>(textureDimensions(background_input_texture).xy) * 4.0;

    // Background sample position and pixel offset
    let coord = vec2<i32>(in.uv * dimensions);
    let sub_coord = coord % vec2<i32>(4,4);
    let center_uv = (vec2<f32>((coord/4)*4)+vec2<f32>(1.5,1.5)) / dimensions;
    let history_sample = textureSample(history_input_texture, nearest_sampler, old_uv);



    let velocity = length((old_uv-in.uv)*dimensions);
    var history_confidence = history_sample.a;
    if(t < 0.0) {
        history_confidence = 0.0;
    }

    // force new sample if historical uv is outside the screen buffer
    //  or if the difference in uvs is too high
    var force_new_sample = any(saturate(old_uv) != old_uv) || velocity > 4.0 || t < 0.0;

    const inverse_mapping = array(4, 11, 8, 5, 0, 13, 1, 9, 14, 10, 7, 12, 2, 15, 6, 3);
    let p = inverse_mapping[(sub_coord.x + sub_coord.y * 4) % 16];    

    var out = Output();
    if(force_new_sample) {
        let background_sample = textureSample(background_input_texture, linear_sampler, in.uv);
        out.history = vec4<f32>(background_sample.xyz,0.0);//background_sample;
        out.view_target =  vec4<f32>(background_sample.xyz,1.0);
    } else if( p == i32(upscale_settings.current_pixel)){
        let background_sample = textureSample(background_input_texture, nearest_sampler, center_uv);
        let blend = mix(history_sample,background_sample,1.0 - history_confidence * 0.5);
        out.history = vec4<f32>(blend.xyz,1.0);
        out.view_target =  vec4<f32>(blend.xyz,1.0);
    }
    else {
        out.history = vec4<f32>(history_sample.xyz,history_confidence);
        out.view_target = vec4<f32>(history_sample.rgb,1.0);
    }
    
    return out;
}
