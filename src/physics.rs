use std::{
    any::TypeId,
    collections::VecDeque,
    marker::PhantomData,
    time::{Duration},
};

use bevy::{
    app::{AppExit, ScheduleRunnerPlugin},
    ecs::{query::WorldQuery, schedule::ShouldRun},
    pbr::MeshUniform,
    prelude::{
        default, AddAsset, App, AppTypeRegistry, AssetPlugin, Component, CoreStage, Deref,
        DerefMut, IntoSystemDescriptor, Mesh, Plugin, ResMut, StageLabel, StartupSchedule,
        StartupStage, SystemStage, With,
    },
    render::{
        extract_component::{
            ExtractComponent as ExtractRenderComponent,
            ExtractComponentPlugin as ExtractRenderComponentPlugin,
        },
        RenderApp, RenderStage,
    },
    scene::Scene,
    time::{Time, TimePlugin, TimeUpdateStrategy},
};
use bevy_rapier3d::prelude::{
    Collider, PhysicsStages, RapierContext, RapierPhysicsPlugin, RigidBody,
};

use std::ops::{Deref, DerefMut};

use bevy::{
    prelude::{
        debug, Commands, Entity, Events, GlobalTransform, Query, Resource, Schedule, Stage,
        Transform, World,
    },
    tasks::{AsyncComputeTaskPool, Task},
};
use bevy_rapier3d::{
    pipeline::ContactForceEvent,
    prelude::{
        systems::{ColliderComponents, RigidBodyComponents, RigidBodyWritebackComponents},
        CollisionEvent, PhysicsHooksWithQueryResource, RapierColliderHandle, RapierConfiguration,
        RapierRigidBodyHandle, SimulationToRenderTime,
    },
};

use crate::timeline::TimelineComponent;

use self::{
    extract_component::ExtractComponentPlugin, extract_param::Extract,
    writeback_component::WritebackComponentPlugin,
};

#[derive(Debug, Default, Copy, Clone)]
pub struct PhysicsPlugin<T = ()> {
    _phantom: PhantomData<T>,
}

pub struct PhysicsAppBuilder<T> {
    app: App,
    phantom: PhantomData<T>,
}

