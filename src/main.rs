// TODO: Formalize Physics
//       * Goals:
//         * Render-decoupled physics
//           * Needed to allow responsive framerates during heavy time-jump simulation
//           * How to achieve?
//             * Can't treat as an exclusive system, as that's synchronous
//               * i.e. Can't take a mutable world reference, needs a buffered solution
//             * Use a similar pattern to bevy-render?
//               * Specialized subapp world
//               * Copy data back and forth during specific stages
//             * Customized rapier setup?
//               * Rapier already reads world data into its own strata
//               * Could this be achieved with some sort of custom async update system?
//                 * Some way to spawn its systems off onto the async task pool
//                 * Simplicity is preferable
//             * Testing: Running a schedule in the async task pool is nonviable
//               * Regular long-running async tasks work as expected,
//                 but nested schedules end up spawning on the main thread and blocking
//               * Conclusion: Need to manually spawn rapier systems as tasks
//         * Separation between physics animations and regular animations
//           * Currently simulating entire scene on each physics step
//             * Not necessary, as almost none of it is relevant to physics
//           * How to split out?
//             * Currently swapping global animation update responsibility between
//               render and physics updates
//             * Probably wiser to make a distinction and use system labels to separate
//               * Multiple system label support; single-label is currently implicit
//                 * ex. Per-animation run criteria take a single label
//                 * Should be able to use system piping to create AND / OR combinators
//
// TODO: Physics interpolation
//       * Interpolate between previous and current physics frames in main world
//         * Adds one frame of game-tick lag, but allows arbitrary framerates
//

pub mod animation;
pub mod fixed_tick;
pub mod fork_system;
pub mod image_loader;
pub mod lift_system;
pub mod npbr;
pub mod physics;
pub mod timeline;
pub mod ui;

use animation::{AnimationPlugin, AnimationSchedule};
use bevy::{
    core_pipeline::clear_color::ClearColorConfig,
    diagnostic::FrameTimeDiagnosticsPlugin,
    prelude::*,
    render::{
        mesh::Indices,
        render_resource::{
            AddressMode, Extent3d, FilterMode, PrimitiveTopology, SamplerDescriptor,
            TextureDimension,
        },
        texture::ImageSampler,
    },
    window::PresentMode,
};
use bevy_inspector_egui::quick::AssetInspectorPlugin;
use bevy_rapier3d::{
    prelude::{Collider, RapierConfiguration, RigidBody, Sensor, TimestepMode, Vect},
    render::RapierDebugRenderPlugin,
};
use image_loader::{ImageLoader, ImageLoaderPlugin};
use internal_assets::InternalAssetsPlugin;
use material_loader::{MaterialLoader, MaterialLoaderPlugin};
use npbr::{
    palette::PaletteInput,
    palette_lighting::{
        AlphaFunction, DitherCoordFunction, DitherFunction, PaletteCoordFunction,
        PaletteLightingMaterial, PaletteLightingMeshBundle, PaletteLightingPlugin,
        SdfGeometryFunction,
    },
    NpbrPlugin,
};
use physics::{
    extract_component::ExtractComponentPlugin, extract_param::Extract, LerpTransform, PhysicsApp,
    PhysicsAppBuilder, PhysicsPlugin, PhysicsStage,
};
use std::{
    borrow::Cow,
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};
use timeline::TimelinePlugin;
use ui::UiPlugin;

use crate::{
    animation::{read_animation_storage, write_animation_storage, AnimationStorage},
    lift_system::IntoLiftSystem,
    npbr::{
        dither::DitherInput,
        palette_lighting::PaletteLightingShader,
        sdf::sdf_3d::{PositionFunction, Sdf3dModule, UvFunction},
        shader_composer::ShaderComposer,
    },
    timeline::TimelineComponent,
};

pub const FIXED_TICK_RATE: f64 = 1.0 / 4.0;

pub mod internal_assets {
    use bevy::{
        asset::Asset,
        prelude::{App, Deref, DerefMut, HandleUntyped, Plugin, Resource},
    };

    pub struct InternalAssetsPlugin;

    impl Plugin for InternalAssetsPlugin {
        fn build(&self, app: &mut App) {
            #[cfg(debug_assertions)]
            {
                app.init_resource::<InternalAssets>();
            }
        }
    }

    #[derive(Debug, Default, Clone, Deref, DerefMut, Resource)]
    pub struct InternalAssets {
        pub assets: Vec<HandleUntyped>,
    }

