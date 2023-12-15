use std::{borrow::Cow, marker::PhantomData};

use bevy::{
    ecs::{archetype::ArchetypeComponentId, component::ComponentId, query::Access},
    prelude::{default, System, World},
};

pub struct LiftSystem<F, I, O> {
    f: F,
    component_access: Access<ComponentId>,
    archetype_component_access: Access<ArchetypeComponentId>,
    last_change_tick: u32,
    _phantom: PhantomData<(I, O)>,
}

impl<F, I, O> System for LiftSystem<F, I, O>
where
    F: FnMut(I) -> O + Send + Sync + 'static,
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    type In = I;

    type Out = O;

    fn name(&self) -> Cow<'static, str> {
        format!("Lift({})", std::any::type_name::<F>()).into()
    }

    fn component_access(&self) -> &Access<ComponentId> {
        &self.component_access
    }

    fn archetype_component_access(&self) -> &Access<ArchetypeComponentId> {
        &self.archetype_component_access
    }

    fn is_send(&self) -> bool {
        true
    }

    fn is_exclusive(&self) -> bool {
        false
    }

    unsafe fn run_unsafe(&mut self, input: Self::In, _: &World) -> Self::Out {
        (self.f)(input)
    }

    fn apply_buffers(&mut self, _: &mut World) {}

    fn initialize(&mut self, _: &mut World) {}

    fn update_archetype_component_access(&mut self, _: &World) {}

    fn check_change_tick(&mut self, _: u32) {}

    fn get_last_change_tick(&self) -> u32 {
        self.last_change_tick
    }

    fn set_last_change_tick(&mut self, last_change_tick: u32) {
        self.last_change_tick = last_change_tick
    }
}

pub trait IntoLiftSystem<I, O>: Sized {
    fn lift(self) -> LiftSystem<Self, I, O>;
}

impl<F, I, O> IntoLiftSystem<I, O> for F
where
    F: FnMut(I) -> O + Send + Sync + 'static,
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    fn lift(self) -> LiftSystem<Self, I, O> {
        LiftSystem {
            f: self,
            component_access: default(),
            archetype_component_access: default(),
            last_change_tick: default(),
            _phantom: default(),
        }
    }
}

