mod adder;



use std::ops::RangeInclusive;
use adder_codec_rs::transcoder::source::davis_source::DavisTranscoderMode;
use adder_codec_rs::transcoder::source::framed_source::FramedSource;
use adder_codec_rs::transcoder::source::video::InstantaneousViewMode;
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::ecs::system::Resource;
use bevy::prelude::*;
use bevy::window::PresentMode;

use bevy_egui::{egui, EguiContext, EguiPlugin, EguiSettings};
use bevy_egui::egui::{Color32, emath, global_dark_light_mode_switch, RichText, Ui};


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
        .init_resource::<ParamsUiState>()
        .init_resource::<InfoUiState>()
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
        // .add_plugin(LogDiagnosticsPlugin::default())
        // .add_plugin(FrameTimeDiagnosticsPlugin)
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
struct ParamsUiState {
    delta_t_ref: f32,
    delta_t_ref_max: f32,
    delta_t_max_mult: u32,
    adder_tresh: f32,
    delta_t_ref_slider: f32,
    delta_t_max_mult_slider: u32,
    adder_tresh_slider: f32,
    scale: f64,
    scale_slider: f64,
    thread_count: usize,
    color: bool,
    view_mode_radio_state: InstantaneousViewMode,
    davis_mode_radio_state: DavisTranscoderMode,
}

impl Default for ParamsUiState {
    fn default() -> Self {
        ParamsUiState {
            delta_t_ref: 255.0,
            delta_t_ref_max: 255.0,
            delta_t_max_mult: 120,
            adder_tresh: 10.0,
            delta_t_ref_slider: 255.0,
            delta_t_max_mult_slider: 120,
            adder_tresh_slider: 10.0,
            scale: 0.5,
            scale_slider: 0.5,
            thread_count: 4,
            color: true,
            view_mode_radio_state: InstantaneousViewMode::Intensity,
            davis_mode_radio_state: DavisTranscoderMode::RawDavis,
        }
    }
}

#[derive(Resource)]
struct InfoUiState {
    events_per_sec: f64,
    events_ppc_per_sec: f64,
    events_ppc_total: u64,
    events_total: u64,
    source_name: RichText,
    view_mode_radio_state: InstantaneousViewMode,
}

impl Default for InfoUiState {
    fn default() -> Self {
        InfoUiState {
            events_per_sec: 0.,
            events_ppc_per_sec: 0.,
            events_ppc_total: 0,
            events_total: 0,
            source_name: RichText::new("No file selected yet"),
            view_mode_radio_state: InstantaneousViewMode::Intensity,
        }
    }
}

fn configure_visuals(mut egui_ctx: ResMut<EguiContext>) {
    egui_ctx.ctx_mut().set_visuals(egui::Visuals {
        window_rounding: 5.0.into(),
        ..Default::default()
    });
}