    pub fn register_internal_asset<T>(app: &mut App, file_str: &'static str, path_str: &'static str)
    where
        T: Asset,
    {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let manifest_dir_path = std::path::Path::new(&manifest_dir);
        let asset_server = app.world.resource::<bevy::prelude::AssetServer>();
        let asset_io = asset_server
            .asset_io()
            .downcast_ref::<bevy::asset::FileAssetIo>()
            .expect("The debug AssetServer only works with FileAssetIo-backed AssetServers");
        let absolute_file_path = manifest_dir_path.join(
            std::path::Path::new(file_str)
                .parent()
                .expect("file path must have a parent"),
        );
        let asset_folder_relative_path = absolute_file_path
            .strip_prefix(asset_io.root_path())
            .expect("The AssetIo root path should be a prefix of the absolute file path");

        let path = std::path::Path::new(path_str);
        let mut handle = app
            .world
            .resource::<bevy::prelude::AssetServer>()
            .load::<T, _>(asset_folder_relative_path.join(path));
        handle.make_strong(&app.world.resource_mut::<bevy::prelude::Assets<T>>());
        app.world
            .resource_mut::<InternalAssets>()
            .push(handle.into());
    }

    #[macro_export]
    macro_rules! load_internal_asset {
        ($app: ident, $handle: ident, $path_str: expr, $ty: ty, $loader: expr) => {
            #[cfg(debug_assertions)]
            {
                $crate::internal_assets::register_internal_asset::<$ty>($app, file!(), $path_str)
            }

            #[cfg(not(debug_assertions))]
            {
                bevy::asset::load_internal_asset!($app, $handle, $path_str, $loader);
            }
        };
    }
}

fn main() {
    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                asset_folder: "".into(),
                watch_for_changes: true,
                ..default()
            })
            .set(WindowPlugin {
                window: WindowDescriptor {
                    title: "Bevy Test".into(),
                    present_mode: PresentMode::AutoVsync,
                    ..default()
                },
                ..default()
            }),
    );

    app.add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(InternalAssetsPlugin)
        .add_plugin(AnimationPlugin {
            system_stage: CoreStage::Update,
        })
        .add_plugin(UiPlugin)
        .add_plugin(AssetInspectorPlugin::<PaletteLightingMaterial>::default())
        .add_plugin(TimelinePlugin);

    app.insert_resource(Msaa { samples: 1 });

    app.insert_resource(
        PhysicsAppBuilder::<()>::default()
            .map(|app| {
                app.add_plugin(AnimationPlugin {
                    system_stage: PhysicsStage::PrePhysics,
                });
            })
            .build(),
    )
    .insert_resource(RapierConfiguration {
        gravity: Vect::ZERO,
        timestep_mode: TimestepMode::Fixed {
            dt: 1.0 / 60.0,
            substeps: 1,
        },
        ..default()
    })
    .add_plugin(PhysicsPlugin::<()>::default())
    .add_plugin(RapierDebugRenderPlugin::default())
    .add_startup_system(|mut physics_app: ResMut<PhysicsApp>| {
        physics_app.schedule.add_system_to_stage(
            PhysicsStage::Extract,
            |mut commands: Commands, query: Extract<Query<(Entity, &TimelineComponent)>>| {
                for (entity, timeline) in query.iter() {
                    let mut commands = commands.get_or_spawn(entity);
                    let mut timeline = *timeline;

                    // FIXME: These need to be multiplied by the current tick rate
                    //        Will likely need a fixed-tick resource to exist inside PhysicsApp::world
                    //        Better yet, use RapierConfiguration
                    timeline.timestamp = timeline.timestamp.floor();
                    timeline.prev_timestamp = timeline.prev_timestamp.floor();
                    commands.insert(timeline);
                }
            },
        );
    })
    .add_plugin(ExtractComponentPlugin::<Torus>::default());

    app.add_plugin(ImageLoaderPlugin)
        .add_plugin(MaterialLoaderPlugin::<PaletteLightingMaterial>::default())
        .add_plugin(NpbrPlugin)
        .add_plugin(PaletteLightingPlugin);

    app.add_startup_system(setup_scene);

    app.add_system(key_input);

    app.run();
}

#[derive(Debug, Default, Copy, Clone, Component)]
struct Row;

#[derive(Debug, Default, Copy, Clone, Component)]
struct Sphere;

#[derive(Debug, Default, Copy, Clone, Component)]
struct Cube;

#[derive(Debug, Default, Copy, Clone, Component)]
pub struct Torus;

#[derive(Debug, Default, Copy, Clone, Component)]
pub struct Quad;

const SAMPLER_PALETTE: SamplerDescriptor = SamplerDescriptor {
    address_mode_u: AddressMode::ClampToEdge,
    address_mode_v: AddressMode::ClampToEdge,
    address_mode_w: AddressMode::ClampToEdge,
    mag_filter: FilterMode::Nearest,
    min_filter: FilterMode::Nearest,
    mipmap_filter: FilterMode::Nearest,
    label: Some("Palette"),
    lod_min_clamp: 0.0,
    lod_max_clamp: std::f32::MAX,
    compare: None,
    anisotropy_clamp: None,
    border_color: None,
};