impl<T> Default for PhysicsAppBuilder<T>
where
    T: 'static + Send + Sync + WorldQuery,
{
    fn default() -> Self {
        // Create the physics app
        let mut app = App::empty();

        // Setup the extraction for reading data from the main world to the physics world
        let mut extract_stage = SystemStage::single_threaded()
            .with_system(extract_collider)
            .with_system(extract_rigid_body);

        app.init_resource::<MainWorld>();
        app.world.remove_resource::<MainWorld>();
        let main_world_in_render = app
            .world
            .components()
            .get_resource_id(TypeId::of::<MainWorld>());

        // `Extract` systems must read from the main world. We want to emit an error when that doesn't occur
        // Safe to unwrap: Ensured it existed just above
        extract_stage.set_must_read_resource(main_world_in_render.unwrap());

        // don't apply buffers when the stage finishes running
        // extract stage runs on the render world, but buffers are applied
        // after access to the main world is removed
        // See also https://github.com/bevyengine/bevy/issues/5082
        extract_stage.set_apply_buffers(false);

        // Setup the writeback stage for reading data from the physics world to the main world
        let mut writeback_stage = SystemStage::single_threaded().with_system(writeback_rigid_body);

        // don't apply buffers when the stage finishes running,
        // as they're instead applied to the main world
        writeback_stage.set_apply_buffers(false);

        app.schedule
            .add_stage(
                StartupSchedule,
                Schedule::default()
                    .with_run_criteria(ShouldRun::once)
                    .with_stage(StartupStage::Startup, SystemStage::single_threaded()),
            )
            .add_stage(CoreStage::First, SystemStage::single_threaded())
            .add_stage(CoreStage::PreUpdate, SystemStage::single_threaded())
            .add_stage(PhysicsStage::Extract, extract_stage)
            .add_stage(PhysicsStage::PrePhysics, SystemStage::single_threaded())
            .add_stage(
                PhysicsStage::RapierSyncBackend,
                SystemStage::single_threaded().with_system_set(
                    RapierPhysicsPlugin::<T>::get_systems(PhysicsStages::SyncBackend),
                ),
            )
            .add_stage(
                PhysicsStage::RapierStepSimulation,
                SystemStage::single_threaded().with_system_set(
                    RapierPhysicsPlugin::<T>::get_systems(PhysicsStages::StepSimulation),
                ),
            )
            .add_stage(
                PhysicsStage::RapierWriteback,
                SystemStage::single_threaded().with_system_set(
                    RapierPhysicsPlugin::<T>::get_systems(PhysicsStages::Writeback),
                ),
            )
            .add_stage(
                PhysicsStage::RapierDetectDespawn,
                SystemStage::single_threaded().with_system_set(
                    RapierPhysicsPlugin::<T>::get_systems(PhysicsStages::DetectDespawn),
                ),
            )
            .add_stage(
                PhysicsStage::PostPhysics,
                SystemStage::single_threaded().with_system(update_lerp_transform),
            )
            .add_stage(PhysicsStage::Writeback, writeback_stage)
            .add_stage(CoreStage::PostUpdate, SystemStage::single_threaded())
            .add_stage(
                CoreStage::Last,
                SystemStage::single_threaded().with_system(World::clear_trackers),
            );

        app.add_event::<AppExit>();
        app.init_resource::<AppTypeRegistry>();

        // Configure it with the minimum setup needed to run rapier
        app.add_plugin(TimePlugin::default());
        app.insert_resource(TimeUpdateStrategy::ManualInstant(
            app.world.resource::<Time>().startup(),
        ));

        app.add_plugin(ScheduleRunnerPlugin::default());

        app.add_plugin(AssetPlugin::default())
            .add_asset::<Mesh>()
            .add_asset::<Scene>();

        PhysicsAppBuilder {
            app,
            phantom: default(),
        }
    }
}

impl<T> PhysicsAppBuilder<T> {
    pub fn map<F: FnOnce(&mut App)>(mut self, f: F) -> Self {
        f(&mut self.app);
        self
    }

    pub fn build(self) -> PhysicsApp {
        self.app.into()
    }
}

impl<T> Plugin for PhysicsPlugin<T>
where
    T: 'static + Send + Sync + WorldQuery,
{
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugin(RapierPhysicsPlugin::<T>::default().with_default_system_setup(false));

        app.init_resource::<ScratchMainWorld>();

        app.add_system(fork_physics::<T>.at_end())
            .add_system(join_physics::<T>.at_end().after(fork_physics::<T>));

        app.add_plugin(ExtractRenderComponentPlugin::<LerpTransform>::default());
        app.add_plugin(ExtractRenderComponentPlugin::<TimelineComponent>::default());
        app.sub_app_mut(RenderApp)
            .add_system_to_stage(RenderStage::Prepare, interpolate_physics.at_start());

        app.add_system(dispatch_physics);

        // Gubbins
        app.add_plugin(ExtractComponentPlugin::<Transform, With<Collider>>::default());
        app.add_plugin(ExtractComponentPlugin::<GlobalTransform, With<Collider>>::default());
        app.add_plugin(ExtractComponentPlugin::<RapierColliderHandle, With<Collider>>::default());
        app.add_plugin(ExtractComponentPlugin::<
            RapierRigidBodyHandle,
            With<RigidBody>,
        >::default());
        app.add_plugin(ExtractComponentPlugin::<LerpTransform, With<RigidBody>>::default());

        app.add_plugin(WritebackComponentPlugin::<
            RapierColliderHandle,
            With<Collider>,
        >::default());

        app.add_plugin(WritebackComponentPlugin::<
            RapierRigidBodyHandle,
            With<RigidBody>,
        >::default());

        app.add_plugin(WritebackComponentPlugin::<LerpTransform, With<RigidBody>>::default());

        app.add_startup_system(|world: &mut World| {
            let mut physics_app = world.remove_resource::<PhysicsApp>().unwrap();

            let schedule = physics_app
                .schedule
                .get_stage_mut::<Schedule>(StartupSchedule)
                .unwrap();

            let stage = schedule
                .get_stage_mut::<SystemStage>(StartupStage::Startup)
                .unwrap();

            stage.run(&mut physics_app.world);

            world.insert_resource(physics_app);
        });
    }
}

