use crate::prelude::*;
use crate::ui::CameraMain;
use bevy::{
    math::bounding::{BoundingSphere, RayCast3d},
    picking::{
        backend::{ray::RayMap, HitData, PointerHits},
        hover::PickingInteraction,
        PickSet,
    },
    prelude::*,
};

pub struct StarPickingPlugin;

impl Plugin for StarPickingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, update_hits.in_set(PickSet::Backend))
            .add_systems(Update, picking_test_gizmo);
    }
}

fn picking_test_gizmo(
    stars: Query<(&Transform, &PickingInteraction), With<PickableStar>>,
    mut gizmos: Gizmos,
) {
    for (transform, interaction) in stars {
        match interaction {
            PickingInteraction::Pressed => {
                gizmos.sphere(transform.translation, 7.0, Color::srgba(0.0, 1.0, 0.0, 1.0));
            }
            PickingInteraction::Hovered => {
                gizmos.sphere(transform.translation, 7.0, Color::srgba(1.0, 0.0, 0.0, 1.0));
            }
            PickingInteraction::None => {}
        }
    }
}

#[derive(Component)]
pub struct PickableStar;

fn ray_sphere_intersection(ray: Ray3d, center: Vec3, radius: f32) -> Option<(f32, Vec3, Vec3)> {
    let raycast = RayCast3d::from_ray(ray, 10000.);
    let sphere = BoundingSphere::new(center, radius);

    let depth = raycast.sphere_intersection_at(&sphere);

    if let Some(depth) = depth {
        let point = ray.get_point(depth);
        let normal = (point - center).normalize();

        Some((depth, point, normal))
    } else {
        None
    }
}

pub fn update_hits(
    ray_map: Res<RayMap>,
    mut output: EventWriter<PointerHits>,
    picking_cameras: Query<&Camera, With<CameraMain>>,
    stars: Query<(Entity, &Transform), With<PickableStar>>,
    galaxy_config: Res<GalaxyConfig>,
) {
    let selection_radius = galaxy_config.major_stars_spacing / 2.0;
    for (&ray_id, &ray) in ray_map.iter() {
        let Ok(camera) = picking_cameras.get(ray_id.camera) else {
            continue;
        };
        let picks: Vec<(Entity, HitData)> = stars
            .iter()
            .filter_map(|(entity, transform)| {
                if let Some((depth, point, normal)) =
                    ray_sphere_intersection(ray, transform.translation, selection_radius)
                {
                    Some((
                        entity,
                        HitData {
                            camera: ray_id.camera,
                            position: Some(point),
                            depth,
                            normal: Some(normal),
                        },
                    ))
                } else {
                    None
                }
            })
            .collect();

        let order = camera.order as f32;

        if !picks.is_empty() {
            output.write(PointerHits::new(ray_id.pointer, picks, order));
        }
    }
}
