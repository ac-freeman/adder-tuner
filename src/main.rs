mod adder;

use std::cmp::min;
use std::error::Error;
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::ecs::system::Resource;
use bevy::prelude::*;
use bevy::window::PresentMode;
use bevy_editor_pls::egui::TextureId;
use bevy_egui::{egui, EguiContext, EguiPlugin, EguiSettings};
use bevy_egui::egui::{Color32, RichText};
use bevy_editor_pls::prelude::*;
use rayon::current_num_threads;
use crate::adder::{AdderTranscoder, consume_source, update_adder_params};

/// This example demonstrates the following functionality and use-cases of bevy_egui:
/// - rendering loaded assets;
/// - toggling hidpi scaling (by pressing '/' button);
/// - configuring egui contexts during the startup.
fn main() {
    App::new()
        .insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)))
        .insert_resource(Msaa { samples: 4 })
        .insert_resource(AdderTranscoder::default())
        .insert_resource(Images::default())
        .init_resource::<UiState>()
        .init_resource::<UiStateMemory>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            window: WindowDescriptor {
                title: "ADΔER Tuner".to_string(),
                width: 1280.,
                height: 720.,
                present_mode: PresentMode::AutoVsync,
                ..default()
            },
            ..default()
        }))
        .add_plugin(EguiPlugin)
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin)
        // .add_plugin(EditorPlugin)
        .add_startup_system(configure_visuals)
        .add_startup_system(configure_ui_state)
        .add_system(update_ui_scale_factor)
        .add_system(ui_example)
        .add_system(file_drop)
        .add_system(update_adder_params)
        .add_system(consume_source)
        .run();
}

#[derive(Resource, Default)]
struct Images {
    image_view: Handle<Image>,
}

#[derive(Resource)]
struct UiStateMemory {
    delta_t_ref_slider: f32,
    delta_t_max_mult_slider: u32,
    adder_tresh_slider: f32,
    scale_slider: f64,
}
impl Default for UiStateMemory {
    fn default() -> Self {
        UiStateMemory {
            delta_t_ref_slider: 255.0,
            delta_t_max_mult_slider: 120,
            adder_tresh_slider: 10.0,
            scale_slider: 0.5
        }
    }
}

#[derive(Resource)]
struct UiState {
    label: String,
    delta_t_ref: f32,
    delta_t_ref_max: f32,
    delta_t_max_mult: u32,
    adder_tresh: f32,
    delta_t_ref_slider: f32,
    delta_t_max_mult_slider: u32,
    adder_tresh_slider: f32,
    scale: f64,
    scale_slider: f64,
    events_per_sec: f64,
    events_ppc_per_sec: f64,
    events_ppc_total: u64,
    events_total: u64,
    drop_target: MyDropTarget,
    inverted: bool,
    egui_texture_handle: Option<egui::TextureHandle>,
    // image: Handle<Image>,
    source_name: RichText,
    thread_count: usize,
    is_window_open: bool,
}

impl Default for UiState {
    fn default() -> Self {
        UiState {
            label: "".to_string(),
            delta_t_ref: 255.0,
            delta_t_ref_max: 255.0,
            delta_t_max_mult: 120,
            adder_tresh: 10.0,
            delta_t_ref_slider: 255.0,
            delta_t_max_mult_slider: 120,
            adder_tresh_slider: 10.0,
            scale: 0.5,
            scale_slider: 0.5,
            events_per_sec: 0.,
            events_ppc_per_sec: 0.,
            events_ppc_total: 0,
            events_total: 0,
            drop_target: Default::default(),
            inverted: false,
            egui_texture_handle: None,
            // image: Default::default(),
            source_name: RichText::new("No file selected yet"),
            thread_count: 4,
            is_window_open: true
        }
    }
}

fn configure_visuals(mut egui_ctx: ResMut<EguiContext>) {
    egui_ctx.ctx_mut().set_visuals(egui::Visuals {
        window_rounding: 5.0.into(),
        ..Default::default()
    });
}