const SAMPLER_DITHER: SamplerDescriptor = SamplerDescriptor {
    address_mode_u: AddressMode::Repeat,
    address_mode_v: AddressMode::Repeat,
    address_mode_w: AddressMode::Repeat,
    mag_filter: FilterMode::Nearest,
    min_filter: FilterMode::Nearest,
    mipmap_filter: FilterMode::Nearest,
    label: Some("Sampler"),
    lod_min_clamp: 0.0,
    lod_max_clamp: std::f32::MAX,
    compare: None,
    anisotropy_clamp: None,
    border_color: None,
};

const SDF_GEOMETRY_DISCARD: SdfGeometryFunction = SdfGeometryFunction(Cow::Borrowed(
    r#"
    let sdf_output = sdf_geometry(in, true);

    if !sdf_output.hit {
        discard;
    }

    return sdf_output;
"#,
));

const SDF_GEOMETRY_RETAIN: SdfGeometryFunction =
    SdfGeometryFunction(Cow::Borrowed("return sdf_geometry(in, true);"));

const SDF_GEOMETRY_RETAIN_WORLD: SdfGeometryFunction =
    SdfGeometryFunction(Cow::Borrowed("return sdf_geometry(in, false);"));

const SDF_NORMAL_SPHERE: &'static str = r#"
    return sdf_3d_normal_sphere(p);
"#;

const SDF_NORMAL_ESTIMATE: &'static str = r#"
    return sdf_3d_normal_estimate(p);
"#;

const SDF_UV_SPHERE: &'static str = r#"
    return sdf_3d_uv_sphere(
        in.world_normal,
    );
"#;

const SDF_UV_TRIPLANAR: &'static str = r#"
    return sdf_3d_uv_triplanar(
        in.world_position.xyz - mesh.model.w.xyz,
        in.world_normal,
        4.0,
    );
"#;

const DITHER_COORD_SCREEN: DitherCoordFunction = DitherCoordFunction(Cow::Borrowed(
    r#"
    var dither_uv = coords_to_viewport_uv(
        in.frag_coord.xy,
        view.viewport
    );
    let screen_offset = screen_position(
        view.view_proj * mesh.model,
        vec3<f32>(0.0)
    ).xy * -dither_uniform.scroll_factor;
    dither_uv += screen_offset;
    return dither_uv;
"#,
));

const DITHER_COORD_SKYBOX: DitherCoordFunction = DitherCoordFunction(Cow::Borrowed(
    r#"
    var dither_uv = coords_to_viewport_uv(
        in.frag_coord.xy,
        view.viewport
    );
    let screen_pos = screen_position(
        view.view_proj,
        vec3<f32>(0.0, 0.0, 10000000.0)
    );
    let screen_pos = screen_pos * 2.0 - 1.0;
    dither_uv -= screen_pos.xy * dither_uniform.scroll_factor * 0.5;
    return dither_uv;
"#,
));

const DITHER_COORD_UV: DitherCoordFunction = DitherCoordFunction(Cow::Borrowed(
    r#"
    return vec3<f32>(in.uv.x, in.uv.y, 0.0);
"#,
));

const DITHER_TEXTURE: DitherFunction = DitherFunction(Cow::Borrowed(
    r#"
    return dithering_texture(dither_texture, dither_sampler, dither_uv, view.viewport.zw).r;
"#,
));

const DITHER_MANHATTAN: DitherFunction = DitherFunction(Cow::Borrowed(
    r#"
    return dithering_manhattan(
        in.uv,
        vec2<f32>(1.0),
        16.0,
    ) + dithering_texture(
        dither_texture,
        dither_sampler,
        dither_uv,
        view.viewport.zw,
    ).r;
"#,
));

const PALETTE_COORD_UV: PaletteCoordFunction = PaletteCoordFunction(Cow::Borrowed(
    r#"
    let dim = palette_dim(palette_texture);
    return vec3<f32>(in.uv.x, in.uv.y, palette_input.color / dim.z);
"#,
));

const PALETTE_COORD_SKYBOX: PaletteCoordFunction = PaletteCoordFunction(Cow::Borrowed(
    r#"
    let fragment_position_view_lh = normalize(
        in.world_position.xyz * vec3<f32>(1.0, 1.0, -1.0)
    );

    let ofs = fragment_position_view_lh.y / (PI * 0.5);

    let dim = palette_dim(palette_texture);

    return vec3<f32>(
        palette_input.brightness,
        0.3 + ofs,
        palette_input.color / dim.z,
    );
"#,
));

