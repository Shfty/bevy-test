use bevy::{
    prelude::{default, Component, Plugin, Query, Res, Deref, DerefMut, CoreStage},
    time::Time, render::extract_component::ExtractComponent,
};

pub struct TimelinePlugin;

impl Plugin for TimelinePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system_to_stage(CoreStage::PreUpdate, timeline);
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Timeline {
    pub timestamp: f64,
    pub prev_timestamp: f64,
    pub timescale: f64,
}

impl Default for Timeline {
    fn default() -> Self {
        Timeline {
            timestamp: default(),
            prev_timestamp: default(),
            timescale: 1.0,
        }
    }
}

impl Timeline {
    pub fn tick(&mut self, dt: f64) {
        self.prev_timestamp = self.timestamp;
        self.timestamp += dt * self.timescale;
    }

    pub fn delta(&self) -> f64 {
        self.timestamp - self.prev_timestamp
    }
}

#[derive(Debug, Default, Copy, Clone, Deref, DerefMut, Component)]
pub struct TimelineComponent(pub Timeline);

impl ExtractComponent for TimelineComponent {
    type Query = &'static Self;

    type Filter = ();

    fn extract_component(item: bevy::ecs::query::QueryItem<'_, Self::Query>) -> Self {
        *item
    }
}

pub fn timeline(time: Res<Time>, mut query: Query<&mut TimelineComponent>) {
    let delta = time.delta_seconds_f64();
    for mut timeline in query.iter_mut() {
        timeline.tick(delta);
    }
}