#[derive(Debug, Default, Clone, Component)]
pub struct LerpTransform {
    pub timestamps: VecDeque<(f64, Transform)>,
}

impl ExtractRenderComponent for LerpTransform {
    type Query = &'static Self;

    type Filter = ();

    fn extract_component(item: bevy::ecs::query::QueryItem<'_, Self::Query>) -> Self {
        item.clone()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, StageLabel)]
pub enum PhysicsStage {
    Extract,
    PrePhysics,
    RapierSyncBackend,
    RapierStepSimulation,
    RapierWriteback,
    RapierDetectDespawn,
    PostPhysics,
    Writeback,
}

#[derive(Debug, Default, Resource)]
pub struct PhysicsApp {
    pub world: World,
    pub schedule: Schedule,
    pub target_tick: usize,
    pub delta: f64,

    current_tick: Option<usize>,
}

impl From<App> for PhysicsApp {
    fn from(value: App) -> Self {
        PhysicsApp {
            world: value.world,
            schedule: value.schedule,
            current_tick: None,
            target_tick: 0,
            delta: 1.0,
        }
    }
}

impl PhysicsApp {
    pub fn current_tick(&self) -> isize {
        self.current_tick
            .map(|current_tick| current_tick as isize)
            .unwrap_or(-1)
    }
}

#[derive(Debug, Deref, DerefMut, Resource)]
pub struct PhysicsTask(Task<PhysicsApp>);

/// The simulation [`World`] of the application, stored as a resource.
/// This resource is only available during [`PhysicsStage::Extract`] and not
/// during command application of that stage.
/// See [`Extract`] for more details.
#[derive(Resource, Default)]
pub struct MainWorld(World);