const PALETTE_COORD_LIGHT: PaletteCoordFunction = PaletteCoordFunction(Cow::Borrowed(
    r#"
    // Calculate PBR lighting
    let pbr = pbr_lighting(
        base_material.base,
        in.frag_coord,
        in.world_position,
        in.world_normal,
        in.uv,
    #ifdef VERTEX_TANGENTS
        in.world_tangent,
    #endif
        in.is_front,
    );

    // Map to palette lighting
    return palette_lighting(
        palette_texture,
        pbr,
        palette_input,
        palette_lighting_input,
    );
"#,
));

const PALETTE_COORD_SDF: PaletteCoordFunction = PaletteCoordFunction(Cow::Borrowed(
    r#"
    let p = in.uv * 2.0 - 1.0;
    let dim = palette_dim(palette_texture);

    return vec3<f32>(
        palette_input.brightness,
        smoothstep(
            0.8,
            0.0,
            min(
                min(
                    sdf_2d_circle(p, 0.1),
                    sdf_2d_chebyshev(p, 1.0)
                ),
                sdf_2d_manhattan(p, 1.5)
            )
        ),
        palette_input.color / dim.z,
    );
"#,
));

// Overwrite palette alpha with a linearized fresnel gradient
const ALPHA_FRESNEL: AlphaFunction = AlphaFunction(Cow::Borrowed(
    r#"
    return a
        * fresnel_linear(in.world_position, in.world_normal, view.view)
        + map_range(dither, 0.0, 1.0, 0.8, 1.0);
"#,
));

const ALPHA_SDF_ANTIALIAS: AlphaFunction = AlphaFunction(Cow::Borrowed(
    r#"
    return map_range(fresnel_linear(in.world_position, in.world_normal, view.view), 0.0, 0.1, 0.0, 1.0);
"#,
));

