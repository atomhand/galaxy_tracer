use crate::prelude::*;
use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderRef, ShaderType},
};
use bytemuck::{Pod, Zeroable};

// These structs are duplicated in intensity_shared.wgsl, so make sure to update both
#[derive(ShaderType, Pod, Zeroable, Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GalaxyParams {
    arm_offsets: Vec4,
    radius: f32,
    num_arms: i32,
    winding_b: f32,
    winding_n: f32,
    padding_coefficient: f32,
    exposure: f32,
    raymarch_steps: f32,
    texture_dimension: f32,
}

impl GalaxyParams {
    pub fn read(config: &GalaxyConfig) -> Self {
        Self {
            padding_coefficient: config.padding_coeff,
            radius: config.radius,
            num_arms: config.n_arms,
            arm_offsets: Vec4::from_array(config.arm_offsets),
            winding_b: config.winding_b,
            winding_n: config.winding_n,
            exposure: config.exposure,
            raymarch_steps: config.raymarch_steps as f32,
            texture_dimension: config.texture_dimension as f32,
        }
    }
}

#[derive(ShaderType, Pod, Zeroable, Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct BulgeParams {
    strength: f32,
    r0: f32, // (inverse) width
    intensity_mod: f32,
}

impl BulgeParams {
    pub fn read(config: &GalaxyConfig) -> Self {
        Self {
            strength: config.bulge_strength,
            r0: config.bulge_radius,
            intensity_mod: config.bulge_intensity,
        }
    }
}

#[derive(ShaderType, Pod, Zeroable, Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ComponentParams {
    strength: f32,
    arm_width: f32, // inverse
    y_thickness: f32,
    radial_extent: f32,   // radial intensity start
    central_falloff: f32, // radial falloff start
    angular_offset: f32,
    winding_factor: f32,
    noise_scale: f32,
    noise_offset: f32,
    noise_tilt: f32,
    noise_persistence: f32,
    noise_octaves: f32,
}

impl ComponentParams {
    pub fn read(component: &ComponentConfig) -> Self {
        Self {
            strength: if component.enabled {
                component.strength
            } else {
                0.0
            },
            arm_width: component.arm_width,
            y_thickness: component.y_thickness,
            radial_extent: component.radial_extent,
            central_falloff: component.radial_dropoff,
            angular_offset: component.angular_offset,
            winding_factor: component.noise_winding_factor,
            noise_scale: component.noise_scale,
            noise_offset: component.noise_offset,
            noise_tilt: component.noise_tilt,
            noise_persistence: component.noise_persistence,
            noise_octaves: if component.noise_enabled {
                component.noise_octaves as f32
            } else {
                0.0
            },
        }
    }
}
