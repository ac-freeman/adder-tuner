use bevy::prelude::Res;
use bevy::time::Time;
use bevy_egui::egui::Ui;
use bevy::ecs::system::Resource;

#[derive(Resource, Default)]
pub struct PlayerState {
    // pub(crate) transcoder: AdderTranscoder,
    // pub ui_state: ParamsUiState,
    // pub(crate) ui_state_mem: UiStateMemory,
    // pub ui_info_state: InfoUiState,
}

impl PlayerState {
    pub fn side_panel_ui(
        &mut self,
        mut ui: &mut Ui,
    ) {

    }

    pub fn central_panel_ui(
        &mut self,
        mut ui: &mut Ui,
        time: Res<Time>
    ) {

    }
}