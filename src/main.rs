mod transcoder;
mod player;


use std::ops::{Deref, RangeInclusive};
use adder_codec_rs::transcoder::source::davis_source::DavisTranscoderMode;
use adder_codec_rs::transcoder::source::framed_source::FramedSource;
use adder_codec_rs::transcoder::source::video::InstantaneousViewMode;
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::ecs::system::Resource;
use bevy::prelude::*;
use bevy::window::PresentMode;
use crate::player::ui::PlayerState;
use crate::transcoder::ui::{ParamsUiState, InfoUiState, UiStateMemory, TranscoderState};

use bevy_egui::{egui, EguiContext, EguiPlugin, EguiSettings};
// use egui_dock::egui as dock_egui;
use bevy_egui::egui::{Color32, emath, global_dark_light_mode_switch, Response, RichText, Rounding, Stroke, Ui, Widget, WidgetText};

use strum::IntoEnumIterator;
use strum_macros::EnumIter;

// use egui_dock::{NodeIndex, Tree};

#[derive(Debug, EnumIter, PartialEq, Clone, Copy)]
enum Tabs {
    Transcoder,
    Player,
}

impl Tabs {
    fn as_str(&self) -> &'static str {
        match self {
            Tabs::Transcoder => {"Transcode"}
            Tabs::Player => {"Play file"}
        }
    }
}

#[derive(Resource)]
pub struct MainUiState {
    view: Tabs,
}

use rayon::current_num_threads;
use crate::transcoder::adder;
use crate::transcoder::adder::{AdderTranscoder, consume_source, replace_adder_transcoder};

/// This example demonstrates the following functionality and use-cases of bevy_egui:
/// - rendering loaded assets;
/// - toggling hidpi scaling (by pressing '/' button);
/// - configuring egui contexts during the startup.
fn main() {
    App::new()
        .insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)))
        .insert_resource(Msaa { samples: 4 })
        .insert_resource(Images::default())
        .insert_resource(MainUiState { view: Tabs::Transcoder })
        .init_resource::<TranscoderState>()
        .init_resource::<PlayerState>()
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
        .add_system(configure_menu_bar)
        .add_system(update_ui_scale_factor)
        .add_system(draw_ui)
        .add_system(file_drop)
        .add_system(update_adder_params)
        .add_system(consume_source)
        .run();
}

#[derive(Resource, Default)]
pub struct Images {
    image_view: Handle<Image>,
}




fn configure_visuals(mut egui_ctx: ResMut<EguiContext>) {
    egui_ctx.ctx_mut().set_visuals(bevy_egui::egui::Visuals {
        window_rounding: 5.0.into(),
        ..Default::default()
    });
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

fn configure_menu_bar(
    mut main_ui_state: ResMut<MainUiState>,
    mut egui_ctx: ResMut<EguiContext>,
) {
    let mut style = (*(*egui_ctx).ctx_mut().clone().style()).clone();

    egui::TopBottomPanel::top("top_panel").show(egui_ctx.ctx_mut(), |ui| {
        egui::menu::bar(ui, |ui| {
            ui.style_mut().visuals.widgets.active.rounding = Rounding::same(0.0);
            ui.style_mut().visuals.widgets.inactive.rounding = Rounding::same(0.0);
            ui.style_mut().visuals.widgets.open.rounding = Rounding::same(0.0);
            ui.style_mut().visuals.widgets.hovered.rounding = Rounding::same(0.0);
            ui.style_mut().visuals.widgets.noninteractive.rounding = Rounding::same(0.0);
            ui.style_mut().visuals.widgets.inactive.expansion = 3.0;
            ui.style_mut().visuals.widgets.active.expansion = 3.0;
            ui.style_mut().visuals.widgets.hovered.expansion = 3.0;
            let default_inactive_stroke = ui.style_mut().visuals.widgets.inactive.fg_stroke;

            let mut new_selection = main_ui_state.view;
            for menu_item in Tabs::iter() {
                let button = {
                    if main_ui_state.view == menu_item {
                        ui.style_mut().visuals.widgets.inactive.fg_stroke = ui.style_mut().visuals.widgets.active.fg_stroke;
                        egui::Button::new(menu_item.as_str()).fill(style.visuals.window_fill)
                    } else {
                        ui.style_mut().visuals.widgets.inactive.fg_stroke = default_inactive_stroke;
                        egui::Button::new(menu_item.as_str()).fill(style.visuals.faint_bg_color)
                    }
                };
                let res = button.ui(ui);
                if res.clicked() {
                    new_selection = menu_item;
                }
            }

            // Now that all the menu items have been drawn, set the selected item for when the next
            // frame is drawn
            main_ui_state.view = new_selection;
        });
    });
}

fn draw_ui(
    mut commands: Commands,
    time: Res<Time>, // Time passed since last frame
    handles: Res<Images>,
    images: ResMut<Assets<Image>>,
    mut egui_ctx: ResMut<EguiContext>,
    // mut ui_state: ResMut<ParamsUiState>,
    // mut ui_info_state: ResMut<InfoUiState>,
    mut transcoder_state: ResMut<TranscoderState>,
    mut player_state: ResMut<PlayerState>,
    main_ui_state: Res<MainUiState>,
) {
    egui::SidePanel::left("side_panel")
        .default_width(300.0)
        .show(egui_ctx.ctx_mut(), |ui| {
            ui.horizontal(|ui|{
                ui.heading("ADΔER Parameters");
                global_dark_light_mode_switch(ui);
                if ui.add(egui::Button::new("Reset params")).clicked() {
                    transcoder_state.ui_state = Default::default();
                }
                if ui.add(egui::Button::new("Reset video")).clicked() {
                    transcoder_state.transcoder = AdderTranscoder::default();
                    transcoder_state.ui_info_state = InfoUiState::default();
                    commands.insert_resource(Images::default());
                }
            });

            match main_ui_state.view {
                Tabs::Transcoder => {
                    transcoder_state.side_panel_ui(ui);
                }
                Tabs::Player => {
                    player_state.side_panel_ui(ui);
                }
            }



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

        ui.heading("Drag and drop your source file here.");


        match main_ui_state.view {
            Tabs::Transcoder => {
                transcoder_state.central_panel_ui(ui, time);
            }
            Tabs::Player => {
                player_state.central_panel_ui(ui, time);
            }
        }

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
}

fn update_adder_params(mut transcoder_state: ResMut<TranscoderState>,
                       mut commands: Commands) {
    transcoder_state.update_adder_params(commands);
}

#[derive(Component, Default)]
struct MyDropTarget;


///https://bevy-cheatbook.github.io/input/dnd.html
fn file_drop(
    mut transcoder_state: ResMut<TranscoderState>,
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

            replace_adder_transcoder(
                &mut commands,
                &mut transcoder_state,
                path_buf, 0);
        }
    }
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