const ALPHA_SDF_2D: AlphaFunction = AlphaFunction(Cow::Borrowed(
    r#"
    let p = in.uv * 2.0 - 1.0;
    return smoothstep(
        0.6,
        0.4,
        min(
            min(
                sdf_2d_circle(p, 0.1),
                sdf_2d_chebyshev(p, 1.0)
            ),
            sdf_2d_manhattan(p, 1.5)
        )
    );
"#,
));

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut shaders: ResMut<Assets<Shader>>,
    materials: Res<Assets<PaletteLightingMaterial>>,
    mut animations: ResMut<AnimationSchedule>,
    mut physics_app: ResMut<PhysicsApp>,
    mut image_loader: ResMut<ImageLoader>,
    mut material_loader: ResMut<MaterialLoader<PaletteLightingMaterial>>,
    type_registry: Res<AppTypeRegistry>,
    asset_server: Res<AssetServer>,
) {
    let palette_combined =
        image_loader.load_with(&asset_server, "assets/palette/Combined.png", |image| {
            image.reinterpret_size(Extent3d {
                width: image.texture_descriptor.size.width,
                height: image.texture_descriptor.size.height / 6,
                depth_or_array_layers: 6,
            });
            image.texture_descriptor.dimension = TextureDimension::D3;
        });

    let noise_bayer = image_loader.load_with_sampler(
        &asset_server,
        "assets/dither/Bayer.512x512.png",
        ImageSampler::Descriptor(SAMPLER_DITHER),
    );

    let noise_blue = image_loader.load_with_sampler(
        &asset_server,
        "assets/dither/Blue Noise.512x512.png",
        ImageSampler::Descriptor(SAMPLER_DITHER),
    );

    let mut physics_animations = physics_app.world.resource_mut::<AnimationSchedule>();

    physics_animations.add(
        "torus",
        |timeline: Query<&TimelineComponent>, mut query: Query<&mut Transform, With<Torus>>| {
            let timeline = timeline.get_single().unwrap();

            for mut transform in query.iter_mut() {
                transform.translation.z = -timeline.timestamp as f32;
                transform.rotation =
                    Quat::from_euler(EulerRot::XYZ, 0.0, 0.0, timeline.timestamp as f32);
            }
        },
    );

    // Spawn elapsed time storage
    let time = commands
        .spawn((
            AnimationStorage::<f32>::default(),
            TimelineComponent::default(),
        ))
        .id();

    // Spawn camera
    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                //hdr: true,
                ..default()
            },
            camera_3d: Camera3d {
                clear_color: ClearColorConfig::Custom(Color::BLACK),
                ..default()
            },
            ..default()
        },
        //BloomSettings::default(),
    ));

    animations.add(
        "camera",
        read_animation_storage::<f32>(time)
            .pipe(
                |In(input): In<f32>, mut query: Query<&mut Transform, With<Camera>>| {
                    let input = input * 0.2;
                    for mut transform in query.iter_mut() {
                        *transform = Transform {
                            translation: Vec3::new(
                                input.sin() * -4.0,
                                input.cos() * 5.0,
                                input.cos() * 4.0,
                            ),
                            ..default()
                        }
                        .looking_at(Vec3::new(0.0, 0.0, -5.0), Vec3::Y)
                    }
                },
            )
            .after("time"),
    );

    // Spawn lights
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 5000.0,
            ..default()
        },
        transform: Transform::IDENTITY.looking_at(Vec3::new(1.0, -1.0, -1.0).normalize(), Vec3::Y),
        ..default()
    });

    commands.spawn(PointLightBundle {
        transform: Transform::IDENTITY,
        ..default()
    });

    commands.spawn(SpotLightBundle {
        spot_light: SpotLight {
            intensity: 4000.0,
            range: 40.0,
            ..default()
        },
        transform: Transform::from_xyz(5.0, 2.0, -20.0)
            .looking_at(Vec3::new(5.0, 1.0, -21.0), Vec3::Y),
        ..default()
    });

    // Create materials
    let material_fine_gold = material_loader.load(
        &materials,
        PaletteLightingMaterial {
            dither_input: DitherInput {
                dither_width: Vec3::new(0.0, 0.5, 0.0),
                ..default()
            },
            palette_input: PaletteInput {
                color: 2.0,
                ..default()
            },
            palette_texture: palette_combined.clone(),
            dither_texture: noise_bayer.clone(),
            shader_handle: Some(
                shaders.add(
                    PaletteLightingShader::default()
                        .with_sdf_3d_module(Sdf3dModule {
                            function_position: Some(PositionFunction(
                                "return sdf_3d_torus(p, vec2(0.25, 0.125));".into(),
                            )),
                            ..default()
                        })
                        .with_sdf_geometry_function(SDF_GEOMETRY_DISCARD.into())
                        .with_palette_coord_function(PALETTE_COORD_LIGHT.into())
                        .with_dither_coord_function(DITHER_COORD_SCREEN.into())
                        .with_dither_function(DITHER_TEXTURE.into())
                        .shader(),
                ),
            ),
            base: StandardMaterial {
                cull_mode: None,
                ..default()
            },
            ..default()
        },
    );

    let material_green = material_loader.load(
        &materials,
        PaletteLightingMaterial {
            dither_input: DitherInput {
                dither_width: Vec3::new(1.0, 1.0, 0.0),
                ..default()
            },
            palette_input: PaletteInput {
                color: 4.0,
                ..default()
            },
            palette_texture: palette_combined.clone(),
            dither_texture: noise_blue.clone(),
            shader_handle: Some(
                shaders.add(
                    PaletteLightingShader::default()
                        .with_palette_coord_function(PALETTE_COORD_UV.into())
                        .with_dither_coord_function(DITHER_COORD_SCREEN.into())
                        .with_dither_function(DITHER_TEXTURE.into())
                        .shader(),
                ),
            ),
            ..default()
        },
    );

    let material_2d = material_loader.load(
        &materials,
        PaletteLightingMaterial {
            dither_input: DitherInput {
                dither_width: Vec3::new(0.0, 1.0, 0.0),
                ..default()
            },
            palette_input: PaletteInput {
                color: 3.0,
                ..default()
            },
            palette_texture: palette_combined.clone(),
            dither_texture: noise_bayer.clone(),
            base: StandardMaterial {
                cull_mode: None,
                double_sided: true,
                alpha_mode: AlphaMode::Mask(0.5),
                ..default()
            },
            shader_handle: Some(
                shaders.add(
                    PaletteLightingShader::default()
                        .with_dither_coord_function(DITHER_COORD_SCREEN.into())
                        .with_dither_function(DITHER_TEXTURE.into())
                        .with_palette_coord_function(PALETTE_COORD_SDF.into())
                        .with_alpha_function(ALPHA_SDF_2D.into())
                        .shader(),
                ),
            ),
            ..default()
        },
    );

    let material_blue = material_loader.load(
        &materials,
        PaletteLightingMaterial {
            dither_input: DitherInput {
                dither_width: Vec3::new(0.0, 0.8, 0.0),
                ..default()
            },
            palette_input: PaletteInput {
                color: 0.0,
                ..default()
            },
            palette_texture: palette_combined.clone(),
            dither_texture: noise_blue.clone(),
            shader_handle: Some(
                shaders.add(
                    PaletteLightingShader::default()
                        .with_sdf_3d_module(Sdf3dModule {
                            function_position: Some(PositionFunction(
                                "return sdf_3d_round_cone( p, 0.125, 0.25, 0.5,);".into(),
                            )),
                            ..default()
                        })
                        .with_sdf_geometry_function(SDF_GEOMETRY_DISCARD.into())
                        .with_dither_coord_function(DITHER_COORD_SCREEN.into())
                        .with_dither_function(DITHER_TEXTURE.into())
                        .with_palette_coord_function(PALETTE_COORD_LIGHT.into())
                        .shader(),
                ),
            ),
            base: StandardMaterial {
                //alpha_mode: AlphaMode::Mask(1.0),
                cull_mode: None,
                ..default()
            },
            ..default()
        },
    );

    let material_white = material_loader.load(
        &materials,
        PaletteLightingMaterial {
            dither_input: DitherInput {
                dither_width: Vec3::new(0.0, 0.25, 0.0),
                dither_scale: Vec2::ONE * 50.0,
                ..default()
            },
            palette_input: PaletteInput {
                color: 1.0,
                ..default()
            },
            palette_texture: palette_combined.clone(),
            dither_texture: noise_blue.clone(),
            shader_handle: Some(
                shaders.add(
                    PaletteLightingShader::default()
                        .with_sdf_3d_module(Sdf3dModule {
                            function_position: Some(PositionFunction(
                                "return sdf_3d_sphere(p, 0.5);".into(),
                            )),
                            function_uv: Some(UvFunction(
                                "return sdf_3d_uv_sphere(normalize(p));".into(),
                            )),
                            ..default()
                        })
                        .with_sdf_geometry_function(SDF_GEOMETRY_DISCARD.into())
                        .with_dither_coord_function(DITHER_COORD_SCREEN.into())
                        .with_dither_function(DITHER_MANHATTAN.into())
                        .with_palette_coord_function(PALETTE_COORD_LIGHT.into())
                        .shader(),
                ),
            ),
            ..default()
        },
    );

    // Create meshes
    let mesh_quad = meshes.add(
        shape::Quad {
            size: Vec2::ONE,
            flip: false,
        }
        .into(),
    );
    let mesh_cube = meshes.add(shape::Cube { size: 1.0 }.into());
    let mesh_half_cube = meshes.add(shape::Cube { size: 0.5 }.into());

    // Spawn skybox
    let mesh_skybox = meshes.add(shape::Cube { size: 10000.0 }.into());

    let material_skybox = material_loader.load(
        &materials,
        PaletteLightingMaterial {
            base: StandardMaterial {
                cull_mode: None,
                ..default()
            },
            dither_input: DitherInput {
                dither_width: Vec3::new(0.0, 1.0, 0.0),
                ..default()
            },
            palette_input: PaletteInput {
                color: 5.0,
                ..default()
            },
            palette_texture: palette_combined.clone(),
            dither_texture: noise_blue.clone(),
            shader_handle: Some(
                shaders.add(
                    PaletteLightingShader::default()
                        .with_dither_coord_function(DITHER_COORD_SKYBOX.into())
                        .with_dither_function(DITHER_TEXTURE.into())
                        .with_palette_coord_function(PALETTE_COORD_SKYBOX.into())
                        .shader(),
                ),
            ),
            ..default()
        },
    );

    commands.spawn((
        Cube,
        PaletteLightingMeshBundle {
            mesh: mesh_skybox,
            material: material_skybox,
            ..default()
        },
    ));

    // Spawn terrain
    let extent = 5.0;
    let vertices = [
        ([-extent, 0.0, extent], [0.0, 1.0, 0.0], [1.0, 1.0]),
        ([-extent, 0.0, -extent], [0.0, 1.0, 0.0], [1.0, 0.0]),
        ([extent, 0.0, -extent], [0.0, 1.0, 0.0], [0.0, 0.0]),
        ([extent, 0.0, extent], [0.0, 1.0, 0.0], [0.0, 1.0]),
    ];

    let indices = Indices::U32(vec![0, 2, 1, 0, 3, 2]);

    let positions: Vec<_> = vertices.iter().map(|(p, _, _)| *p).collect();
    let normals: Vec<_> = vertices.iter().map(|(_, n, _)| *n).collect();
    let uvs: Vec<_> = vertices.iter().map(|(_, _, uv)| *uv).collect();

    let mut mesh_terrain = Mesh::new(PrimitiveTopology::TriangleList);
    mesh_terrain.set_indices(Some(indices));
    mesh_terrain.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh_terrain.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh_terrain.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    let mesh_terrain = meshes.add(mesh_terrain);

    let material_terrain = material_loader.load(
        &materials,
        PaletteLightingMaterial {
            dither_input: DitherInput {
                dither_width: Vec3::new(0.0, 1.0, 0.0),
                ..default()
            },
            palette_input: PaletteInput {
                color: 2.0,
                ..default()
            },
            palette_texture: palette_combined.clone(),
            dither_texture: noise_blue.clone(),
            shader_handle: Some(
                shaders.add(
                    PaletteLightingShader::default()
                        .with_sdf_3d_module(Sdf3dModule {
                            function_position: Some(PositionFunction(
                                r#"
                                    let offset = vec3<f32>(0.0, mesh.model.w.y, 0.0);
                                    let scale = vec3<f32>(0.5, -3.0, 0.5);
                                    let uv = p.xz * scale.xz + offset.xz;
                                    let height = gln_perlin(uv) / 2.0 + 0.5;
                                    let height = height * gln_perlin(uv * 0.5) / 2.0 + 0.5;
                                    let height = height * scale.y + offset.y;
                                    return p.y - height;
                                "#
                                .into(),
                            )),
                            ..default()
                        })
                        .with_sdf_geometry_function(SDF_GEOMETRY_RETAIN_WORLD.into())
                        .with_dither_coord_function(DITHER_COORD_SCREEN.into())
                        .with_dither_function(DITHER_TEXTURE.into())
                        .with_palette_coord_function(PALETTE_COORD_LIGHT.into())
                        .shader(),
                ),
            ),
            ..default()
        },
    );

    for x in -10..10 {
        for z in -10..10 {
            commands.spawn(PaletteLightingMeshBundle {
                mesh: mesh_terrain.clone(),
                material: material_terrain.clone(),
                transform: Transform::from_xyz(x as f32 * 10.0, -3.0, z as f32 * 10.0),
                ..default()
            });
        }
    }

    // Spawn rows and spheres
    let x_min = -25;
    let x_max = 25;
    let z_min = -25;
    let z_max = 25;

    let mut rows = vec![];

    for (ix, x) in (x_min..x_max).into_iter().enumerate() {
        let row = commands
            .spawn((
                Row,
                AnimationStorage::<isize>::new(x),
                AnimationStorage::<f32>::default(),
            ))
            .id();

        let mut spheres = vec![];

        for (iz, z) in (z_min..z_max).into_iter().enumerate() {
            let mut hasher = DefaultHasher::new();
            (x, z).hash(&mut hasher);
            let rand = hasher.finish() % 2 == 0;

            let sphere = if rand {
                commands
                    .spawn((
                        Sphere,
                        AnimationStorage::<isize>::new(z),
                        PaletteLightingMeshBundle {
                            mesh: mesh_cube.clone(),
                            material: material_blue.clone(),
                            transform: Transform::from_xyz(x as f32 * 2.0, 0.0, z as f32 * 2.0),
                            ..default()
                        },
                    ))
                    .id()
            } else {
                commands
                    .spawn((
                        Sphere,
                        AnimationStorage::<isize>::new(z),
                        PaletteLightingMeshBundle {
                            mesh: mesh_cube.clone(),
                            material: material_white.clone(),
                            transform: Transform::from_xyz(x as f32 * 2.0, 0.0, z as f32 * 2.0),
                            ..default()
                        },
                    ))
                    .id()
            };

            spheres.push((iz, z, sphere));
        }

        rows.push((ix, x, row, spheres));
    }

    let mut cubes = vec![];

    for (iz, z) in (z_min * 10..z_max * 10).into_iter().enumerate() {
        let mut hasher = DefaultHasher::new();
        z.hash(&mut hasher);
        let rand = hasher.finish() % 2 == 0;

        let cube = if rand {
            commands
                .spawn((
                    Cube,
                    AnimationStorage::<isize>::new(z),
                    PaletteLightingMeshBundle {
                        mesh: mesh_half_cube.clone(),
                        material: material_green.clone(),
                        transform: Transform::from_xyz(0.0, 1.0, z as f32 * 2.0),
                        ..default()
                    },
                ))
                .id()
        } else {
            commands
                .spawn((
                    Cube,
                    AnimationStorage::<isize>::new(z),
                    PaletteLightingMeshBundle {
                        mesh: mesh_cube.clone(),
                        material: material_white.clone(),
                        transform: Transform::from_xyz(0.0, 1.0, z as f32 * 2.0),
                        ..default()
                    },
                ))
                .id()
        };

        cubes.push((iz, z, cube));
    }

    commands.spawn((
        Torus,
        PaletteLightingMeshBundle {
            mesh: mesh_cube.clone(),
            material: material_fine_gold.clone(),
            transform: Transform::from_xyz(0.0, 3.0, 0.0),
            ..default()
        },
        RigidBody::KinematicPositionBased,
        Collider::ball(0.5),
        LerpTransform::default(),
    ));

    commands.spawn((
        Quad,
        PaletteLightingMeshBundle {
            mesh: mesh_quad.clone(),
            material: material_2d.clone(),
            transform: Transform::from_xyz(0.0, 3.0, 0.0),
            ..default()
        },
    ));

    commands.spawn((
        TransformBundle::from(Transform::from_xyz(0.0, 3.0, -5.0)),
        Collider::cuboid(2.0, 2.0, 1.0),
        Sensor,
    ));

    // Setup animations
    animations.add(
        "time",
        (|query: Query<&TimelineComponent>| {
            query
                .get_single()
                .expect("Missing Timeline entity")
                .timestamp as f32
        })
        .pipe(AnimationStorage::new.lift())
        .pipe(write_animation_storage(time)),
    );

    animations.add(
        "quad",
        |timeline: Query<&TimelineComponent>, mut query: Query<&mut Transform, With<Quad>>| {
            let timeline = timeline.get_single().unwrap();

            for mut transform in query.iter_mut() {
                transform.rotation =
                    Quat::from_euler(EulerRot::XYZ, 0.0, timeline.timestamp as f32, 0.0);
            }
        },
    );

    animations.add(
            "cube",
            read_animation_storage::<f32>(time)
                .pipe(
                    |In(time): In<f32>,
                     mut query: Query<(&Cube, &AnimationStorage<isize>, &mut Transform)>| {
                        query.par_for_each_mut(2500, |(_, index, mut transform)| {
                            let t = time + **index as f32;
                            transform.translation.x = t.sin();
                            transform.translation.y = t.cos();
                        })
                    },
                )
                .after("time")
        );

    animations.add(
        "row",
        read_animation_storage::<f32>(time).pipe(
            |In(input): In<f32>,
             mut query: Query<(&AnimationStorage<isize>, &mut AnimationStorage<f32>)>| {
                query.par_for_each_mut(10, |(idx, mut out)| **out = (input + (**idx as f32)).sin())
            },
        ).after("time"),
    );

    animations.add(
            "sphere",
            read_animation_storage::<f32>(time)
                .pipe(
                    |In(time): In<f32>,
                     mut query: Query<(&Sphere, &AnimationStorage<isize>, &mut Transform)>| {
                        query.par_for_each_mut(2500, |(_, index, mut transform)| {
                            transform.translation.y = (time + **index as f32).cos()
                        })
                    },
                )
                .after("row")
        );
}

