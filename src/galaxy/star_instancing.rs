use bevy::{
    prelude::*,
    reflect::TypePath,
    render::{
        mesh::MeshTag,
        render_resource::{AsBindGroup, ShaderRef},
    },
};
use crate::prelude::*;
use rand::prelude::*;

const SHADER_ASSET_PATH: &str = "shaders/star_instancing.wgsl";

pub struct StarInstancingPlugin;

impl Plugin for StarInstancingPlugin {
    fn build(&self, app : &mut App) {
        app.add_plugins(MaterialPlugin::<StarInstanceMaterial>::default())
        .add_systems(Startup, setup);
    }
}

/// Sets up an instanced grid of cubes, where each cube is colored based on an image that is
/// sampled in the vertex shader. The cubes are then animated in a spiral pattern.
///
/// This example demonstrates one use of automatic instancing and how to use `MeshTag` to use
/// external data in a custom material. For example, here we use the "index" of each cube to
/// determine the texel coordinate to sample from the image in the shader.
fn setup(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StarInstanceMaterial>>,
    galaxy_config : Res<GalaxyConfig>,
) {
    // We will use this image as our external data for our material to sample from in the vertex shader

    // billboard mesh
    let mesh_handle = meshes.add(Rectangle::from_size(Vec2::splat(2.0)));

    // Create the custom material with a reference to our texture
    // Automatic instancing works with any Material, including the `StandardMaterial`.
    // This custom material is used to demonstrate the optional `MeshTag` feature.
    let material_handle = materials.add(StarInstanceMaterial {
        alpha_mode : AlphaMode::Add
        //image: image.clone(),
    });

    for index in 0..65536 {
        let pos = sample_star_pos(&galaxy_config);

        commands.spawn((
            // For automatic instancing to take effect you need to
            // use the same mesh handle and material handle for each instance
            Mesh3d(mesh_handle.clone()),
            MeshMaterial3d(material_handle.clone()),
            // This is an optional component that can be used to help tie external data to a mesh instance
            MeshTag(index),
            Transform::from_translation(pos),
        ));
    }
}

fn sample_unit_circle(rng : &mut ThreadRng) -> Vec2 {
    let length = rng.random::<f32>().sqrt();
    let angle = std::f32::consts::PI * rng.random_range(0.0..2.0);

    vec2(angle.cos(),angle.sin()) * length
}

fn sample_pos(rng : &mut ThreadRng, radius : f32) -> Vec3 {
    let circle_sample = sample_unit_circle(rng) * radius;
    let height_sample: f32 = rng.random_range(-2.0..2.0);
    
    //height_sample /= height_sample.abs().sqrt();

    vec3(circle_sample.x,height_sample,circle_sample.y)
}

fn sample_star_pos(galaxy_config : &GalaxyConfig) -> Vec3 {
    let mut rng = rand::rng();

    let arm_painter = super::ArmLutGenerator::new(galaxy_config,&galaxy_config.disk_params);

    let current_pos = sample_pos(&mut rng, galaxy_config.radius);
    let mut best = current_pos;
    let weight = arm_painter.get_xyz_intensity(current_pos);
    let mut weight_sum = weight;

    for _ in 0..100 {
        let current_pos = sample_pos(&mut rng, galaxy_config.radius);
        let weight = arm_painter.get_xyz_intensity(current_pos);
        weight_sum += weight;

        if rng.random::<f32>() < weight / weight_sum {
            best = current_pos;
        }
    }

    best
}

// This struct defines the data that will be passed to your shader
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct StarInstanceMaterial {
   // #[texture(0)]
    //#[sampler(1)]
    //image: Handle<Image>,
    alpha_mode : AlphaMode
}

impl Material for StarInstanceMaterial {
    fn vertex_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        self.alpha_mode
    }
}
