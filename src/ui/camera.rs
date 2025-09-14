use crate::{galaxy::PickableStar, prelude::*};
use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        view::NoIndirectDrawing,
    },
};

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera)
            .add_observer(camera_mode_system)
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
        NoIndirectDrawing, // req for custom instancing support
    ));
}

#[derive(Clone)]
enum CameraMode {
    Galaxy,
    Star,
}

#[derive(Component, Clone, ExtractComponent)]
pub struct CameraMain {
    mode: CameraMode,
    pub mode_transition: f32,
    target_pos: Vec3,
    galaxy_radius: f32,

    system_center: Option<Vec3>,
    system_radius: f32,

    galaxy_zoom: f32,
    system_zoom: f32,

    side_view: bool,
    smooth_zoom_buffer: f32,
    far_view: bool,
    drag_origin: Option<Vec3>,
    pub translation: Vec3,
}

impl Default for CameraMain {
    fn default() -> Self {
        Self {
            mode: CameraMode::Galaxy,
            mode_transition: 0.0,
            target_pos: Vec3::new(0.0, 0., 0.0),

            galaxy_radius: 1.0,
            system_radius: 1.0,

            galaxy_zoom: 1.0,
            system_zoom: 1.0,

            side_view: false,
            system_center: None,
            smooth_zoom_buffer: 0.0,
            far_view: false,
            drag_origin: None,
            translation: Vec3::ZERO,
        }
    }
}

impl CameraMain {
    const MAX_ZOOM_SCALE: f32 = 4.0;
    const MODE_TRANSITION_SPEED: f32 = 4.0;
    const CAMERA_TILT_FACTOR: f32 = 0.25;

    // returns true if zoom changed
    fn update_zoom(&mut self, input: f32) -> bool {
        if let CameraMode::Star = self.mode {
            if self.system_zoom == 1.0 && input < 0.0 {
                self.mode = CameraMode::Galaxy;
                self.target_pos = self.system_center.unwrap_or(self.target_pos);
                self.galaxy_zoom = 0.0;
            }
        }

        self.smooth_zoom_buffer += input * 0.05;
        let smooth_zoom_min = 0.001f32;
        let smooth_zoom_factor = 0.2f32;

        let smooth_zoom_amount = if self.smooth_zoom_buffer < 0.0 {
            f32::min(
                self.smooth_zoom_buffer * smooth_zoom_factor,
                (-smooth_zoom_min).max(self.smooth_zoom_buffer),
            )
        } else {
            f32::max(
                self.smooth_zoom_buffer * smooth_zoom_factor,
                smooth_zoom_min.min(self.smooth_zoom_buffer),
            )
        };

        match self.mode {
            CameraMode::Star => {
                let old_zoom = self.system_zoom;
                self.system_zoom -= smooth_zoom_amount;
                self.smooth_zoom_buffer -= smooth_zoom_amount;

                self.system_zoom = self.system_zoom.clamp(0., 1.);

                return self.system_zoom != old_zoom;
            }
            CameraMode::Galaxy => {
                let old_zoom = self.galaxy_zoom;
                self.galaxy_zoom -= smooth_zoom_amount;
                self.smooth_zoom_buffer -= smooth_zoom_amount;

                self.galaxy_zoom = self.galaxy_zoom.clamp(0., 1.);

                return self.galaxy_zoom != old_zoom;
            }
        }
    }

    fn apply_keyboard_pan(&mut self, key_delta: Vec3, delta_secs: f32) -> Vec3 {
        let speed: f32 = self.adjusted_zoom() * 1.25 * delta_secs;
        let pan_offset = key_delta * speed;
        self.target_pos += pan_offset;
        pan_offset
    }

    fn update_transition(&mut self, delta_secs: f32) {
        match self.mode {
            CameraMode::Galaxy => {
                self.mode_transition =
                    0.0f32.max(self.mode_transition - delta_secs * Self::MODE_TRANSITION_SPEED)
            }
            CameraMode::Star => {
                self.mode_transition =
                    1.0f32.min(self.mode_transition + delta_secs * Self::MODE_TRANSITION_SPEED)
            }
        }
    }

    fn curve_function(edge0: f32, edge1: f32, interpolant: f32) -> f32 {
        let min_factor = f32::log10(edge0);
        let max_factor = f32::log10(edge1);
        let factor = f32::lerp(min_factor, max_factor, interpolant);
        10.0f32.powf(factor)
    }

