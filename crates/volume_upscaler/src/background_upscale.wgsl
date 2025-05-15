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
    // need to pass this as a uniform this constant
    let galaxy_radius : f32 = 500.0;

    // Calculate reference fragment world position for reprojection
    // We use whichever intersection is closest - (or just the only intersection, if one is a miss/behind the camera)
    // -- an origin-centred sphere scaled with a radius chosen to be the larger of
    // -A The distance of the camera to the origin
    // -B The galaxy radius scaled by a const factor
    //      This is for the case when the camera is facing away from the origin.
    //      The galaxy radius could be substituted with a system radius etc., the important thing is it needs to be defined in terms of the area the camera is constrained to 
    //
    // -- the galactic (XZ) plane

    // Get intersection
    let rd  : vec3<f32>= coords_to_ray_direction(in.position.xy, view.viewport);
    let clip_pos = uv_to_ndc(in.uv); // Convert from uv to clip space
    let ro : vec3<f32> = view.world_position;//(view.world_from_clip * vec4(clip_pos, 0.0, 1.0)).xyz;
    // backface of a sphere
    let rad = max(galaxy_radius*1.5,length(ro));
    var sph = sphIntersect(ro,rd, rad);
    var t = max(sph.x,sph.y);
    // galactic xz plane
    let n =vec3(0.0,1.0,0.0);
    let plane_intersection = -dot(n,ro) / dot(n, rd);
    if plane_intersection > 0.0 && (plane_intersection < t || t < 0.0) {
        t = plane_intersection;
    }
    // reconstruct old uv from world pos
    var world_pos : vec4<f32> = vec4<f32>(ro + rd * t,1.0);
    let prev_clip_pos = (previous_view.clip_from_world * world_pos);
    let old_uv = ndc_to_uv(prev_clip_pos.xy/prev_clip_pos.w);

    // Background sample position and pixel offset
    let dimensions = vec2<f32>(textureDimensions(background_input_texture).xy) * 4.0;
    let coord = vec2<i32>(in.uv * dimensions);
    let center_uv = (vec2<f32>((coord/4)*4)+vec2<f32>(1.5,1.5)) / dimensions;
    let history_sample = textureSample(history_input_texture, linear_sampler, old_uv);

    // The effect of velocity on image needs to be smoothly continuous
    // - any sort of step or threshold can produce strong artefacts
    // Biasing the ramp towards 0 (eg squaring velocity) will make the image less floaty
    // but biasing it towards 1 makes it better at suppressing discontinuities.

    // The current function is empirically chosen to be a good balance,
    // but if the position reconstruction were higher quality (eg. had a depth input),
    // discontinuities would be less common and we could afford to increase velocity's
    // effect to reduce floatiness.
    let velocity = length((old_uv-in.uv)*dimensions);
    let confidence_velocity_factor = saturate(1.0 /velocity);
    var history_confidence = min(confidence_velocity_factor,history_sample.a);
    
    if any(saturate(old_uv) != old_uv) {
        history_confidence = 0.0;
    }

    const inverse_mapping = array(4, 11, 8, 5, 0, 13, 1, 9, 14, 10, 7, 12, 2, 15, 6, 3);
    let sub_coord = coord % vec2<i32>(4,4);
    let p = inverse_mapping[(sub_coord.x + sub_coord.y * 4) % 16];

    var out = Output();
    if( p == i32(upscale_settings.current_pixel)){
        let background_sample = textureSample(background_input_texture, nearest_sampler, center_uv);
        // Not sure whether blending (instead of plain overwrite) is an overall benefit
        // at this step
        let blend = mix(history_sample,background_sample,1.0-history_confidence*0.5);
        out.history = vec4<f32>(blend.rgb,1.0);
        out.view_target =  vec4<f32>(blend.rgb,1.0);
    }
    else {
        let background_sample = textureSample(background_input_texture, linear_sampler, in.uv);
        let blended_sample = mix(background_sample.rgb,history_sample.rgb,history_confidence);

        out.history = vec4<f32>(blended_sample,history_confidence);
        out.view_target = vec4<f32>(blended_sample,1.0);
    }
    
    return out;
}
