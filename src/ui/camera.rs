use crate::prelude::*;
use bevy::{
    input::mouse::MouseWheel,
    prelude::*,
    render::extract_component::{ExtractComponent, ExtractComponentPlugin},
};

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera)
            .add_systems(PostUpdate, camera_control_system)
            .add_plugins(ExtractComponentPlugin::<CameraMain>::default());
    }
}

fn spawn_camera(mut commands: Commands, mut clearcolor: ResMut<ClearColor>) {
    *clearcolor = ClearColor(Color::BLACK);
    commands.spawn((
        // NEED TO SET CLEAR COLOR TO BLACK...
        Camera3d { ..default() },
        Camera { ..default() },
        Transform::from_xyz(10.0, 12.0, 16.0).looking_at(Vec3::ZERO, Vec3::Y),
        CameraMain::default(),
        volume_upscaler::BackgroundCamera,
    ));
}

#[derive(Component, Clone, ExtractComponent)]
pub struct CameraMain {
    target_pos: Vec3,
    galaxy_radius : f32,
    max_zoom_scale : f32,
    zoom: f32,
    side_view : bool,
    smooth_zoom_buffer: f32,
    far_view : bool,
    drag_origin: Option<Vec3>,
    pub translation: Vec3,
}

impl Default for CameraMain {
    fn default() -> Self {
        Self {
            target_pos: Vec3::new(0.0, 0., 0.0),
            galaxy_radius : 1.0,
            zoom: 1.0,
            max_zoom_scale : 4.0,
            side_view : false,
            smooth_zoom_buffer: 0.0,
            far_view : false,
            drag_origin: None,
            translation: Vec3::ZERO,
        }
    }
}

impl CameraMain {
    fn adjusted_zoom(&self) -> f32 {
        let base_scale = if self.far_view { 10.0 } else { 1.0 }; 
        let min_zoom = 25.0 * base_scale;
        let max_zoom = self.galaxy_radius *  base_scale * self.max_zoom_scale;

        let min_factor = f32::log10(min_zoom);
        let max_factor = f32::log10(max_zoom);
        let zoom_as_factor = f32::lerp(min_factor,max_factor,self.zoom);
        10.0f32.powf(zoom_as_factor)
    }

    fn translation(&self) -> Vec3 {
        let adjusted_scale = self.adjusted_zoom();

        if self.side_view {
            let antitilt = 0.25;
            self.target_pos + Vec3::new(0., adjusted_scale * antitilt, -adjusted_scale)
        } else {
            let antitilt = 0.6;
            self.target_pos + Vec3::new(0., adjusted_scale, -adjusted_scale * antitilt)
        }
    }

    fn look_pos(&self) -> Vec3 {
        self.target_pos
    }

    fn smooth_constrain(&mut self) {        
        let d = self.target_pos.xz().length();
        if d > self.galaxy_radius {
            // Constrain the rate of change to get a gradual transition when stopping dragging
            let fac = (self.galaxy_radius /d).max(0.975);
            self.target_pos *= fac;
        }
    }

    fn set_transform(&mut self, transform : &mut Transform) {
        self.translation = self.translation();
        transform.translation = self.translation;
        transform.look_at(self.look_pos(), Vec3::Y);
    }
}

use bevy::input::mouse::MouseScrollUnit;