fn configure_ui_state(mut ui_state: ResMut<ParamsUiState>) {
    ui_state.color = true;
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
    transcoder: Res<AdderTranscoder>,
    images: ResMut<Assets<Image>>,
    mut egui_ctx: ResMut<EguiContext>,
    mut ui_state: ResMut<ParamsUiState>,
    mut ui_info_state: ResMut<InfoUiState>,
) {
    egui::SidePanel::left("side_panel")
        .default_width(300.0)
        .show(egui_ctx.ctx_mut(), |ui| {
            ui.horizontal(|ui|{
                ui.heading("ADΔER Parameters");
                global_dark_light_mode_switch(ui);
                if ui.add(egui::Button::new("Reset params")).clicked() {
                    commands.insert_resource::<ParamsUiState>(Default::default());
                }
                if ui.add(egui::Button::new("Reset video")).clicked() {
                    commands.insert_resource::<AdderTranscoder>(AdderTranscoder::default());
                    commands.insert_resource::<InfoUiState>(InfoUiState::default());
                    commands.insert_resource(Images::default());
                }
            });

            egui::Grid::new("my_grid")
                .num_columns(2)
                .spacing([10.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    side_panel_grid_contents(transcoder, ui, &mut ui_state);
                });


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
        egui::warn_if_debug_build(ui);
        // ui.separator();

        ui.heading("Drag and drop your source file here.");



        ui.label(ui_info_state.source_name.clone());

        ui.label(format!(
            "{:.2} transcoded FPS\t\
            {:.2} events per source sec\t\
            {:.2} events PPC per source sec\t\
            {:.0} events total\t\
            {:.0} events PPC total
            ",
                1. / time.delta_seconds(),
            ui_info_state.events_per_sec,
            ui_info_state.events_ppc_per_sec,
            ui_info_state.events_total,
            ui_info_state.events_ppc_total
        ));




        match (image, texture_id) {
            (Some(image), Some(texture_id)) => {
                let avail_size = ui.available_size();
                let size = match (image.texture_descriptor.size.width as f32, image.texture_descriptor.size.height as f32) {
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

    // egui::Window::new("Window")
    //     .vscroll(true)
    //     .open(&mut ui_state.is_window_open)
    //     .show(egui_ctx.ctx_mut(), |ui| {
    //         ui.label("Windows can be moved by dragging them.");
    //         ui.label("They are automatically sized based on contents.");
    //         ui.label("You can turn on resizing and scrolling if you like.");
    //         ui.label("You would normally chose either panels OR windows.");
    //     });
}

#[derive(Component, Default)]
struct MyDropTarget;


///https://bevy-cheatbook.github.io/input/dnd.html
fn file_drop(
    mut ui_state: ResMut<ParamsUiState>,
    mut ui_info_state: ResMut<InfoUiState>,
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

            replace_adder_transcoder(&mut commands, &mut ui_state, &mut ui_info_state, path_buf, 0);
        }
    }
}

pub(crate) fn replace_adder_transcoder(commands: &mut Commands,
                                       ui_state: &mut ResMut<ParamsUiState>,
                                       mut ui_info_state: &mut ResMut<InfoUiState>,
                                       path_buf: &std::path::PathBuf,
                                       current_frame: u32) {
    ui_info_state.events_per_sec = 0.0;
    ui_info_state.events_ppc_total = 0;
    ui_info_state.events_total = 0;
    ui_info_state.events_ppc_per_sec = 0.0;
    match AdderTranscoder::new(path_buf, ui_state, current_frame) {
        Ok(transcoder) => {
            commands.remove_resource::<AdderTranscoder>();
            commands.insert_resource
            (
                transcoder
            );
            ui_info_state.source_name = RichText::new(path_buf.to_str().unwrap()).color(Color32::DARK_GREEN);

        }
        Err(e) => {
            commands.remove_resource::<AdderTranscoder>();
            commands.insert_resource
            (
                AdderTranscoder::default()
            );
            ui_info_state.source_name = RichText::new(e.to_string()).color(Color32::RED);
        }
    };
}

fn side_panel_grid_contents(transcoder: Res<AdderTranscoder>, ui: &mut Ui, ui_state: &mut ResMut<ParamsUiState>) {
    let dtr_max = ui_state.delta_t_ref_max;
    let enabled = match transcoder.davis_source {
        None => { true}
        Some(_) => { false }
    };
    ui.add_enabled(enabled, egui::Label::new("Δt_ref:"));
    slider_pm(enabled, ui, &mut ui_state.delta_t_ref_slider, 1.0..=dtr_max, 10.0);
    ui.end_row();

    ui.label("Δt_max multiplier:");
    slider_pm(true, ui, &mut ui_state.delta_t_max_mult_slider, 2..=1000, 10);
    ui.end_row();

    ui.label("ADΔER threshold:");
    slider_pm(true, ui, &mut ui_state.adder_tresh_slider, 0.0..=255.0, 1.0);
    ui.end_row();


    ui.label("Thread count:");
    slider_pm(true, ui, &mut ui_state.thread_count, 1..=(current_num_threads()-1).max(4), 1);
    ui.end_row();

    ui.label("Video scale:");
    slider_pm(enabled, ui, &mut ui_state.scale_slider, 0.01..=1.0, 0.1);
    ui.end_row();


    ui.label("Channels:");
    ui.add_enabled(enabled, egui::Checkbox::new(&mut ui_state.color, "Color?"));
    ui.end_row();


    ui.label("View mode:");
    ui.horizontal(|ui| {
        ui.radio_value(&mut ui_state.view_mode_radio_state, InstantaneousViewMode::Intensity, "Intensity");
        ui.radio_value(&mut ui_state.view_mode_radio_state, InstantaneousViewMode::D, "D");
        ui.radio_value(&mut ui_state.view_mode_radio_state, InstantaneousViewMode::DeltaT, "Δt");
    });
    ui.end_row();

    ui.label("Davis mode:");
    ui.add_enabled_ui(!enabled, |ui| {
        ui.horizontal(|ui| {
            ui.radio_value(&mut ui_state.davis_mode_radio_state, DavisTranscoderMode::Framed, "Framed recon");
            ui.radio_value(&mut ui_state.davis_mode_radio_state, DavisTranscoderMode::RawDavis, "Raw DAVIS");
            ui.radio_value(&mut ui_state.davis_mode_radio_state, DavisTranscoderMode::RawDvs, "Raw DVS");
        });
    });
    ui.end_row();
}

fn slider_pm<Num: emath::Numeric + Pm>(enabled: bool, ui: &mut Ui, value: &mut Num, range: RangeInclusive<Num>, interval: Num) {
    ui.add_enabled_ui(enabled, |ui| {
        ui.horizontal(|ui| {
            if ui.button("-").clicked() {
                value.decrement(range.start(), &interval);
            }
            ui.add(egui::Slider::new(value, range.clone()));
            if ui.button("+").clicked() {
                value.increment(range.end(), &interval);
            }
        });
    });
}

trait Pm {
    fn increment(&mut self, bound: &Self, interval: &Self);
    fn decrement(&mut self, bound: &Self, interval: &Self);
}

macro_rules! impl_pm_float {
    ($t: ident) => {
        impl Pm for $t {
            #[inline(always)]
            fn increment(&mut self, bound: &Self, interval: &Self) {
                #[allow(trivial_numeric_casts)]
                {
                    *self += *interval;
                    if *self > *bound {
                        *self = *bound
                    }
                }
            }

            #[inline(always)]
            fn decrement(&mut self, bound: &Self, interval: &Self) {
                #[allow(trivial_numeric_casts)]
                {
                    *self -= *interval;
                    if *self < *bound {
                        *self = *bound
                    }
                }
            }
        }
    };
}
macro_rules! impl_pm_integer {
    ($t: ident) => {
        impl Pm for $t {
            #[inline(always)]
            fn increment(&mut self, bound: &Self, interval: &Self) {
                #[allow(trivial_numeric_casts)]
                {
                    *self = self.saturating_add(*interval);
                    if *self > *bound {
                        *self = *bound
                    }
                }
            }

            #[inline(always)]
            fn decrement(&mut self, bound: &Self, interval: &Self) {
                #[allow(trivial_numeric_casts)]
                {
                    *self = self.saturating_sub(*interval);
                    if *self < *bound {
                        *self = *bound
                    }
                }
            }
        }
    };
}

impl_pm_float!(f32);
impl_pm_float!(f64);
impl_pm_integer!(i8);
impl_pm_integer!(u8);
impl_pm_integer!(i16);
impl_pm_integer!(u16);
impl_pm_integer!(i32);
impl_pm_integer!(u32);
impl_pm_integer!(i64);
impl_pm_integer!(u64);
impl_pm_integer!(isize);
impl_pm_integer!(usize);