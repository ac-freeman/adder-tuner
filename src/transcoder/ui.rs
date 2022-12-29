use crate::transcoder::adder::{replace_adder_transcoder, AdderTranscoder};
use crate::{slider_pm, Images};
use adder_codec_rs::transcoder::source::davis_source::DavisTranscoderMode;
use adder_codec_rs::transcoder::source::video::{FramedViewMode, Source, SourceError};
use bevy::ecs::system::Resource;
use bevy::prelude::{Assets, Commands, Image, Res, ResMut, Time};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy_egui::{egui, EguiContext};
use bevy_egui::egui::{Context, RichText, Ui};
use opencv::core::{Mat, MatTraitConstManual};
use opencv::imgproc;
use rayon::current_num_threads;
use std::path::PathBuf;
use bevy::reflect::GetPath;
use egui_file::FileDialog;

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
    pub thread_count_slider: usize,
    pub color: bool,
    pub view_mode_radio_state: FramedViewMode,
    pub davis_mode_radio_state: DavisTranscoderMode,
    pub davis_output_fps: f64,
    pub davis_output_fps_slider: f64,
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
            thread_count_slider: 4,
            color: true,
            view_mode_radio_state: FramedViewMode::Intensity,
            davis_mode_radio_state: DavisTranscoderMode::RawDavis,
            davis_output_fps: 500.0,
            davis_output_fps_slider: 500.0,
            optimize_c: true,
        }
    }
}

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
            scale_slider: 0.5,
        }
    }
}

pub struct InfoUiState {
    pub events_per_sec: f64,
    pub events_ppc_per_sec: f64,
    pub events_ppc_total: f64,
    pub events_total: u64,
    pub source_name: RichText,
    input_path: Option<PathBuf>,
    pub view_mode_radio_state: FramedViewMode, // TODO: Move to different struct
}

impl Default for InfoUiState {
    fn default() -> Self {
        InfoUiState {
            events_per_sec: 0.,
            events_ppc_per_sec: 0.,
            events_ppc_total: 0.0,
            events_total: 0,
            source_name: RichText::new("No file selected yet"),
            input_path: None,
            view_mode_radio_state: FramedViewMode::Intensity,
        }
    }
}

unsafe impl Sync for InfoUiState {}

#[derive(Resource, Default)]
pub struct TranscoderState {
    pub(crate) transcoder: AdderTranscoder,
    pub ui_state: ParamsUiState,
    pub(crate) ui_state_mem: UiStateMemory,
    pub ui_info_state: InfoUiState,
}

impl TranscoderState {
    pub fn side_panel_ui(
        &mut self,
        ui: &mut Ui,
        mut commands: Commands,
        _images: &mut ResMut<Assets<Image>>,
    ) {
        ui.horizontal(|ui| {
            ui.heading("ADΔER Parameters");
            if ui.add(egui::Button::new("Reset params")).clicked() {
                self.ui_state = Default::default();
            }
            if ui.add(egui::Button::new("Reset video")).clicked() {
                self.transcoder = AdderTranscoder::default();
                self.ui_info_state = InfoUiState::default();
                commands.insert_resource(Images::default());
            }
        });
        egui::Grid::new("my_grid")
            .num_columns(2)
            .spacing([10.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                side_panel_grid_contents(&self.transcoder, ui, &mut self.ui_state);
            });
    }

    pub fn central_panel_ui(&mut self, ctx: &Context, ui: &mut Ui, time: Res<Time>) {
        ui.horizontal(|ui| {
            if ui.button("Open file").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("video", &["mp4", "aedat4"])
                    .pick_file() {
                    self.ui_info_state.input_path = Some(path.clone());
                    replace_adder_transcoder(self, Some(path), None,0);
                }
            }

            ui.label("OR drag and drop your source file here (.mp4, .aedat4)");
        });

        ui.label(self.ui_info_state.source_name.clone());