pub fn camera_control_system(
    mut query: Query<(&mut Camera, &mut Transform, &mut CameraMain)>,
    windows: Query<&Window>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    galaxy_config: Res<GalaxyConfig>,
    mut scroll_evr: EventReader<MouseWheel>,
    //mut gizmos : Gizmos,
) {
    let galaxy_scale = galaxy_config.radius * 2.5;
    let (mut cam, mut transform, mut camera_main) =
        query.single_mut().expect("Error: Require ONE camera");

    camera_main.galaxy_radius = galaxy_config.radius;

    // HIDE CURSOR
    //windows.single_mut().cursor.visible = false;

    let Ok(window) = windows.single() else {
        return;
    };

    let cursor = window.cursor_position(); // cache this cause we will use it twice
    let mouse_world_pos = cursor
        .and_then(|cursor| {
            cam.viewport_to_world(&GlobalTransform::from(*transform), cursor)
                .ok()
        })
        .and_then(|ray| {
            ray.intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y))
                .map(|distance| ray.get_point(distance))
        });

    if mouse_buttons.pressed(MouseButton::Middle) {
        if camera_main.drag_origin.is_none() {
            camera_main.drag_origin = mouse_world_pos;
        }
    } else {
        camera_main.drag_origin = None;
    }

    // key delta to use later
    let mut key_delta = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        key_delta.z += 1.0;
    }
    if keys.pressed(KeyCode::KeyA) {
        key_delta.x += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) {
        key_delta.z -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) {
        key_delta.x -= 1.0;
    }

    if keys.just_pressed(KeyCode::KeyH) {
        cam.hdr = !cam.hdr;
    }
    if keys.just_pressed(KeyCode::KeyV) {
        camera_main.far_view = !camera_main.far_view;
    }


    camera_main.side_view = keys.pressed(KeyCode::Space);

    let old_zoom = camera_main.zoom;

    // Update
    // scroll delta is cached to a buffer
    // buffer is converted to actual zoom over time for a smooth zooming effect
    for ev in scroll_evr.read() {
        match ev.unit {
            MouseScrollUnit::Line => {
                //camera_main.zoom -= ev.y * 0.05;
                camera_main.smooth_zoom_buffer += ev.y * 0.05;
            }
            MouseScrollUnit::Pixel => {
                //camera_main.zoom -= ev.y * 0.05;
                camera_main.smooth_zoom_buffer += ev.y * 0.05;
            }
        }
    }

    let smooth_zoom_min = 0.001f32;
    let smooth_zoom_factor = 0.2f32;

    let smooth_zoom_amount = if camera_main.smooth_zoom_buffer < 0.0 {
        f32::min(
            camera_main.smooth_zoom_buffer * smooth_zoom_factor,
            (-smooth_zoom_min).max(camera_main.smooth_zoom_buffer),
        )
    } else {
        f32::max(
            camera_main.smooth_zoom_buffer * smooth_zoom_factor,
            smooth_zoom_min.min(camera_main.smooth_zoom_buffer),
        )
    };
    camera_main.zoom -= smooth_zoom_amount;
    camera_main.smooth_zoom_buffer -= smooth_zoom_amount;

    camera_main.zoom = camera_main.zoom.clamp(0., 1.);
    let tzoom = camera_main.zoom * 0.85 + 0.15;
    let speed: f32 = (tzoom * galaxy_scale) * 0.5 * time.delta_secs();
    camera_main.target_pos += key_delta * speed;

    // Activate the mouse drag system while zooming
    if camera_main.zoom != old_zoom && camera_main.drag_origin.is_none() {
        camera_main.drag_origin = mouse_world_pos;
    }
    // apply key delta  to drag origin so keyboard movement works as expected during drag
    if let Some(drag) = camera_main.drag_origin {
        camera_main.drag_origin = Some(drag + key_delta * speed);
    } else {
        // if not dragging, constrain camera target to the galaxy radius 
        // -- Could do this when dragging too, but I find this has behaviour overall more pleasant
        camera_main.smooth_constrain();

    }

    camera_main.set_transform(&mut transform);
    for _i in 0..2 {
        let Some(mouse_pos) = cursor
            .and_then(|cursor| {
                cam.viewport_to_world(&GlobalTransform::from(*transform), cursor)
                    .ok()
            })
            .and_then(|ray| {
                ray.intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y))
                    .map(|distance| ray.get_point(distance))
            })
        else {
            return;
        };

        if let Some(drag_origin) = camera_main.drag_origin {
            let drag_offset = drag_origin - mouse_pos;

            camera_main.target_pos += drag_offset;
        }
        
        camera_main.set_transform(&mut transform);
    }
}