impl Deref for MainWorld {
    type Target = World;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MainWorld {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// A "scratch" world used to avoid allocating new worlds every frame when
/// swapping out the [`MainWorld`] for [`PhysicsStage::Extract`].
#[derive(Resource, Default)]
pub struct ScratchMainWorld(World);

pub fn fork_physics<T: WorldQuery + 'static>(main_world: &mut World) {
    let Some(mut physics_app) = main_world.remove_resource::<PhysicsApp>() else {
        return
    };

    let current_tick = physics_app.current_tick();
    let delta = physics_app.target_tick as isize - current_tick;
    if delta == 0 {
        main_world.insert_resource(physics_app);
        return;
    }

    debug!("Physics world ready, dispatching async");

    // reserve all existing app entities for use in render_app
    // they can only be spawned using `get_or_spawn()`
    let meta_len = main_world.entities().meta_len();

    assert_eq!(
        physics_app.world.entities().len(),
        0,
        "An entity was spawned after the entity list was cleared last frame and before the extract stage began. This is not supported",
    );

    // This is safe given the clear_entities call in the past frame and the assert above
    unsafe {
        physics_app
            .world
            .entities_mut()
            .flush_and_reserve_invalid_assuming_no_entities(meta_len);
    }

    // Copy components from main world to physics world
    extract::<T>(main_world, &mut physics_app);

    // Dispatch async task
    let task_pool = AsyncComputeTaskPool::get();
    let task = task_pool.spawn(async move {
        for _ in 0..delta.abs() {
            let startup = physics_app.world.resource::<Time>().startup();
            let instant = startup + Duration::from_secs_f64(physics_app.delta * physics_app.current_tick().abs() as f64);

            let TimeUpdateStrategy::ManualInstant(time_update) = &mut *physics_app.world.resource_mut::<TimeUpdateStrategy>() else {panic!()};
            *time_update = instant;

            let first = physics_app
                .schedule
                .get_stage_mut::<SystemStage>(CoreStage::First)
                .unwrap();
            first.run(&mut physics_app.world);

            let pre_physics = physics_app
                .schedule
                .get_stage_mut::<SystemStage>(PhysicsStage::PrePhysics)
                .unwrap();
            pre_physics.run(&mut physics_app.world);

            let rapier_sync_backend = physics_app
                .schedule
                .get_stage_mut::<SystemStage>(PhysicsStage::RapierSyncBackend)
                .unwrap();
            rapier_sync_backend.run(&mut physics_app.world);

            let rapier_step_simulation = physics_app
                .schedule
                .get_stage_mut::<SystemStage>(PhysicsStage::RapierStepSimulation)
                .unwrap();
            rapier_step_simulation.run(&mut physics_app.world);

            let rapier_writeback = physics_app
                .schedule
                .get_stage_mut::<SystemStage>(PhysicsStage::RapierWriteback)
                .unwrap();
            rapier_writeback.run(&mut physics_app.world);

            let rapier_detect_despawn = physics_app
                .schedule
                .get_stage_mut::<SystemStage>(PhysicsStage::RapierDetectDespawn)
                .unwrap();
            rapier_detect_despawn.run(&mut physics_app.world);

            let post_physics = physics_app
                .schedule
                .get_stage_mut::<SystemStage>(PhysicsStage::PostPhysics)
                .unwrap();
            post_physics.run(&mut physics_app.world);

            physics_app.current_tick = Some((physics_app.current_tick() + delta.signum()) as usize);
        }

        physics_app
    });

    main_world.insert_resource(PhysicsTask(task));
}

pub fn join_physics<T: WorldQuery + 'static>(main_world: &mut World) {
    let Some(mut task) = main_world.remove_resource::<PhysicsTask>() else {
        return
    };

    debug!("Physics world not ready, polling future");
    if let Some(mut async_app) =
        futures_lite::future::block_on(futures_lite::future::poll_once(&mut *task))
    {
        debug!("Task finished, replacing world");

        // Copy components from physics world to main world
        writeback::<T>(main_world, &mut async_app);

        // Clear entities from physics world
        async_app.world.clear_entities();

        main_world.insert_resource(async_app);
    } else {
        debug!("Task not finished, replacing");
        main_world.insert_resource(task)
    }
}

fn move_resource<T: Resource>(from: &mut World, to: &mut World) {
    // Move resources
    to.insert_resource(from.remove_resource::<T>().unwrap());
}

fn extract<T: WorldQuery + 'static>(main_world: &mut World, physics_app: &mut PhysicsApp) {
    // Move resources
    move_resource::<RapierContext>(main_world, &mut physics_app.world);
    main_world.insert_resource(RapierContext::default());

    move_resource::<RapierConfiguration>(main_world, &mut physics_app.world);
    move_resource::<SimulationToRenderTime>(main_world, &mut physics_app.world);
    move_resource::<Events<CollisionEvent>>(main_world, &mut physics_app.world);
    move_resource::<Events<ContactForceEvent>>(main_world, &mut physics_app.world);
    move_resource::<PhysicsHooksWithQueryResource<T>>(main_world, &mut physics_app.world);

    // Run extract stage
    let extract = physics_app
        .schedule
        .get_stage_mut::<SystemStage>(PhysicsStage::Extract)
        .unwrap();

    // temporarily add the app world to the render world as a resource
    let scratch_world = main_world.remove_resource::<ScratchMainWorld>().unwrap();
    let inserted_world = std::mem::replace(main_world, scratch_world.0);
    let running_world = &mut physics_app.world;
    running_world.insert_resource(MainWorld(inserted_world));

    extract.run(running_world);
    // move the app world back, as if nothing happened.
    let inserted_world = running_world.remove_resource::<MainWorld>().unwrap();
    let scratch_world = std::mem::replace(main_world, inserted_world.0);
    main_world.insert_resource(ScratchMainWorld(scratch_world));

    // Note: We apply buffers (read, Commands) after the `MainWorld` has been removed from the render app's world
    // so that in future, pipelining will be able to do this too without any code relying on it.
    // see <https://github.com/bevyengine/bevy/issues/5082>
    extract.apply_buffers(running_world);
}

