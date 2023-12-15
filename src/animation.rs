use bevy::{
    ecs::{
        schedule::{ShouldRun, SystemLabelId},
        system::{lifetimeless::SQuery, FunctionSystem, InputMarker, PipeSystem},
    },
    prelude::{
        default, info, App, Component, Deref, DerefMut, Entity, In, IntoPipeSystem, IntoSystem,
        IntoSystemDescriptor, Mut, Plugin, Query, Res, ResMut, Resource, Schedule, Stage,
        StageLabel, SystemLabel, SystemStage, World,
    },
    reflect::{GetPath, Reflect},
    utils::HashSet,
};

use crate::lift_system::{IntoLiftSystem, LiftSystem};

pub struct AnimationPlugin<T> {
    pub system_stage: T,
}

impl<T> Plugin for AnimationPlugin<T>
where
    T: Clone + StageLabel + Send + Sync,
{
    fn build(&self, app: &mut App) {
        app.init_resource::<AnimationSchedule>()
            .init_resource::<AnimationMeta>()
            .add_system_to_stage(self.system_stage.clone(), run_animations.at_end());
    }
}

#[derive(StageLabel)]
struct DefaultAnimationStage;

#[derive(Debug, Resource)]
pub struct AnimationSchedule {
    schedule: Schedule,
    meta: Option<AnimationMeta>,
}

impl Default for AnimationSchedule {
    fn default() -> Self {
        let mut schedule: Schedule = default();
        schedule.add_stage(DefaultAnimationStage, SystemStage::parallel());
        AnimationSchedule {
            schedule,
            meta: Some(default()),
        }
    }
}

impl AnimationSchedule {
    pub fn add<L, F, Params>(&mut self, label: L, f: F) -> &mut Self
    where
        L: SystemLabel,
        F: IntoSystemDescriptor<Params>,
    {
        let label = label.as_label();
        self.schedule.add_system_to_stage(
            DefaultAnimationStage,
            f.label(label)
                .with_run_criteria(should_run_animation(label)),
        );

        let mut meta = self
            .meta
            .take()
            .expect("Animation metadata may not be modified during evaluation");

        meta.start(label);

        self.meta = Some(meta);

        self
    }

    pub fn start<L: SystemLabel>(&mut self, label: L) {
        self.meta
            .as_mut()
            .expect("Can't start an animation during evaluation")
            .start(label)
    }

    pub fn stop<L: SystemLabel>(&mut self, label: L) {
        self.meta
            .as_mut()
            .expect("Can't stop an animation during evaluation")
            .stop(label)
    }

    pub fn toggle<L: SystemLabel>(&mut self, label: L) {
        self.meta
            .as_mut()
            .expect("Can't toggle an animation during evaluation")
            .toggle(label)
    }
}

fn curry<T, U, O, F>(mut f: F, b: U) -> impl FnMut(T) -> O
where
    F: FnMut(T, U) -> O,
    U: Copy,
{
    move |a| f(a, b)
}

fn join<T, U, O, F>(f: F) -> impl Fn((T, U)) -> O
where
    F: Fn(T, U) -> O,
{
    move |(a, b)| f(a, b)
}

fn copy_component<T: Copy + Component>(entity: Entity) -> impl FnMut(Query<&T>) -> T {
    move |query: Query<&T>| *query.get(entity).unwrap()
}

fn clone_component<T: Copy + Component>(query: Query<&T>) -> T {
    query.iter().next().unwrap().clone()
}

fn print_value<T: std::fmt::Debug>(input: In<T>) -> T {
    info!("{:#?}", input.0);
    input.0
}

fn lift_mut<I, O, F>(mut f: F) -> impl FnMut(Mut<I>) -> O + Send + Sync + Clone
where
    F: FnMut(&mut I) -> O + Send + Sync + Clone,
{
    move |mut input: Mut<I>| f(&mut input)
}

pub fn lift_res<I, O, F>(mut f: F) -> impl FnMut(Res<I>) -> O + Send + Sync + Clone
where
    F: FnMut(&I) -> O + Send + Sync + Clone,
    I: Resource,
{
    move |input: Res<I>| f(input.into_inner())
}

pub fn lift_res_mut<I, O, F>(mut f: F) -> impl FnMut(ResMut<I>) -> O + Send + Sync + Clone
where
    F: FnMut(&mut I) -> O + Send + Sync + Clone,
    I: Resource,
{
    move |input: ResMut<I>| f(input.into_inner())
}

