use crate::prelude::*;
use bevy::{input::mouse::MouseWheel, prelude::*};

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera)
            .add_systems(PostUpdate, camera_control_system);
    }
}

fn spawn_camera(mut commands: Commands, mut clearcolor: ResMut<ClearColor>) {
    *clearcolor = ClearColor(Color::BLACK);
    commands.spawn((
        // NEED TO SET CLEAR COLOR TO BLACK...
        Camera3d { ..default() },
        Transform::from_xyz(10.0, 12.0, 16.0).looking_at(Vec3::ZERO, Vec3::Y),
        CameraMain::default(),
        volume_upscaler::BackgroundCamera
    ));
}

#[derive(Component, Clone)]
pub struct CameraMain {
    target_pos: Vec3,
    zoom: f32,
    smooth_zoom_buffer: f32,
    drag_origin: Option<Vec3>,
}

impl Default for CameraMain {
    fn default() -> Self {
        Self {
            target_pos: Vec3::new(0.0, 0., 0.0),
            zoom: 1.0,
            smooth_zoom_buffer: 0.0,
            drag_origin: None,
        }
    }
}

impl CameraMain {
    fn translation(&self, galaxy_scale: f32) -> Vec3 {
        let galaxy_zoom = self.zoom * 0.85 + 0.15;
        let adjusted_scale = galaxy_scale * galaxy_zoom;

        let antitilt = 0.6;
        self.look_pos() + Vec3::new(0., adjusted_scale, -adjusted_scale * antitilt)
    }

    fn look_pos(&self) -> Vec3 {
        self.target_pos
    }
}

use bevy::input::mouse::MouseScrollUnit;

pub fn camera_control_system(
    mut query: Query<(&Camera, &mut Transform, &mut CameraMain)>,
    windows: Query<&Window>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    galaxy_config: Res<GalaxyConfig>,
    mut scroll_evr: EventReader<MouseWheel>,
    //mut gizmos : Gizmos,
) {
    let galaxy_scale = galaxy_config.radius * 2.5;
    let (cam, mut transform, mut camera_main) =
        query.single_mut().expect("Error: Require ONE camera");

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
    }

    let d = camera_main.target_pos.xz().length();
    if d > galaxy_config.radius {
        camera_main.target_pos *= galaxy_config.radius / d;
    }

    //

    for _i in 0..2 {
        transform.translation = camera_main.translation(galaxy_scale);
        transform.look_at(camera_main.look_pos(), Vec3::Y);

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

        transform.translation = camera_main.translation(galaxy_scale);
        transform.look_at(camera_main.look_pos(), Vec3::Y);
    }
}