fn writeback<T: WorldQuery + 'static>(main_world: &mut World, async_app: &mut PhysicsApp) {
    let writeback = async_app
        .schedule
        .get_stage_mut::<SystemStage>(PhysicsStage::Writeback)
        .unwrap();

    writeback.run(&mut async_app.world);

    writeback.apply_buffers(main_world);

    // Move resources
    move_resource::<RapierContext>(&mut async_app.world, main_world);
    move_resource::<RapierConfiguration>(&mut async_app.world, main_world);
    move_resource::<SimulationToRenderTime>(&mut async_app.world, main_world);
    move_resource::<Events<CollisionEvent>>(&mut async_app.world, main_world);
    move_resource::<Events<ContactForceEvent>>(&mut async_app.world, main_world);
    move_resource::<PhysicsHooksWithQueryResource<T>>(&mut async_app.world, main_world);
}

pub fn extract_rigid_body(
    mut commands: Commands,
    query_components: Extract<Query<RigidBodyComponents>>,
) {
    for (
        entity,
        rigid_body,
        global_transform,
        velocity,
        additional_mass_properties,
        read_mass_properties,
        locked_axes,
        external_force,
        gravity_scale,
        ccd,
        dominance,
        sleeping,
        damping,
        rigid_body_disabled,
    ) in query_components.iter()
    {
        debug!("Extracting RigidBody {entity:#?}");

        let mut commands = commands.get_or_spawn(entity);
        commands.insert(*rigid_body);

        if let Some(global_transform) = global_transform {
            commands.insert(*global_transform);
        }

        if let Some(velocity) = velocity {
            commands.insert(*velocity);
        }

        if let Some(additional_mass_properties) = additional_mass_properties {
            commands.insert(*additional_mass_properties);
        }

        if let Some(read_mass_properties) = read_mass_properties {
            commands.insert(*read_mass_properties);
        }

        if let Some(locked_axes) = locked_axes {
            commands.insert(*locked_axes);
        }

        if let Some(external_force) = external_force {
            commands.insert(*external_force);
        }

        if let Some(gravity_scale) = gravity_scale {
            commands.insert(*gravity_scale);
        }

        if let Some(ccd) = ccd {
            commands.insert(*ccd);
        }

        if let Some(dominance) = dominance {
            commands.insert(*dominance);
        }

        if let Some(sleeping) = sleeping {
            commands.insert(*sleeping);
        }

        if let Some(damping) = damping {
            commands.insert(*damping);
        }

        if let Some(rigid_body_disabled) = rigid_body_disabled {
            commands.insert(*rigid_body_disabled);
        }
    }
}

pub fn extract_collider(
    mut commands: Commands,
    query_components: Extract<Query<ColliderComponents>>,
    query_transform: Extract<Query<(Option<&Transform>, Option<&GlobalTransform>)>>,
) {
    for (
        entity,
        collider,
        sensor,
        collider_mass_properties,
        active_events,
        active_hooks,
        active_collision_types,
        friction,
        restitution,
        collision_groups,
        solver_groups,
        contact_force_event_threshold,
        collider_disabled,
    ) in query_components.iter()
    {
        debug!("Extracting Collider {entity:#?}");
        let mut commands = commands.get_or_spawn(entity);
        commands.insert(collider.clone());

        if let Ok((transform, global_transform)) = query_transform.get(entity) {
            if let Some(transform) = transform {
                commands.insert(*transform);
            }

            if let Some(global_transform) = global_transform {
                commands.insert(*global_transform);
            }
        }

        if let Some(sensor) = sensor {
            commands.insert(*sensor);
        }

        if let Some(collider_mass_properties) = collider_mass_properties {
            commands.insert(*collider_mass_properties);
        }

        if let Some(active_events) = active_events {
            commands.insert(*active_events);
        }

        if let Some(active_hooks) = active_hooks {
            commands.insert(*active_hooks);
        }

        if let Some(active_collision_types) = active_collision_types {
            commands.insert(*active_collision_types);
        }

        if let Some(friction) = friction {
            commands.insert(*friction);
        }

        if let Some(restitution) = restitution {
            commands.insert(*restitution);
        }

        if let Some(collision_groups) = collision_groups {
            commands.insert(*collision_groups);
        }

        if let Some(solver_groups) = solver_groups {
            commands.insert(*solver_groups);
        }

        if let Some(contact_force_event_threshold) = contact_force_event_threshold {
            commands.insert(*contact_force_event_threshold);
        }

        if let Some(collider_disabled) = collider_disabled {
            commands.insert(*collider_disabled);
        }
    }
}

