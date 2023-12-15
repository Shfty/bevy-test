use bevy::prelude::{Local, Query};

use crate::timeline::TimelineComponent;

pub fn fixed_tick(step: f64) -> impl FnMut(Local<f64>, Query<&TimelineComponent>) -> bool {
    move |mut last_tick: Local<f64>, query: Query<&TimelineComponent>| {
        let timeline = query.get_single().expect("Missing Timeline entity");
        let timestamp = timeline.timestamp;

        let delta = timestamp - *last_tick;

        if delta.abs() >= step {
            *last_tick += step * delta.signum();
            true
        } else {
            false
        }
    }
}