fn key_input(events: Res<Input<KeyCode>>, mut animations: ResMut<AnimationSchedule>) {
    for event in events.get_just_pressed() {
        match event {
            KeyCode::J => animations.toggle("time"),
            KeyCode::K => animations.toggle("row"),
            KeyCode::L => animations.toggle("sphere"),
            _ => (),
        }
    }
}

mod material_loader {
    use std::marker::PhantomData;

    use bevy::{
        asset::{Asset, HandleId},
        prelude::{default, info, Assets, Handle, Plugin, Res, ResMut, Resource},
        utils::HashMap,
    };

    use crate::{image_loader::ImageLoader, npbr::palette_lighting::PaletteLightingMaterial};

    #[derive(Debug, Copy, Clone, Default)]
    pub struct MaterialLoaderPlugin<T> {
        _phantom: PhantomData<T>,
    }

    impl<T> Plugin for MaterialLoaderPlugin<T>
    where
        T: Asset,
    {
        fn build(&self, app: &mut bevy::prelude::App) {
            app.init_resource::<MaterialLoader<T>>()
                .add_system(material_loader);
        }
    }

    #[derive(Debug, Clone, Resource)]
    pub struct MaterialLoader<T>
    where
        T: Asset,
    {
        materials: HashMap<Handle<T>, T>,
    }