pub fn extract_timeline(
    mut commands: Commands,
    query: Extract<Query<(Entity, &TimelineComponent)>>,
) {
    for (entity, timeline) in query.iter() {
        let mut commands = commands.get_or_spawn(entity);
        commands.insert(*timeline);
    }
}

pub fn writeback_rigid_body(
    mut commands: Commands,
    query_components: Query<RigidBodyWritebackComponents>,
) {
    for (entity, _parent, transform, transform_interpolation, velocity, sleeping) in
        query_components.iter()
    {
        debug!("Writing back RigidBody {entity:#?}");

        let mut commands = commands.get_or_spawn(entity);

        // NOTE: Parent is private, non-copy and non-clone, so we can't write it back

        if let Some(transform) = transform {
            commands.insert(*transform);
        }

        if let Some(transform_interpolation) = transform_interpolation {
            commands.insert(*transform_interpolation);
        }

        if let Some(velocity) = velocity {
            commands.insert(*velocity);
        }

        if let Some(sleeping) = sleeping {
            commands.insert(*sleeping);
        }
    }
}

pub fn update_lerp_transform(
    query_timeline: Query<&TimelineComponent>,
    mut query_lerp_transform: Query<(&Transform, &mut LerpTransform)>,
) {
    let timeline = query_timeline.get_single().unwrap();

    for (transform, mut lerp_transform) in query_lerp_transform.iter_mut() {
        lerp_transform
            .timestamps
            .push_front((timeline.timestamp, *transform));

        if lerp_transform.timestamps.len() > 2 {
            lerp_transform.timestamps.drain(2..);
        }
    }
}

pub mod extract_component {
    use std::marker::PhantomData;

    use bevy::{
        ecs::query::ReadOnlyWorldQuery,
        prelude::{default, Commands, Component, Entity, Plugin, Query, ResMut, SystemStage},
    };

    use super::{extract_param::Extract, PhysicsApp, PhysicsStage};

    #[derive(Debug)]
    pub struct ExtractComponentPlugin<T, F = ()> {
        phantom: PhantomData<(T, F)>,
    }

    impl<T, F> Default for ExtractComponentPlugin<T, F> {
        fn default() -> Self {
            Self { phantom: default() }
        }
    }

    impl<T, F> Plugin for ExtractComponentPlugin<T, F>
    where
        T: Clone + Component,
        F: 'static + Send + Sync + ReadOnlyWorldQuery,
    {
        fn build(&self, app: &mut bevy::prelude::App) {
            app.add_startup_system(|mut physics_app: ResMut<PhysicsApp>| {
                physics_app
                    .schedule
                    .get_stage_mut::<SystemStage>(PhysicsStage::Extract)
                    .unwrap()
                    .add_system(extract_component::<T, F>);
            });
        }
    }

    fn extract_component<T, F: ReadOnlyWorldQuery>(
        mut commands: Commands,
        query: Extract<Query<(Entity, &T), F>>,
    ) where
        T: Clone + Component,
    {
        for (entity, component) in query.iter() {
            let mut commands = commands.get_or_spawn(entity);
            commands.insert(component.clone());
        }
    }
}

