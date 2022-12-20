use adder_codec_rs::transcoder::source::davis_source::DavisTranscoderMode;
use adder_codec_rs::transcoder::source::video::InstantaneousViewMode;
use bevy::ecs::system::Resource;
use bevy_egui::egui::RichText;



#[derive(Resource)]
pub struct ParamsUiState {
    pub delta_t_ref: f32,
    pub delta_t_ref_max: f32,
    pub delta_t_max_mult: u32,
    pub adder_tresh: f32,
    pub delta_t_ref_slider: f32,
    pub delta_t_max_mult_slider: u32,
    pub adder_tresh_slider: f32,
    pub scale: f64,
    pub scale_slider: f64,
    pub thread_count: usize,
    pub color: bool,
    pub view_mode_radio_state: InstantaneousViewMode,
    pub davis_mode_radio_state: DavisTranscoderMode,
    pub davis_output_fps: f64,
    pub optimize_c: bool,
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
            davis_output_fps: 500.0,
            optimize_c: true,
        }
    }
}

#[derive(Resource)]
pub struct UiStateMemory {
    pub delta_t_ref_slider: f32,
    pub delta_t_max_mult_slider: u32,
    pub adder_tresh_slider: f32,
    pub scale_slider: f64,
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
pub struct InfoUiState {
    pub events_per_sec: f64,
    pub events_ppc_per_sec: f64,
    pub events_ppc_total: u64,
    pub events_total: u64,
    pub source_name: RichText,
    pub view_mode_radio_state: InstantaneousViewMode,
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