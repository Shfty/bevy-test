use std::borrow::Cow;

use bevy::{
    ecs::{archetype::ArchetypeComponentId, component::ComponentId, query::Access},
    prelude::{IntoSystem, System, World},
};

/// A [`System`] created by combining the first and second systems' output into a tuple
pub struct ForkSystem<SystemA, SystemB> {
    system_a: SystemA,
    system_b: SystemB,
    name: Cow<'static, str>,
    component_access: Access<ComponentId>,
    archetype_component_access: Access<ArchetypeComponentId>,
}

impl<SystemA: System, SystemB: System<In = ()>> System for ForkSystem<SystemA, SystemB>
where
    SystemA::In: Clone,
{
    type In = SystemA::In;
    type Out = (SystemA::Out, SystemB::Out);

    fn name(&self) -> Cow<'static, str> {
        self.name.clone()
    }

    fn archetype_component_access(&self) -> &Access<ArchetypeComponentId> {
        &self.archetype_component_access
    }

    fn component_access(&self) -> &Access<ComponentId> {
        &self.component_access
    }

    fn is_send(&self) -> bool {
        self.system_a.is_send() && self.system_b.is_send()
    }

    fn is_exclusive(&self) -> bool {
        self.system_a.is_exclusive() || self.system_b.is_exclusive()
    }

    unsafe fn run_unsafe(&mut self, input: Self::In, world: &World) -> Self::Out {
        let lhs = self.system_a.run_unsafe(input.clone(), world);
        let rhs = self.system_b.run_unsafe((), world);
        (lhs, rhs)
    }

    // needed to make exclusive systems work
    fn run(&mut self, input: Self::In, world: &mut World) -> Self::Out {
        let lhs = self.system_a.run(input.clone(), world);
        let rhs = self.system_b.run((), world);
        (lhs, rhs)
    }

    fn apply_buffers(&mut self, world: &mut World) {
        self.system_a.apply_buffers(world);
        self.system_b.apply_buffers(world);
    }

    fn initialize(&mut self, world: &mut World) {
        self.system_a.initialize(world);
        self.system_b.initialize(world);
        self.component_access
            .extend(self.system_a.component_access());
        self.component_access
            .extend(self.system_b.component_access());
    }

    fn update_archetype_component_access(&mut self, world: &World) {
        self.system_a.update_archetype_component_access(world);
        self.system_b.update_archetype_component_access(world);

        self.archetype_component_access
            .extend(self.system_a.archetype_component_access());
        self.archetype_component_access
            .extend(self.system_b.archetype_component_access());
    }

    fn check_change_tick(&mut self, change_tick: u32) {
        self.system_a.check_change_tick(change_tick);
        self.system_b.check_change_tick(change_tick);
    }

    fn get_last_change_tick(&self) -> u32 {
        self.system_a.get_last_change_tick()
    }

    fn set_last_change_tick(&mut self, last_change_tick: u32) {
        self.system_a.set_last_change_tick(last_change_tick);
        self.system_b.set_last_change_tick(last_change_tick);
    }
}

/// An extension trait providing the [`IntoForkSystem::fork`] method to pass input from one system into the next.
pub trait IntoForkSystem<ParamA, Payload, SystemB, ParamB, Out>:
    IntoSystem<(), Payload, ParamA> + Sized
where
    SystemB: IntoSystem<(), Out, ParamB>,
{
    /// Pass the output of this system `A` into a second system `B`, creating a new compound system.
    fn fork(self, system: SystemB) -> ForkSystem<Self::System, SystemB::System>;
}

impl<SystemA, ParamA, Payload, SystemB, ParamB, Out>
    IntoForkSystem<ParamA, Payload, SystemB, ParamB, Out> for SystemA
where
    SystemA: IntoSystem<(), Payload, ParamA>,
    SystemB: IntoSystem<(), Out, ParamB>,
{
    fn fork(self, system: SystemB) -> ForkSystem<SystemA::System, SystemB::System> {
        let system_a = IntoSystem::into_system(self);
        let system_b = IntoSystem::into_system(system);
        ForkSystem {
            name: Cow::Owned(format!("Fork({}, {})", system_a.name(), system_b.name())),
            system_a,
            system_b,
            archetype_component_access: Default::default(),
            component_access: Default::default(),
        }
    }
}

