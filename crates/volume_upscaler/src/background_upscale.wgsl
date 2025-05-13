#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_render::view::View
#import bevy_pbr::view_transformations::{uv_to_ndc,ndc_to_uv}

struct BackgroundUpscaleSettings {
    current_pixel : f32,
    dimensions : vec2<f32>
}

struct PreviousViewUniforms {
    view_from_world: mat4x4<f32>,
    clip_from_world: mat4x4<f32>,
}

//@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(0) var history_input_texture: texture_2d<f32>;
@group(0) @binding(1) var background_input_texture: texture_2d<f32>;
@group(0) @binding(2) var texture_sampler: sampler;
@group(0) @binding(3) var<uniform> view: View;
@group(0) @binding(4) var<uniform> previous_view: PreviousViewUniforms;
@group(0) @binding(5) var<uniform> upscale_settings : BackgroundUpscaleSettings;

struct PostProcessSettings {
    intensity: f32,
#ifdef SIXTEEN_BYTE_ALIGNMENT
    // WebGL2 structs must be 16 byte aligned.
    _webgl2_padding: vec3<f32>
#endif
}
@group(0) @binding(2) var<uniform> settings: PostProcessSettings;

struct Output {
    @location(0) view_target: vec4<f32>,
    @location(1) history: vec4<f32>,
};

@fragment
fn fragment(in: FullscreenVertexOutput) -> Output {   
    //let view_sample = textureSample(screen_texture, texture_sampler, in.uv);

    // For history sample, reproject UV according to the current and previous view transforms
    // TODO - This hasn't actually improved quality. Not sure if bugged, or that this form of reprojection isn't useful in my test case.
    // Need to work out the problem.
    let clip_pos = uv_to_ndc(in.uv); // Convert from uv to clip space
    let world_pos = view.world_from_clip * vec4(clip_pos, 0.0, 1.0);
    let prev_clip_pos = (previous_view.clip_from_world * world_pos).xy;
    let velocity = (clip_pos - prev_clip_pos) * vec2(0.5, -0.5); // this is the ndc_to_uv conversion, but we can drop some terms that cancel out
    let old_uv = ndc_to_uv(prev_clip_pos);

    let force_new_sample =any(saturate(old_uv) != old_uv);

    // Background sample position and pixel offset
    let dimensions = vec2<f32>(textureDimensions(background_input_texture).xy) * 4.0;
    let coord = vec2<i32>(in.uv * dimensions);
    let sub_coord = coord % vec2<i32>(4,4);

    const inverse_mapping = array(4, 11, 8, 5, 0, 13, 1, 9, 14, 10, 7, 12, 2, 15, 6, 3);
    let p = inverse_mapping[(sub_coord.x + sub_coord.y * 4) % 16];

    var out = Output();
    if(force_new_sample || p == i32(upscale_settings.current_pixel)) {
        let center_uv = (vec2<f32>((coord/4)*4)+vec2<f32>(1.5,1.5)) / dimensions;
        let background_sample = textureSample(background_input_texture, texture_sampler, center_uv);

        // Match
        out.history = background_sample;
        out.view_target =  background_sample;
    } else {
        let history_sample = textureSample(history_input_texture, texture_sampler, old_uv);
        out.history = history_sample;
        out.view_target =history_sample;
    }

    return out;
}
