use std::{
    collections::{BTreeMap, VecDeque},
    time::Instant,
};

use bevy::{
    diagnostic::{DiagnosticId, Diagnostics},
    prelude::{default, Camera, Local, Plugin, Query, Res, ResMut, UVec2},
    render::camera::Viewport,
};
use bevy_egui::{
    egui::{self, plot::Line, Frame, Ui},
    EguiContext,
};
use bevy_rapier3d::prelude::RapierContext;

use crate::timeline::TimelineComponent;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugin(bevy_egui::EguiPlugin);

        app.add_system(timeline_panel)
            .add_system(diagnostic_widget);
            //.add_system(intersection_widget);

        // NOTE: Breaks bloom, fix coming in bevy 0.10
        //app.add_system(camera_viewport.after(diagnostic_panel));
    }
}

fn timeline_panel(mut ctx: ResMut<EguiContext>, mut query: Query<&mut TimelineComponent>) {
    let ctx = ctx.ctx_mut();
    let panel = egui::TopBottomPanel::bottom("timeline_panel");

    panel.show(ctx, |ui| {
        ui.vertical(|ui| {
            let width = ui.available_width();
            let style = ui.style_mut();
            style.spacing.slider_width = width;

            let mut timeline = query.iter_mut().next().expect("Missing Timeline entity");
            let response = ui.add(egui::widgets::Slider::new(
                &mut timeline.timestamp,
                0.0..=100.0,
            ));

            if response.drag_started() {
                timeline.timescale = 0.0;
            }

            if response.drag_released() {
                timeline.timescale = 1.0;
            }

            ui.reset_style();
        })
    });
}

fn intersection_widget(mut ctx: ResMut<EguiContext>, rapier: Res<RapierContext>) {
    egui::Window::new("Intersections").show(ctx.ctx_mut(), |ui| {
        for (lhs, rhs, intersection) in rapier.intersection_pairs() {
            let text = format!("{lhs:?} / {rhs:?}: {intersection:}");
            ui.label(text);
        }
    });
}

fn diagnostic_widget(
    mut diag_widget: Local<DiagnosticsWidget>,
    mut ctx: ResMut<EguiContext>,
    diag: Res<Diagnostics>,
) {
    egui::Window::new("Diagnostics")
        .scroll2([false, true])
        .show(ctx.ctx_mut(), diag_widget.diagnostics(&diag));
}

pub struct DiagnosticsWidget {
    start: Instant,
    history: BTreeMap<DiagnosticId, VecDeque<[f64; 2]>>,
}

impl Default for DiagnosticsWidget {
    fn default() -> Self {
        DiagnosticsWidget {
            start: Instant::now(),
            history: default(),
        }
    }
}

impl DiagnosticsWidget {
    pub fn diagnostics<'a>(&'a mut self, diag: &'a Diagnostics) -> impl FnMut(&mut Ui) + 'a {
        move |ui: &mut Ui| {
            let mut diagnostics = diag.iter().collect::<Vec<_>>();
            diagnostics.sort_by(|lhs, rhs| lhs.name.cmp(&rhs.name));

            for diag in diagnostics {
                let entry = self.history.entry(diag.id).or_default();

                let Some(measurement) = diag.measurement() else {
                continue
            };

                entry.push_back([
                    measurement.time.duration_since(self.start).as_secs_f64(),
                    diag.measurement().unwrap().value,
                ]);

                while entry.len() > 201 {
                    entry.pop_front();
                }

                ui.set_width(200.0);

                ui.label(format!(
                    "{} ({:.2})",
                    diag.name.as_ref(),
                    diag.average().unwrap_or_default()
                ));

                if diag.history_len() > 1 {
                    egui::widgets::plot::Plot::new(diag.name.as_ref())
                        .height(100.0)
                        .include_y(0)
                        .show(ui, |plot| {
                            plot.line(Line::new(entry.iter().copied().collect::<Vec<_>>()))
                        });
                }
            }
        }
    }
}

#[allow(unused)]
fn camera_viewport(mut ctx: ResMut<EguiContext>, mut query: Query<&mut Camera>) {
    let ctx = ctx.ctx_mut();

    egui::CentralPanel::default()
        .frame(Frame::none())
        .show(ctx, move |ui| {
            for mut camera in query.iter_mut() {
                let size = ui.available_size() * ctx.pixels_per_point();
                camera.viewport = Some(Viewport {
                    physical_size: UVec2::new(size.x as u32, size.y as u32),
                    ..default()
                });
            }
        });
}