pub mod writeback_component {
    use std::marker::PhantomData;

    use bevy::{
        ecs::query::ReadOnlyWorldQuery,
        prelude::{default, Commands, Component, Entity, Plugin, Query, ResMut, SystemStage},
    };

    use super::{PhysicsApp, PhysicsStage};

    #[derive(Debug)]
    pub struct WritebackComponentPlugin<T, F = ()> {
        phantom: PhantomData<(T, F)>,
    }

    impl<T, F> Default for WritebackComponentPlugin<T, F> {
        fn default() -> Self {
            Self { phantom: default() }
        }
    }

    impl<T, F> Plugin for WritebackComponentPlugin<T, F>
    where
        T: Clone + Component,
        F: 'static + Send + Sync + ReadOnlyWorldQuery,
    {
        fn build(&self, app: &mut bevy::prelude::App) {
            app.add_startup_system(|mut physics_app: ResMut<PhysicsApp>| {
                physics_app
                    .schedule
                    .get_stage_mut::<SystemStage>(PhysicsStage::Writeback)
                    .unwrap()
                    .add_system(writeback_component::<T, F>);
            });
        }
    }

    fn writeback_component<T, F>(mut commands: Commands, query: Query<(Entity, &T), F>)
    where
        T: Clone + Component,
        F: ReadOnlyWorldQuery,
    {
        for (entity, component) in query.iter() {
            let mut commands = commands.get_or_spawn(entity);
            commands.insert(component.clone());
        }
    }
}

pub mod extract_param {
    use super::MainWorld;
    use bevy::ecs::{
        prelude::*,
        system::{
            ReadOnlySystemParamFetch, ResState, SystemMeta, SystemParam, SystemParamFetch,
            SystemParamItem, SystemParamState, SystemState,
        },
    };
    use std::ops::{Deref, DerefMut};

    /// A helper for accessing [`MainWorld`] content using a system parameter.
    ///
    /// A [`SystemParam`] adapter which applies the contained `SystemParam` to the [`World`]
    /// contained in [`MainWorld`]. This parameter only works for systems run
    /// during [`RenderStage::Extract`].
    ///
    /// This requires that the contained [`SystemParam`] does not mutate the world, as it
    /// uses a read-only reference to [`MainWorld`] internally.
    ///
    /// ## Context
    ///
    /// [`RenderStage::Extract`] is used to extract (move) data from the simulation world ([`MainWorld`]) to the
    /// render world. The render world drives rendering each frame (generally to a [Window]).
    /// This design is used to allow performing calculations related to rendering a prior frame at the same
    /// time as the next frame is simulated, which increases throughput (FPS).
    ///
    /// [`Extract`] is used to get data from the main world during [`RenderStage::Extract`].
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use bevy_ecs::prelude::*;
    /// use bevy_render::Extract;
    /// # #[derive(Component)]
    /// # struct Cloud;
    /// fn extract_clouds(mut commands: Commands, clouds: Extract<Query<Entity, With<Cloud>>>) {
    ///     for cloud in &clouds {
    ///         commands.get_or_spawn(cloud).insert(Cloud);
    ///     }
    /// }
    /// ```
    ///
    /// [`RenderStage::Extract`]: crate::RenderStage::Extract
    /// [Window]: bevy_window::Window
    pub struct Extract<'w, 's, P: SystemParam + 'static>
    where
        P::Fetch: ReadOnlySystemParamFetch,
    {
        item: <P::Fetch as SystemParamFetch<'w, 's>>::Item,
    }

    impl<'w, 's, P: SystemParam> SystemParam for Extract<'w, 's, P>
    where
        P::Fetch: ReadOnlySystemParamFetch,
    {
        type Fetch = ExtractState<P>;
    }