        if ui.button("Save file").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("adder video", &["adder"])
                .save_file() {
                replace_adder_transcoder(self, self.ui_info_state.input_path.clone(), Some(&path), 0);
            }
        }

        ui.label(format!(
            "{:.2} transcoded FPS\t\
            {:.2} events per source sec\t\
            {:.2} events PPC per source sec\t\
            {:.0} events total\t\
            {:.0} events PPC total
            ",
            1. / time.delta_seconds(),
            self.ui_info_state.events_per_sec,
            self.ui_info_state.events_ppc_per_sec,
            self.ui_info_state.events_total,
            self.ui_info_state.events_ppc_total
        ));
    }

    pub fn update_adder_params(&mut self, mut commands: Commands) {
        // let ui_state = &mut self.ui_state;
        // let ui_state_mem = &mut self.ui_state_mem;
        // // First, check if the sliders have changed. If they have, don't do anything this frame.
        // if ui_state.delta_t_ref_slider != ui_state_mem.delta_t_ref_slider {
        //     ui_state_mem.delta_t_ref_slider = ui_state.delta_t_ref_slider;
        //     return;
        // }
        // if ui_state.delta_t_max_mult_slider != ui_state_mem.delta_t_max_mult_slider {
        //     ui_state_mem.delta_t_max_mult_slider = ui_state.delta_t_max_mult_slider;
        //     return;
        // }
        // if ui_state.adder_tresh_slider != ui_state_mem.adder_tresh_slider {
        //     ui_state_mem.adder_tresh_slider = ui_state.adder_tresh_slider;
        //     return;
        // }
        // if ui_state.scale_slider != ui_state_mem.scale_slider {
        //     ui_state_mem.scale_slider = ui_state.scale_slider;
        //     return;
        // }
        //
        // ui_state.delta_t_ref = ui_state.delta_t_ref_slider;
        // ui_state.delta_t_max_mult = ui_state.delta_t_max_mult_slider;
        // ui_state.adder_tresh = ui_state.adder_tresh_slider;
        // ui_state.scale = ui_state.scale_slider;

        // TODO: do conditionals on the sliders themselves
        let source: &mut dyn Source = {
            match &mut self.transcoder.framed_source {
                None => {
                    match &mut self.transcoder.davis_source {
                        None => {
                            return;
                        }
                        Some(source) => {
                            if source.mode != self.ui_state.davis_mode_radio_state
                                || source.get_reconstructor().output_fps
                                    != self.ui_state.davis_output_fps
                            {
                                let source_name = self.ui_info_state.source_name.clone();
                                replace_adder_transcoder(
                                    self,
                                    self.ui_info_state.input_path.clone(),
                                    None,   // TODO!!
                                    0,
                                );
                                return;
                            }
                            // let tmp = source.get_reconstructor();
                            let tmp = source.get_reconstructor_mut();
                            tmp.set_optimize_c(self.ui_state.optimize_c);
                            source
                        }
                    }
                }
                Some(source) => {
                    if source.scale != self.ui_state.scale
                        || source.get_ref_time() != self.ui_state.delta_t_ref as u32
                        || match source.get_video().channels {
                            1 => {
                                // True if the transcoder is gray, but the user wants color
                                self.ui_state.color
                            }
                            _ => {
                                // True if the transcoder is color, but the user wants gray
                                !self.ui_state.color
                            }
                        }
                    {
                        let source_name = self.ui_info_state.source_name.clone();
                        let current_frame =
                            source.get_video().in_interval_count + source.frame_idx_start;
                        replace_adder_transcoder(
                            self,
                            self.ui_info_state.input_path.clone(),
                            None, // TODO!!
                            current_frame,
                        );
                        return;
                    }
                    source
                }
            }
        };

        let video = source.get_video_mut();
        video.update_adder_thresh_pos(self.ui_state.adder_tresh as u8);
        video.update_adder_thresh_neg(self.ui_state.adder_tresh as u8);
        video.update_delta_t_max(self.ui_state.delta_t_max_mult * video.get_ref_time());
        video.instantaneous_view_mode = self.ui_state.view_mode_radio_state;
    }

    pub fn consume_source(
        &mut self,
        mut images: ResMut<Assets<Image>>,
        mut handles: ResMut<Images>,
        mut commands: Commands,
    ) {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(self.ui_state.thread_count)
            .build()
            .unwrap();

        let mut ui_info_state = &mut self.ui_info_state;
        ui_info_state.events_per_sec = 0.;

        let source: &mut dyn Source = {
            match &mut self.transcoder.framed_source {
                None => match &mut self.transcoder.davis_source {
                    None => {
                        return;
                    }
                    Some(source) => source,
                },
                Some(source) => source,
            }
        };
        match source.consume(1, &pool) {
            Ok(events_vec_vec) => {
                for events_vec in events_vec_vec {
                    ui_info_state.events_total += events_vec.len() as u64;
                    ui_info_state.events_per_sec += events_vec.len() as f64;
                }
                ui_info_state.events_ppc_total = ui_info_state.events_total as f64
                    / (source.get_video().width as f64
                        * source.get_video().height as f64
                        * source.get_video().channels as f64);
                let source_fps =
                    source.get_video().get_tps() as f64 / source.get_video().get_ref_time() as f64;
                ui_info_state.events_per_sec *= source_fps;
                ui_info_state.events_ppc_per_sec = ui_info_state.events_per_sec
                    / (source.get_video().width as f64
                        * source.get_video().height as f64
                        * source.get_video().channels as f64);
            }
            Err(SourceError::Open) => {}
            Err(_) => {
                // Start video over from the beginning
                let source_name = ui_info_state.source_name.clone();
                replace_adder_transcoder(
                    self,
                    self.ui_info_state.input_path.clone(),
                    None, // TODO!!
                    0,
                );
                return;
            }
        };

        let image_mat = &source.get_video().instantaneous_frame;

        // add alpha channel
        let mut image_mat_bgra = Mat::default();
        imgproc::cvt_color(&image_mat, &mut image_mat_bgra, imgproc::COLOR_BGR2BGRA, 4).unwrap();

        let image_bevy = Image::new(
            Extent3d {
                width: source.get_video().width.into(),
                height: source.get_video().height.into(),
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            Vec::from(image_mat_bgra.data_bytes().unwrap()),
            TextureFormat::Bgra8UnormSrgb,
        );
        self.transcoder.live_image = image_bevy;

        let handle = images.add(self.transcoder.live_image.clone());
        handles.image_view = handle;
    }
}