    fn adjusted_zoom(&self) -> f32 {
        let galaxy_zoom = {
            let base_scale = if self.far_view { 10.0 } else { 1.0 };
            let min_zoom = 25.0 * base_scale;
            let max_zoom = self.galaxy_radius * base_scale * Self::MAX_ZOOM_SCALE;
            Self::curve_function(min_zoom, max_zoom, self.galaxy_zoom)
        };
        let system_zoom = {
            let base_scale = if self.far_view { 10.0 } else { 1.0 };
            let min_zoom = self.system_radius * base_scale;
            let max_zoom = self.system_radius * base_scale * Self::MAX_ZOOM_SCALE;
            Self::curve_function(min_zoom, max_zoom, self.system_zoom)
        };
        galaxy_zoom.lerp(system_zoom, self.mode_transition)
    }

    fn translation(&self) -> Vec3 {
        let adjusted_scale = self.adjusted_zoom();

        self.look_pos()
            + if self.side_view {
                Vec3::new(
                    0.,
                    adjusted_scale * Self::CAMERA_TILT_FACTOR,
                    -adjusted_scale,
                )
            } else {
                Vec3::new(
                    0.,
                    adjusted_scale,
                    -adjusted_scale * Self::CAMERA_TILT_FACTOR,
                )
            }
    }

    fn look_pos(&self) -> Vec3 {
        self.target_pos.lerp(
            self.system_center.unwrap_or(self.target_pos),
            self.mode_transition,
        )
    }

    fn smooth_constrain(&mut self) {
        let d = self.target_pos.xz().length();
        if d > self.galaxy_radius {
            // Constrain the rate of change to get a gradual transition when stopping dragging
            let fac = (self.galaxy_radius / d).max(0.975);
            self.target_pos *= fac;
        }
    }

    fn set_transform(&mut self, transform: &mut Transform) {
        self.translation = self.translation();
        transform.translation = self.translation;
        transform.look_at(self.look_pos(), Vec3::Y);
    }
}

pub fn camera_mode_system(
    click: Trigger<Pointer<Click>>,
    mut cam_query: Query<&mut CameraMain>,
    // NOTE - Syntax changing to On<Pointer<Click>> in Bevy 0.17.0
    star_query: Query<&Transform, With<PickableStar>>,
) {
    if click.button == PointerButton::Primary {
        let mut camera_main = cam_query.single_mut().expect("Error: Require ONE camera");
        let entity = click.target();
        if let Ok(transform) = star_query.get(entity) {
            camera_main.mode = CameraMode::Star;
            camera_main.system_center = Some(transform.translation);
            // TODO - hook up to actual system radius
            camera_main.system_radius = 1.0;
            camera_main.system_zoom = 1.0;
            camera_main.smooth_zoom_buffer = 0.0;
        }
    }
}

pub fn camera_control_system(
    mut cam_query: Query<(&mut Camera, &mut Transform, &mut CameraMain), Without<PickableStar>>,
    windows: Query<&Window>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    galaxy_config: Res<GalaxyConfig>,
    mut scroll_evr: EventReader<MouseWheel>,
) {
    let (mut cam, mut transform, mut camera_main) =
        cam_query.single_mut().expect("Error: Require ONE camera");

    camera_main.galaxy_radius = galaxy_config.radius;
    camera_main.update_transition(time.delta_secs());

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

    // Update
    // scroll delta is cached to a buffer
    // buffer is converted to actual zoom over time for a smooth zooming effect
    let mut zoom_input: f32 = 0.0;
    for ev in scroll_evr.read() {
        match ev.unit {
            MouseScrollUnit::Line => {
                zoom_input += ev.y;
            }
            MouseScrollUnit::Pixel => {
                zoom_input += ev.y;
            }
        }
    }

    let zoom_changed = camera_main.update_zoom(zoom_input);
    let pan_offset = camera_main.apply_keyboard_pan(key_delta, time.delta_secs());

    // Activate the mouse drag system while zooming
    if zoom_changed && camera_main.drag_origin.is_none() {
        camera_main.drag_origin = mouse_world_pos;
    }
    // apply key delta  to drag origin so keyboard movement works as expected during drag
    if let Some(drag) = camera_main.drag_origin {
        camera_main.drag_origin = Some(drag + pan_offset);
    } else {
        // if not dragging, constrain camera target to the galaxy radius
        // -- Could do this when dragging too, but I find this has behaviour overall more pleasant
        camera_main.smooth_constrain();
    }

    camera_main.set_transform(&mut transform);
    {
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
