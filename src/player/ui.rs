use bevy::asset::Assets;
use bevy::prelude::{Commands, Image, Res, ResMut};
use bevy::time::Time;
use bevy_egui::egui::{Ui};
use bevy::ecs::system::Resource;
use bevy_egui::egui;
use crate::Images;
use crate::player::adder::AdderPlayer;

#[derive(Default)]
pub struct PlayerUiState {
    pub(crate) player: AdderPlayer,
    pub(crate) playback_speed: f32,
    pub(crate) playing: bool,
    pub(crate) current_frame: u32,
    pub(crate) total_frames: u32,
    pub(crate) current_time: f32,
    pub(crate) total_time: f32,
}

#[derive(Resource, Default)]
pub struct PlayerState {
    // pub(crate) transcoder: AdderTranscoder,
    pub ui_state: PlayerUiState,
    // pub(crate) ui_state_mem: UiStateMemory,
    // pub ui_info_state: InfoUiState,
}

impl PlayerState {
    // Fill in the side panel with sliders for playback speed and buttons for play/pause/stop
    pub fn side_panel_ui(
        &mut self,
        mut ui: &mut Ui,
    ) {
        ui.add(
            egui::Slider::new(&mut self.ui_state.playback_speed, 0.0..=10.0)
                .text("Playback speed"),
        );
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Play").clicked() {
                self.ui_state.playing = true;
            }
            if ui.button("Pause").clicked() {
                self.ui_state.playing = false;
            }
            if ui.button("Stop").clicked() {
                self.ui_state.playing = false;
                self.ui_state.current_frame = 0;
            }
        });



    }

    pub fn consume_source(
        &mut self,
        mut images: ResMut<Assets<Image>>,
        mut handles: ResMut<Images>,
        mut commands: Commands,
    ) {

    }

    pub fn central_panel_ui(
        &mut self,
        mut ui: &mut Ui,
        time: Res<Time>
    ) {

        ui.heading("Drag and drop your ADΔER file here (.adder)");
    }

    pub fn replace_player(&mut self, path_buf: &std::path::PathBuf) {
        self.ui_state.player = AdderPlayer::new(path_buf).unwrap();
        self.ui_state.current_frame = 0;

    }
}