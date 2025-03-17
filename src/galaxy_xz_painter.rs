use super::GalaxyConfig;
use bevy::prelude::*;
use std::f32::consts::PI;

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let s = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    return s * s * (3.0 - 2.0 * s);
}
pub struct GalaxyPainter {
    radius: f32,
    winding_b: f32,
    winding_n: f32,
    arm_offsets: [f32; 4],
    num_arms: i32,
}

pub struct PaintResult {
    pub intensity : f32,
    pub winding : f32
}

impl GalaxyPainter {
    pub fn from(config: &GalaxyConfig) -> GalaxyPainter {
        GalaxyPainter {
            radius: config.radius,
            winding_b: config.winding_b,
            winding_n: config.winding_n,
            arm_offsets: config.arm_offsets.clone(),
            num_arms: config.n_arms,
        }
    }

    fn get_radial_intensity(&self, distance: f32, r0: f32) -> f32 {
        // Altho this is a virtual function in the reference codebase, I don't think anything overwrites it
        let r = f32::exp(-distance / (r0 * 0.5f32));
        return (r - 0.01f32).clamp(0.0, 1.0);
    }

    fn get_winding(&self, rad: f32) -> f32 {
        let r = rad + 0.05;

        let t = f32::atan(f32::exp(-0.25 / (0.5 * r)) / self.winding_b) * 2.0 * self.winding_n;
        //let t= atan(exp(1.0/r) / wb) * 2.0 * wn;

        return t;
    }

    fn find_theta_difference(&self, t1: f32, t2: f32) -> f32 {
        let v1 = (t1 - t2).abs();
        let v2 = (t1 - t2 - 2.0 * PI).abs();
        let v3 = (t1 - t2 + 2.0 * PI).abs();
        let v4 = (t1 - t2 - 2.0 * PI * 2.0).abs();
        let v5 = (t1 - t2 + 2.0 * PI * 2.0).abs();

        let mut v = f32::min(v1, v2);
        v = f32::min(v, v3);
        v = f32::min(v, v4);
        v = f32::min(v, v5);

        return v;
    }

    fn arm_modifier(&self, p: Vec2, winding: f32, angular_offset: f32, arm_id: i32) -> f32 {
        let aw = 0.1 * (arm_id + 1) as f32;
        let disp = self.arm_offsets[arm_id as usize]; // angular offset

        let theta = -(f32::atan2(p.x, p.y) + angular_offset);

        let v = (self.find_theta_difference(winding, theta + disp)).abs() / PI;

        return (1.0 - v).powf(aw * 15.0);
    }

    fn all_arms_modifier(&self, winding: f32, p: Vec2, angular_offset: f32) -> f32 {
        let mut v : f32 = 0.0;
        for i in 0..self.num_arms {
            v = v.max(self.arm_modifier(p, winding, angular_offset, i));
        }
        return v;
    }

    pub fn get_xz_intensity(&self, p: Vec2, angular_offset: f32) -> PaintResult {
        let r0 = 0.5;
        let inner = 0.1; // central falloff parameter

        let d = p.length() / self.radius; // distance to galactic central axis

        // this paramater is called scale in the reference codebase
        let central_falloff = (smoothstep(0.0, 1.0 * inner, d)).powf(4.0);
        let r = self.get_radial_intensity(d, r0);

        let winding = self.get_winding(d);
        let arm_mod = self.all_arms_modifier(winding, p, angular_offset);

        return PaintResult {
            intensity : central_falloff * arm_mod * r,
            winding
        };
    }
}
