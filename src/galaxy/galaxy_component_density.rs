use crate::prelude::*;
use bevy::prelude::*;
use std::f32::consts::PI;

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let s = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    s * s * (3.0 - 2.0 * s)
}
pub struct GalaxyComponentDensity<'a> {
    galaxy: &'a GalaxyConfig,
    component: &'a ComponentConfig,
}

impl GalaxyComponentDensity<'_> {
    pub fn new<'a>(
        config: &'a GalaxyConfig,
        component: &'a ComponentConfig,
    ) -> GalaxyComponentDensity<'a> {
        GalaxyComponentDensity {
            galaxy: config,
            component,
        }
    }

    fn get_radial_intensity(&self, distance: f32, r0: f32) -> f32 {
        // Altho this is a virtual function in the reference codebase, I don't think anything overwrites it
        let r = f32::exp(-distance / (r0 * 0.5f32));
        (r - 0.01f32).clamp(0.0, 0.1)
    }
    pub fn pos_winding(&self, p: Vec2) -> f32 {
        let rad = p.length() / self.galaxy.radius;
        self.rad_winding(rad)
    }

    /// Returns winding value given a radial distance to the galaxy center (scaled to the unit galaxy)
    pub fn rad_winding(&self, radial_distance: f32) -> f32 {
        let r = radial_distance + 0.05;
        f32::atan(f32::exp(-0.5 / r) / self.galaxy.winding_b) * 2.0 * self.galaxy.winding_n
        //let t= atan(exp(1.0/r) / wb) * 2.0 * wn;
    }

    fn find_theta_difference(&self, t1: f32, t2: f32) -> f32 {
        let diff: f32 = (t1 - t2).abs() / PI;
        let normalized_diff: f32 = ((diff + 1.0) % 2.0) - 1.0;
        (normalized_diff).abs()
    }

    /// Returns the highest density out of all arms at the given position
    fn arms_modifier(&self, winding: f32, p: Vec2) -> f32 {
        let angular_offset = self.component.angular_offset.to_radians();
        let theta = -(f32::atan2(p.x, p.y) + angular_offset);

        // modifier for each arm
        let mut highest: f32 = 0.0;            
        for arm_id in 0..self.galaxy.n_arms {
            let disp = self.galaxy.arm_offsets[arm_id as usize]; // angular offset
            let v = self.find_theta_difference(winding, theta + disp);
            highest = f32::max(highest,(1.0 - v).powf(self.component.arm_width * 15.0))
        }
        highest
    }

    fn get_height_modulation(&self, height: f32) -> f32 {
        let h = f32::abs(height / (self.component.y_thickness * self.galaxy.radius));
        if h > 2.0 {
            return 0.0;
        }

        let val = 1.0 / f32::cosh(h);
        val * val
    }

    pub fn xyz_density(&self, p: Vec3) -> f32 {
        self.xz_density(p.xz()) * self.get_height_modulation(p.y)
    }

    pub fn xz_density(&self, p: Vec2) -> f32 {
        let r0 = self.component.radial_extent;
        let inner = self.component.radial_dropoff; // central falloff parameter

        let d = p.length() / self.galaxy.radius; // distance to galactic central axis

        // this paramater is called scale in the reference codebase
        let central_falloff = (smoothstep(0.0, 1.0 * inner, d)).powi(4);
        let r = self.get_radial_intensity(d, r0);

        // I think the component winding_factor is only meant to apply to noise?
        let winding = self.rad_winding(d); // * self.component.winding_factor;
        let arm_mod = self.arms_modifier(winding,p);

        central_falloff * arm_mod * r
    }
}
