use std::error::Error;

use std::fmt;
use std::path::PathBuf;
use bevy::prelude::{Commands, Image, ResMut, Resource};
use adder_codec_rs::transcoder::source::framed_source::FramedSource;
use adder_codec_rs::transcoder::source::davis_source::DavisSource;
use adder_codec_rs::{SourceCamera};
use adder_codec_rs::transcoder::source::framed_source::FramedSourceBuilder;
use adder_codec_rs::transcoder::source::video::{Source, SourceError};
use adder_codec_rs::transcoder::source::davis_source::DavisTranscoderMode;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use adder_codec_rs::davis_edi_rs::util::reconstructor::Reconstructor;
use adder_codec_rs::aedat::base::ioheader_generated::Compression;

use bevy_egui::EguiContext;
use crate::{Images,};
use opencv::core::{Mat};

use opencv::{imgproc, prelude::*, Result};
use bevy::{
    prelude::*,
};
use bevy_egui::egui::{Color32, RichText};
use crate::transcoder::ui::{InfoUiState, ParamsUiState, TranscoderState, UiStateMemory};


#[derive(Default)]
pub struct AdderTranscoder {
    pub(crate) framed_source: Option<FramedSource>,
    pub(crate) davis_source: Option<DavisSource>,
    pub(crate) live_image: Image,
}

#[derive(Debug)]
struct AdderTranscoderError(String);

impl fmt::Display for AdderTranscoderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ADDER transcoder: {}", self.0)
    }
}

impl Error for AdderTranscoderError {}

impl AdderTranscoder {
    pub(crate) fn new(path_buf: &PathBuf, ui_state: &mut ParamsUiState, current_frame: u32) -> Result<Self, Box<dyn Error>> {
        match path_buf.extension() {
            None => {
                Err(Box::new(AdderTranscoderError("Invalid file type".into())))
            }
            Some(ext) => {
                match ext.to_str() {
                    None => {Err(Box::new(AdderTranscoderError("Invalid file type".into())))}
                    Some("mp4") => {
                        match FramedSourceBuilder::new(
                            path_buf.to_str().unwrap().to_string(),
                            SourceCamera::FramedU8,
                        )
                            .frame_start(current_frame)
                            .chunk_rows(64)
                            .scale(ui_state.scale)
                            .communicate_events(true)
                            // .output_events_filename("/home/andrew/Downloads/events.adder".to_string())
                            .color(ui_state.color)
                            .contrast_thresholds(ui_state.adder_tresh as u8, ui_state.adder_tresh as u8)
                            .show_display(false)
                            .time_parameters(ui_state.delta_t_ref as u32, ui_state.delta_t_max_mult * ui_state.delta_t_ref as u32 )
                            .finish() {
                            Ok(source) => {
                                ui_state.delta_t_ref_max = 255.0;
                                Ok(AdderTranscoder {
                                    framed_source: Some(source),
                                    davis_source: None,
                                    live_image: Default::default(),
                                })
                            }
                            Err(_e) => {
                                Err(Box::new(AdderTranscoderError("Invalid file type".into())))
                            }
                        }


                    }
                    Some("aedat4") => {

                        let events_only = match &ui_state.davis_mode_radio_state {
                            DavisTranscoderMode::Framed => {false}
                            DavisTranscoderMode::RawDavis => {false}
                            DavisTranscoderMode::RawDvs => {true}
                        };
                        let deblur_only = match &ui_state.davis_mode_radio_state {
                            DavisTranscoderMode::Framed => false,
                            DavisTranscoderMode::RawDavis => true,
                            DavisTranscoderMode::RawDvs => true,
                        };

                        let rt = tokio::runtime::Builder::new_multi_thread()
                            .worker_threads(ui_state.thread_count)
                            .enable_time()
                            .build()
                            .unwrap();
                        let dir = path_buf.parent().expect("File must be in some directory")
                            .to_str().expect("Bad path").to_string();
                        let filename = path_buf.file_name().expect("File must exist")
                            .to_str().expect("Bad filename").to_string();
                        eprintln!("{}", filename);
                        let reconstructor = rt.block_on(Reconstructor::new(
                            dir + "/",
                            filename,
                            "".to_string(),
                            "file".to_string(), // TODO
                            0.15,
                            ui_state.optimize_c,
                            false,
                            false,
                            false,
                            ui_state.davis_output_fps,
                            Compression::None,
                            346,
                            260,
                            deblur_only,
                            events_only,
                            1000.0, // Target latency (not used)
                            true,
                        ));

                        let mut davis_source = DavisSource::new(
                            reconstructor,
                            None,   // TODO
                            (1000000) as u32, // TODO
                            1000000.0 / ui_state.davis_output_fps,
                            (1000000.0 * ui_state.delta_t_max_mult as f32) as u32, // TODO
                            false,
                            ui_state.adder_tresh as u8,
                            ui_state.adder_tresh as u8,
                            false,
                            rt,
                            ui_state.davis_mode_radio_state,
                            false,
                        )
                            .unwrap();

                        Ok(AdderTranscoder {
                            framed_source: None,
                            davis_source: Some(davis_source),
                            live_image: Default::default(),
                        })


                    }
                    Some(_) => {Err(Box::new(AdderTranscoderError("Invalid file type".into())))}
                }
            }
        }
    }
}

pub(crate) fn update_adder_params(
    _images: ResMut<Assets<Image>>,
    _handles: ResMut<Images>,
    _egui_ctx: ResMut<EguiContext>,
    mut transcoder_state: ResMut<TranscoderState>,
    mut commands: Commands,
    ) {

}

pub(crate) fn consume_source(
    mut images: ResMut<Assets<Image>>,
    mut handles: ResMut<Images>,
    _egui_ctx: ResMut<EguiContext>,
    mut transcoder_state: ResMut<TranscoderState>,
    mut commands: Commands,
) {
    transcoder_state.consume_source(images, handles, commands);




}

pub(crate) fn replace_adder_transcoder(commands: &mut Commands,
                                       transcoder_state: &mut TranscoderState,
                                       path_buf: &std::path::PathBuf,
                                       current_frame: u32) {
    let mut ui_info_state = &mut transcoder_state.ui_info_state;
    ui_info_state.events_per_sec = 0.0;
    ui_info_state.events_ppc_total = 0;
    ui_info_state.events_total = 0;
    ui_info_state.events_ppc_per_sec = 0.0;
    match AdderTranscoder::new(path_buf, &mut transcoder_state.ui_state, current_frame) {
        Ok(transcoder) => {
            transcoder_state.transcoder = transcoder;
            ui_info_state.source_name = RichText::new(path_buf.to_str().unwrap()).color(Color32::DARK_GREEN);

        }
        Err(e) => {
            transcoder_state.transcoder = AdderTranscoder::default();
            ui_info_state.source_name = RichText::new(e.to_string()).color(Color32::RED);
        }
    };
}