fn configure_ui_state(mut ui_state: ResMut<UiState>) {
    ui_state.is_window_open = true;
}

fn update_ui_scale_factor(
    keyboard_input: Res<Input<KeyCode>>,
    mut toggle_scale_factor: Local<Option<bool>>,
    mut egui_settings: ResMut<EguiSettings>,
    windows: Res<Windows>,
) {
    if keyboard_input.just_pressed(KeyCode::Slash) || toggle_scale_factor.is_none() {
        *toggle_scale_factor = Some(!toggle_scale_factor.unwrap_or(true));

        if let Some(window) = windows.get_primary() {
            let scale_factor = if toggle_scale_factor.unwrap() {
                1.0
            } else {
                1.0 / window.scale_factor()
            };
            egui_settings.scale_factor = scale_factor;
        }
    }
}

fn ui_example(
    mut commands: Commands,
    time: Res<Time>, // Time passed since last frame
    handles: Res<Images>,
    mut images: ResMut<Assets<Image>>,
    mut egui_ctx: ResMut<EguiContext>,
    mut ui_state: ResMut<UiState>,
) {
    egui::SidePanel::left("side_panel")
        .default_width(300.0)
        .show(egui_ctx.ctx_mut(), |ui| {
            ui.heading("Side Panel");

            let dtr_max = ui_state.delta_t_ref_max;
            ui.add(egui::Slider::new(&mut ui_state.delta_t_ref_slider, 1.0..=dtr_max).text("Δt_ref"));
            if ui.button("Increment").clicked() {
                ui_state.delta_t_ref_slider += 1.0;
            }

            ui.add(egui::Slider::new(&mut ui_state.delta_t_max_mult_slider, 2..=1000).text("Δt_max multiplier"));
            if ui.button("Increment").clicked() {
                ui_state.delta_t_max_mult_slider += 1;
            }
            ui.add(egui::Slider::new(&mut ui_state.adder_tresh_slider, 0.0..=255.0).text("ADΔER threshold"));
            if ui.button("Increment").clicked() {
                ui_state.adder_tresh_slider += 1.0;
            }

            ui.add(egui::Slider::new(&mut ui_state.thread_count, 1..=current_num_threads()).text("Thread count"));
            if ui.button("Increment").clicked() {
                ui_state.thread_count += 1;
                ui_state.thread_count = ui_state.thread_count.min(current_num_threads());
            }

            ui.add(egui::Slider::new(&mut ui_state.scale_slider, 0.01..=1.0).text("Video scale"));
            if ui.button("Decrement").clicked() {
                ui_state.scale_slider -= 0.1;
                ui_state.scale_slider = ui_state.scale_slider.max(0.01);
            }
            if ui.button("Increment").clicked() {
                ui_state.scale_slider += 0.1;
                ui_state.scale_slider = ui_state.scale_slider.min(1.0);
            }

            ui.allocate_space(egui::Vec2::new(1.0, 100.0));

            ui.allocate_space(egui::Vec2::new(1.0, 10.0));
            ui.checkbox(&mut ui_state.is_window_open, "Window Is Open");
        });

    egui::TopBottomPanel::top("top_panel").show(egui_ctx.ctx_mut(), |ui| {
        // The top panel is often a good place for a menu bar:
        egui::menu::bar(ui, |ui| {
            egui::menu::menu_button(ui, "File", |ui| {
                if ui.button("Quit").clicked() {
                    std::process::exit(0);
                }
            });
        });
    });

    let (image, texture_id) = match images.get(&handles.image_view) {
        // texture_id = Some(egui_ctx.add_image(handles.image_view.clone()));
        None => { (None, None)}
        Some(image) => {
            (Some(image),Some(egui_ctx.add_image(handles.image_view.clone())))
        }
    };

    egui::CentralPanel::default().show(egui_ctx.ctx_mut(), |ui| {
        ui.heading("Egui Template");
        egui::warn_if_debug_build(ui);

        ui.separator();

        ui.heading("Drag and drop your source file here.");



        ui.label(ui_state.source_name.clone());

        ui.label(format!(
            "{:.2} transcoded FPS\t\
            {:.2} events per source sec\t\
            {:.2} events PPC per source sec\t\
            {:.0} events total\t\
            {:.0} events PPC total
            ",
                1. / time.delta_seconds(),
                ui_state.events_per_sec,
                ui_state.events_ppc_per_sec,
                ui_state.events_total,
                ui_state.events_ppc_total
        ));


        match (image, texture_id) {
            (Some(image), Some(texture_id)) => {
                let avail_size = ui.available_size();
                let mut size= Default::default();

                size = match (image.texture_descriptor.size.width as f32, image.texture_descriptor.size.height as f32) {
                    (a, b) if a/b > avail_size.x/avail_size.y => {
                        /*
                        The available space has a taller aspect ratio than the video
                        Fill the available horizontal space.
                         */
                        bevy_egui::egui::Vec2 { x: avail_size.x, y: (avail_size.x/a) * b }
                    }
                    (a, b) => {
                        /*
                        The available space has a shorter aspect ratio than the video
                        Fill the available vertical space.
                         */
                        bevy_egui::egui::Vec2 { x: (avail_size.y/b) * a, y: avail_size.y }
                    }
                };
                ui.image(texture_id,  size);
            }
            _ => {}
        }


    });

    egui::Window::new("Window")
        .vscroll(true)
        .open(&mut ui_state.is_window_open)
        .show(egui_ctx.ctx_mut(), |ui| {
            ui.label("Windows can be moved by dragging them.");
            ui.label("They are automatically sized based on contents.");
            ui.label("You can turn on resizing and scrolling if you like.");
            ui.label("You would normally chose either panels OR windows.");
        });
}