fn side_panel_grid_contents(
    transcoder: &AdderTranscoder,
    ui: &mut Ui,
    ui_state: &mut ParamsUiState,
) {
    let dtr_max = ui_state.delta_t_ref_max;
    let enabled = match transcoder.davis_source {
        None => true,
        Some(_) => false,
    };
    ui.add_enabled(enabled, egui::Label::new("Δt_ref:"));
    slider_pm(
        enabled,
        false,
        ui,
        &mut ui_state.delta_t_ref,
        &mut ui_state.delta_t_ref_slider,
        1.0..=dtr_max,
        vec![],
        10.0,
    );
    ui.end_row();

    ui.label("Δt_max multiplier:");
    slider_pm(
        true,
        false,
        ui,
        &mut ui_state.delta_t_max_mult,
        &mut ui_state.delta_t_max_mult_slider,
        2..=1000,
        vec![],
        10,
    );
    ui.end_row();

    ui.label("ADΔER threshold:");
    slider_pm(
        true,
        false,
        ui,
        &mut ui_state.adder_tresh,
        &mut ui_state.adder_tresh_slider,
        0.0..=255.0,
        vec![],
        1.0,
    );
    ui.end_row();

    ui.label("Thread count:");
    slider_pm(
        true,
        false,
        ui,
        &mut ui_state.thread_count,
        &mut ui_state.thread_count_slider,
        1..=(current_num_threads() - 1).max(4),
        vec![],
        1,
    );
    ui.end_row();

    ui.label("Video scale:");
    slider_pm(
        enabled,
        false,
        ui,
        &mut ui_state.scale,
        &mut ui_state.scale_slider,
        0.01..=1.0,
        vec![0.25, 0.5, 0.75],
        0.1,
    );
    ui.end_row();

    ui.label("Channels:");
    ui.add_enabled(enabled, egui::Checkbox::new(&mut ui_state.color, "Color?"));
    ui.end_row();

    ui.label("View mode:");
    ui.horizontal(|ui| {
        ui.radio_value(
            &mut ui_state.view_mode_radio_state,
            FramedViewMode::Intensity,
            "Intensity",
        );
        ui.radio_value(&mut ui_state.view_mode_radio_state, FramedViewMode::D, "D");
        ui.radio_value(
            &mut ui_state.view_mode_radio_state,
            FramedViewMode::DeltaT,
            "Δt",
        );
    });
    ui.end_row();

    ui.label("DAVIS mode:");
    ui.add_enabled_ui(!enabled, |ui| {
        ui.horizontal(|ui| {
            ui.radio_value(
                &mut ui_state.davis_mode_radio_state,
                DavisTranscoderMode::Framed,
                "Framed recon",
            );
            ui.radio_value(
                &mut ui_state.davis_mode_radio_state,
                DavisTranscoderMode::RawDavis,
                "Raw DAVIS",
            );
            ui.radio_value(
                &mut ui_state.davis_mode_radio_state,
                DavisTranscoderMode::RawDvs,
                "Raw DVS",
            );
        });
    });
    ui.end_row();

    ui.label("DAVIS deblurred FPS:");
    slider_pm(
        !enabled,
        false,
        ui,
        &mut ui_state.davis_output_fps,
        &mut ui_state.davis_output_fps_slider,
        1.0..=10000.0,
        vec![2500.0, 5000.0, 7500.0],
        50.0,
    );
    ui.end_row();

    ui.label("Optimize:");
    ui.add_enabled(
        !enabled,
        egui::Checkbox::new(&mut ui_state.optimize_c, "Optimize θ?"),
    );
    ui.end_row();
}