pub fn read_animation_storage<T>(
    entity: Entity,
) -> PipeSystem<
    FunctionSystem<
        (),
        AnimationStorage<T>,
        (SQuery<&'static AnimationStorage<T>>,),
        (),
        impl FnMut(Query<&AnimationStorage<T>>) -> AnimationStorage<T>,
    >,
    LiftSystem<impl FnMut(AnimationStorage<T>) -> T, AnimationStorage<T>, T>,
>
where
    T: Copy + Reflect,
{
    copy_component::<AnimationStorage<T>>(entity).pipe(AnimationStorage::<T>::into_value.lift())
}

pub fn write_animation_storage<T>(
    entity: Entity,
) -> FunctionSystem<
    AnimationStorage<T>,
    (),
    (SQuery<&'static mut AnimationStorage<T>>,),
    InputMarker,
    impl FnMut(In<AnimationStorage<T>>, Query<&mut AnimationStorage<T>>),
>
where
    T: 'static + Send + Sync + Clone,
{
    IntoSystem::into_system(apply_component::<1, AnimationStorage<T>>([entity]))
}

pub fn apply_resource<T>() -> impl FnMut(In<T>, ResMut<T>)
where
    T: Clone + Resource,
{
    move |input: In<T>, mut res: ResMut<T>| {
        *res = input.0.clone();
    }
}

pub fn apply_resource_path<'p, T>(path: &'p str) -> impl FnMut(In<T>, ResMut<T>) + 'p
where
    T: Clone + Resource + Reflect,
{
    move |input: In<T>, mut res: ResMut<T>| {
        *res.get_path_mut(path).unwrap() = input.0.clone();
    }
}

pub fn apply_component<const N: usize, T>(entities: [Entity; N]) -> impl FnMut(In<T>, Query<&mut T>)
where
    T: Clone + Component,
{
    move |input: In<T>, mut query: Query<&mut T>| {
        for entity in entities {
            *query.get_mut(entity).unwrap() = input.0.clone();
        }
    }
}

pub fn apply_component_path<'p, T, U, const N: usize>(
    path: &'p str,
    entities: [Entity; N],
) -> impl FnMut(In<U>, Query<&mut T>) + 'p
where
    T: Component + Reflect,
    U: Clone + Reflect,
{
    move |input: In<U>, mut query: Query<&mut T>| {
        for mut component in query.get_many_mut(entities).unwrap() {
            *component.get_path_mut::<U>(path).unwrap() = input.0.clone();
        }
    }
}

pub fn tail<T>(_: In<T>) {}

#[derive(Debug, Default, Copy, Clone, Deref, DerefMut, Component)]
pub struct AnimationStorage<T> {
    pub value: T,
}

impl<T> AnimationStorage<T>
where
    T: Reflect,
{
    pub fn new(value: T) -> Self {
        AnimationStorage { value }
    }

    pub fn into_value(self) -> T {
        self.value
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn value_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

#[derive(Debug, Default, Clone, Resource)]
struct AnimationMeta {
    active: HashSet<SystemLabelId>,
    inactive: HashSet<SystemLabelId>,
}

impl AnimationMeta {
    fn start<T: SystemLabel>(&mut self, label: T) {
        let label = label.as_label();
        self.remove(label);
        self.active.insert(label);
    }

    fn stop<T: SystemLabel>(&mut self, label: T) {
        let label = label.as_label();
        self.remove(label);
        self.inactive.insert(label);
    }

    fn remove<T: SystemLabel>(&mut self, label: T) -> bool {
        let label = label.as_label();
        self.active.remove(&label) || self.inactive.remove(&label)
    }

    fn contains<T: SystemLabel>(&self, label: T) -> bool {
        let label = label.as_label();
        self.active.contains(&label) || self.inactive.contains(&label)
    }

    fn is_active<T: SystemLabel>(&self, label: T) -> bool {
        self.active.contains(&label.as_label())
    }

    fn toggle<T: SystemLabel>(&mut self, label: T) {
        let label = label.as_label();

        if self.is_active(label) {
            self.stop(label)
        } else {
            self.start(label)
        }
    }
}

fn should_run_animation<T: SystemLabel>(label: T) -> impl FnMut(Res<AnimationMeta>) -> ShouldRun {
    move |animations: Res<AnimationMeta>| animations.active.contains(&label.as_label()).into()
}

pub fn run_animations(world: &mut World) {
    let mut animations: AnimationSchedule = world.remove_resource().unwrap();
    let meta = animations
        .meta
        .take()
        .expect("Run animations may not be invoked recursively.");
    world.insert_resource(meta);
    animations.schedule.run(world);
    let meta = world
        .remove_resource::<AnimationMeta>()
        .expect("AnimationMeta removed during schedule evaluation");
    animations.meta = Some(meta);
    world.insert_resource(animations);
}
