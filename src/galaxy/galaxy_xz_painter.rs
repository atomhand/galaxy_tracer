use crate::prelude::*;
use bevy::prelude::*;
use std::f32::consts::PI;

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let s = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    return s * s * (3.0 - 2.0 * s);
}
pub struct GalaxyPainter<'a> {
    galaxy: &'a GalaxyConfig,
    component: &'a ComponentConfig,
}

impl GalaxyPainter<'_> {
    pub fn new<'a>(config: &'a GalaxyConfig, component: &'a ComponentConfig) -> GalaxyPainter<'a> {
        GalaxyPainter {
            galaxy: config,
            component,
        }
    }

    fn get_radial_intensity(&self, distance: f32, r0: f32) -> f32 {
        // Altho this is a virtual function in the reference codebase, I don't think anything overwrites it
        let r = f32::exp(-distance / (r0 * 0.5f32));
        return (r - 0.01f32).clamp(0.0, 0.1);
    }
    pub fn pos_winding(&self, p: Vec2) -> f32 {
        let rad = p.length() / self.galaxy.radius;
        return self.get_raw_winding(rad);
    }
    pub fn get_raw_winding(&self, rad: f32) -> f32 {
        let r = rad + 0.05;

        let t = f32::atan(f32::exp(-0.25 / (0.5 * r)) / self.galaxy.winding_b)
            * 2.0
            * self.galaxy.winding_n;
        //let t= atan(exp(1.0/r) / wb) * 2.0 * wn;

        return t;
    }

    fn find_theta_difference(&self, t1: f32, t2: f32) -> f32 {
        let diff: f32 = (t1 - t2).abs() / PI;
        let normalized_diff: f32 = ((diff + 1.0) % 2.0) - 1.0;
        return (normalized_diff).abs();
    }

    fn arm_modifier(&self, p: Vec2, winding: f32, arm_id: i32) -> f32 {
        let disp = self.galaxy.arm_offsets[arm_id as usize]; // angular offset

        let angular_offset = self.component.angular_offset.to_radians();
        let theta = -(f32::atan2(p.x, p.y) + angular_offset);

        let v = self.find_theta_difference(winding, theta + disp);

        return (1.0 - v).powf(self.component.arm_width * 15.0);
    }

    fn all_arms_modifier(&self, winding: f32, p: Vec2) -> f32 {
        let mut v: f32 = 0.0;
        for i in 0..self.galaxy.n_arms {
            v = v.max(self.arm_modifier(p, winding, i));
        }
        return v;
    }

    pub fn get_xz_intensity(&self, p: Vec2) -> f32 {
        let r0 = self.component.radial_extent;
        let inner = self.component.radial_dropoff; // central falloff parameter

        let d = p.length() / self.galaxy.radius; // distance to galactic central axis

        // this paramater is called scale in the reference codebase
        let central_falloff = (smoothstep(0.0, 1.0 * inner, d)).powi(4);
        let r = self.get_radial_intensity(d, r0);

        // I think the component winding_factor is only meant to apply to noise?
        let winding = self.get_raw_winding(d) ; // * self.component.winding_factor;
        let arm_mod = self.all_arms_modifier(winding, p);

        return central_falloff * arm_mod * r;
    }
}