    impl<T> Default for MaterialLoader<T>
    where
        T: Asset,
    {
        fn default() -> Self {
            MaterialLoader {
                materials: default(),
            }
        }
    }

    impl MaterialLoader<PaletteLightingMaterial> {
        pub fn load(
            &mut self,
            materials: &Assets<PaletteLightingMaterial>,
            material: PaletteLightingMaterial,
        ) -> Handle<PaletteLightingMaterial> {
            let handle_id = HandleId::random::<PaletteLightingMaterial>();
            let mut handle = Handle::weak(handle_id);
            self.materials.insert(handle.clone(), material);
            handle.make_strong(&materials);
            handle
        }
    }

    pub fn material_loader(
        mut material_loader: ResMut<MaterialLoader<PaletteLightingMaterial>>,
        mut materials: ResMut<Assets<PaletteLightingMaterial>>,
        image_loader: Res<ImageLoader>,
    ) {
        for (mut handle, material) in material_loader.materials.drain_filter(|_, material| {
            image_loader.is_loaded(&material.palette_texture)
                && image_loader.is_loaded(&material.dither_texture)
        }) {
            info!("Loaded material with handle {handle:?}\n{material:#?}");
            handle.make_strong(&materials);
            materials.set_untracked(handle, material);
        }
    }
}