    #[doc(hidden)]
    pub struct ExtractState<P: SystemParam + 'static> {
        state: SystemState<P>,
        main_world_state: ResState<MainWorld>,
    }

    // SAFETY: only accesses MainWorld resource with read only system params using ResState,
    // which is initialized in init()
    unsafe impl<P: SystemParam + 'static> SystemParamState for ExtractState<P> {
        fn init(world: &mut World, system_meta: &mut SystemMeta) -> Self {
            let mut main_world = world.resource_mut::<MainWorld>();
            Self {
                state: SystemState::new(&mut main_world),
                main_world_state: ResState::init(world, system_meta),
            }
        }
    }

    impl<'w, 's, P: SystemParam + 'static> SystemParamFetch<'w, 's> for ExtractState<P>
    where
        P::Fetch: ReadOnlySystemParamFetch,
    {
        type Item = Extract<'w, 's, P>;

        unsafe fn get_param(
            state: &'s mut Self,
            system_meta: &SystemMeta,
            world: &'w World,
            change_tick: u32,
        ) -> Self::Item {
            let main_world = ResState::<MainWorld>::get_param(
                &mut state.main_world_state,
                system_meta,
                world,
                change_tick,
            );
            let item = state.state.get(main_world.into_inner());
            Extract { item }
        }
    }

    impl<'w, 's, P: SystemParam> Deref for Extract<'w, 's, P>
    where
        P::Fetch: ReadOnlySystemParamFetch,
    {
        type Target = <P::Fetch as SystemParamFetch<'w, 's>>::Item;

        #[inline]
        fn deref(&self) -> &Self::Target {
            &self.item
        }
    }

    impl<'w, 's, P: SystemParam> DerefMut for Extract<'w, 's, P>
    where
        P::Fetch: ReadOnlySystemParamFetch,
    {
        #[inline]
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.item
        }
    }

    impl<'a, 'w, 's, P: SystemParam> IntoIterator for &'a Extract<'w, 's, P>
    where
        P::Fetch: ReadOnlySystemParamFetch,
        &'a SystemParamItem<'w, 's, P>: IntoIterator,
    {
        type Item = <&'a SystemParamItem<'w, 's, P> as IntoIterator>::Item;
        type IntoIter = <&'a SystemParamItem<'w, 's, P> as IntoIterator>::IntoIter;

        fn into_iter(self) -> Self::IntoIter {
            (&self.item).into_iter()
        }
    }
}

fn interpolate_transform(from: &Transform, to: &Transform, t: f32) -> Transform {
    let translation = from.translation.lerp(to.translation, t);
    let rotation = from.rotation.slerp(to.rotation, t);
    let scale = from.scale.lerp(to.scale, t);

    Transform {
        translation,
        rotation,
        scale,
    }
}

fn interpolate_physics(
    query_timeline: Query<&TimelineComponent>,
    mut query_lerp_transform: Query<(&mut MeshUniform, &LerpTransform)>,
) {
    let timeline = query_timeline.get_single().unwrap();

    for (mut mesh_uniform, lerp_transform) in query_lerp_transform.iter_mut() {
        let Some((timestamp, transform)) = lerp_transform.timestamps.get(0).copied() else {
            continue
        };

        let Some((prev_timestamp, prev_transform)) = lerp_transform.timestamps.get(1).copied() else {
            continue
        };

        let min = timestamp.min(prev_timestamp);
        let max = timestamp.max(prev_timestamp);

        if min == max {
            continue;
        }

        let duration = (max - min).abs();
        let local_t = (((timeline.timestamp - min) / duration) - 1.0).abs();

        let trx = interpolate_transform(&prev_transform, &transform, local_t as f32);

        mesh_uniform.transform = trx.compute_matrix();
        mesh_uniform.inverse_transpose_model = mesh_uniform.transform.inverse().transpose();
    }
}

fn dispatch_physics(physics_app: Option<ResMut<PhysicsApp>>, query: Query<&TimelineComponent>) {
    let Some(mut physics_app) = physics_app else {
        return;
    };

    let timeline = query.get_single().unwrap().0;
    physics_app.target_tick = (timeline.timestamp * (1.0 / physics_app.delta)).floor() as usize;
}