#[derive(Component, Default)]
struct MyDropTarget;


///https://bevy-cheatbook.github.io/input/dnd.html
fn file_drop(
    mut ui_state: ResMut<UiState>,
    mut commands: Commands,
    mut dnd_evr: EventReader<FileDragAndDrop>,
    query_ui_droptarget: Query<&Interaction, With<MyDropTarget>>,
) {
    for ev in dnd_evr.iter() {
        println!("{:?}", ev);
        if let FileDragAndDrop::DroppedFile { id, path_buf } = ev {
            println!("Dropped file with path: {:?}", path_buf);

            if id.is_primary() {
                // it was dropped over the main window

            }

            for interaction in query_ui_droptarget.iter() {
                if *interaction == Interaction::Hovered {
                    // it was dropped over our UI element
                    // (our UI element is being hovered over)
                }
            }

            replace_adder_transcoder(&mut commands, &mut ui_state, path_buf, 0);
        }
    }
}

pub(crate) fn replace_adder_transcoder(commands: &mut Commands, mut ui_state: &mut ResMut<UiState>, path_buf: &std::path::PathBuf, current_frame: u32) {
    ui_state.events_per_sec = 0.0;
    ui_state.events_ppc_total = 0;
    ui_state.events_total = 0;
    ui_state.events_ppc_per_sec = 0.0;
    match AdderTranscoder::new(path_buf, &mut ui_state, current_frame) {
        Ok(transcoder) => {
            commands.remove_resource::<AdderTranscoder>();
            commands.insert_resource
            (
                transcoder
            );
            ui_state.source_name = RichText::new(path_buf.to_str().unwrap()).color(Color32::DARK_GREEN);

        }
        Err(e) => {
            commands.remove_resource::<AdderTranscoder>();
            commands.insert_resource
            (
                AdderTranscoder::default()
            );
            ui_state.source_name = RichText::new(e.to_string()).color(Color32::RED);
        }
    };
}

