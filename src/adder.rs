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
use crate::{Images, replace_adder_transcoder, ParamsUiState, UiStateMemory, InfoUiState};
use opencv::core::{Mat};

use opencv::{imgproc, prelude::*, Result};
use bevy::{
    prelude::*,
};


#[derive(Resource, Default)]
pub struct AdderTranscoder {
    pub(crate) framed_source: Option<FramedSource>,
    pub(crate) davis_source: Option<DavisSource>,
    live_image: Image,
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
    pub(crate) fn new(path_buf: &PathBuf, ui_state: &mut ResMut<ParamsUiState>, current_frame: u32) -> Result<Self, Box<dyn Error>> {
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
                            true,
                            false,
                            false,
                            false,
                            500.0,
                            Compression::None,
                            346,
                            260,
                            deblur_only,
                            events_only,
                            1000.0,
                            true,
                        ));

                        let mut davis_source = DavisSource::new(
                            reconstructor,
                            None,   // TODO
                            (1000000) as u32, // TODO
                            1000000.0 / 500.0,
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
    mut ui_state: ResMut<ParamsUiState>,
    mut ui_state_mem: ResMut<UiStateMemory>,
    mut ui_info_state: ResMut<InfoUiState>,
    mut commands: Commands,
    mut transcoder: ResMut<AdderTranscoder>) {
    // First, check if the sliders have changed. If they have, don't do anything this frame.
    if ui_state.delta_t_ref_slider != ui_state_mem.delta_t_ref_slider {
        ui_state_mem.delta_t_ref_slider = ui_state.delta_t_ref_slider;
        return;
    }
    if ui_state.delta_t_max_mult_slider != ui_state_mem.delta_t_max_mult_slider {
        ui_state_mem.delta_t_max_mult_slider = ui_state.delta_t_max_mult_slider;
        return;
    }
    if ui_state.adder_tresh_slider != ui_state_mem.adder_tresh_slider {
        ui_state_mem.adder_tresh_slider = ui_state.adder_tresh_slider;
        return;
    }
    if ui_state.scale_slider != ui_state_mem.scale_slider {
        ui_state_mem.scale_slider = ui_state.scale_slider;
        return;
    }

    ui_state.delta_t_ref = ui_state.delta_t_ref_slider;
    ui_state.delta_t_max_mult = ui_state.delta_t_max_mult_slider;
    ui_state.adder_tresh = ui_state.adder_tresh_slider;
    ui_state.scale = ui_state.scale_slider;



    let source: &mut dyn Source = {

        match &mut transcoder.framed_source {
            None => {
                match &mut transcoder.davis_source {
                    None => { return; }
                    Some(source) => {
                        if source.mode != ui_state.davis_mode_radio_state
                        {
                            let source_name = ui_info_state.source_name.clone();
                            replace_adder_transcoder(&mut commands, &mut ui_state, &mut ui_info_state, &PathBuf::from(source_name.text()), 0);
                            return;
                        }
                        source
                    }
                }
            }
            Some(source) => {
                if source.scale != ui_state.scale
                    || source.get_ref_time() != ui_state.delta_t_ref as u32
                    || match source.get_video().channels {
                            1 => {
                                // True if the transcoder is gray, but the user wants color
                                ui_state.color
                            }
                            _ => {
                                // True if the transcoder is color, but the user wants gray
                                !ui_state.color
                            }
                        }
                {
                    let source_name = ui_info_state.source_name.clone();
                    let current_frame = source.get_video().in_interval_count + source.frame_idx_start;
                    replace_adder_transcoder(&mut commands, &mut ui_state, &mut ui_info_state, &PathBuf::from(source_name.text()), current_frame);
                    return;
                }
                source
            }
        }
    };

    let video = source.get_video_mut();
    video.update_adder_thresh_pos(ui_state.adder_tresh as u8);
    video.update_adder_thresh_neg(ui_state.adder_tresh as u8);
    video.update_delta_t_max(ui_state.delta_t_max_mult as u32 * video.get_ref_time());
    video.instantaneous_view_mode = ui_state.view_mode_radio_state;


}

pub(crate) fn consume_source(
    mut images: ResMut<Assets<Image>>,
    mut handles: ResMut<Images>,
    _egui_ctx: ResMut<EguiContext>,
    mut ui_state: ResMut<ParamsUiState>,
    mut ui_info_state: ResMut<InfoUiState>,
    mut commands: Commands,
    mut transcoder: ResMut<AdderTranscoder>) {

    let source: &mut dyn Source = {

        match &mut transcoder.framed_source {
            None => {
                match &mut transcoder.davis_source {
                    None => { return; }
                    Some(source) => {
                        source
                    }
                }
            }
            Some(source) => {
                source
            }
        }
    };


    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(ui_state.thread_count)
        .build()
        .unwrap();

    ui_info_state.events_per_sec = 0.;
    match source.consume(1, &pool) {
        Ok(events_vec_vec) => {
            for events_vec in events_vec_vec {
                ui_info_state.events_total += events_vec.len() as u64;
                ui_info_state.events_per_sec += events_vec.len() as f64;
            }
            ui_info_state.events_ppc_total = ui_info_state.events_total as u64 / (source.get_video().width as u64 * source.get_video().height as u64 * source.get_video().channels as u64);
            let source_fps = source.get_video().get_tps() as f64 / source.get_video().get_ref_time() as f64;
            ui_info_state.events_per_sec = ui_info_state.events_per_sec  as f64 * source_fps;
            ui_info_state.events_ppc_per_sec = ui_info_state.events_per_sec / (source.get_video().width as f64 * source.get_video().height as f64 * source.get_video().channels as f64);
        }
        Err(SourceError::Open) => {

        }
        Err(_) => {
            // Start video over from the beginning
            let source_name = ui_info_state.source_name.clone();
            replace_adder_transcoder(&mut commands, &mut ui_state, &mut ui_info_state, &PathBuf::from(source_name.text()), 0);
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
        TextureFormat::Bgra8UnormSrgb);
    transcoder.live_image = image_bevy;


    let handle = images.add(transcoder.live_image.clone());
    handles.image_view = handle;
}