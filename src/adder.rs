use std::error::Error;
use std::ffi::OsStr;
use std::fmt;
use std::path::PathBuf;
use bevy::prelude::{Commands, Image, NodeBundle, Query, ResMut, Resource};
use adder_codec_rs::transcoder::source::framed_source::FramedSource;
use adder_codec_rs::transcoder::source::davis_source::DavisSource;
use adder_codec_rs::{Event, SourceCamera};
use adder_codec_rs::transcoder::source::framed_source::FramedSourceBuilder;
use adder_codec_rs::transcoder::source::video::{Source, SourceError};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy_egui::egui::TextureFilter;
use bevy_egui::EguiContext;
use crate::{Images, replace_adder_transcoder, UiState, UiStateMemory};
use opencv::core::{CV_32FC3, CV_32FC4, Mat};
use opencv::videoio::{VideoCapture, CAP_PROP_FPS, CAP_PROP_FRAME_COUNT, CAP_PROP_POS_FRAMES};
use opencv::{imgproc, prelude::*, videoio, Result, highgui};
use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
    winit::WinitSettings,
};


#[derive(Resource, Default)]
pub struct AdderTranscoder {
    framed_source: Option<FramedSource>,
    davis_source: Option<DavisSource>,
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
    pub(crate) fn new(path_buf: &PathBuf, mut ui_state: &mut ResMut<UiState>, current_frame: u32) -> Result<Self, Box<dyn Error>> {
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
                            .color(true)
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
                            Err(e) => {
                                Err(Box::new(AdderTranscoderError("Invalid file type".into())))
                            }
                        }


                    }
                    Some("aedat4") => {
                        todo!();
                        Ok(AdderTranscoder {
                            framed_source: None,
                            davis_source: None,
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
    mut images: ResMut<Assets<Image>>,
    mut handles: ResMut<Images>,
    mut egui_ctx: ResMut<EguiContext>,
    mut ui_state: ResMut<UiState>,
    mut ui_state_mem: ResMut<UiStateMemory>,
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



    let mut source: &mut dyn Source = {

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
                if source.scale != ui_state.scale
                    || source.get_ref_time() != ui_state.delta_t_ref as u32 {
                    let source_name = ui_state.source_name.clone();
                    let current_frame = source.get_video().in_interval_count + source.frame_idx_start;
                    replace_adder_transcoder(&mut commands, &mut ui_state, &PathBuf::from(source_name.text()), current_frame);
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


}

pub(crate) fn consume_source(
    mut images: ResMut<Assets<Image>>,
    mut handles: ResMut<Images>,
    mut egui_ctx: ResMut<EguiContext>,
    mut ui_state: ResMut<UiState>,
    mut commands: Commands,
    mut transcoder: ResMut<AdderTranscoder>) {

    let mut source: &mut dyn Source = {

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
                // if source.scale != ui_state.scale {
                //     let source_name = ui_state.source_name.clone();
                //     let current_frame = source.get_video().in_interval_count + source.frame_idx_start;
                //     replace_adder_transcoder(&mut commands, &mut ui_state, &PathBuf::from(source_name.text()), current_frame);
                //     return;
                // }
                source
            }
        }
    };


    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(ui_state.thread_count)
        .build()
        .unwrap();

    ui_state.events_per_sec = 0.;
    match source.consume(1, &pool) {
        Ok(events_vec_vec) => {
            for events_vec in events_vec_vec {
                ui_state.events_total += events_vec.len() as u64;
                ui_state.events_per_sec += events_vec.len() as f64;
            }
            ui_state.events_ppc_total = ui_state.events_total as u64 / (source.get_video().width as u64 * source.get_video().height as u64);
            let source_fps = source.get_video().get_delta_t_max() as f64 / source.get_video().get_ref_time() as f64;
            ui_state.events_per_sec = ui_state.events_per_sec  as f64 * source_fps;
            ui_state.events_ppc_per_sec = ui_state.events_per_sec / (source.get_video().width as f64 * source.get_video().height as f64);
        }
        Err(SourceError::Open) => {

        }
        Err(_) => {
            // Start video over from the beginning
            let source_name = ui_state.source_name.clone();
            replace_adder_transcoder(&mut commands, &mut ui_state, &PathBuf::from(source_name.text()), 0);
            return;
        }
    };

    let image_mat = &source.get_video().instantaneous_frame;

    // add alpha channel
    let mut image_mat_bgra = Mat::default();
    imgproc::cvt_color(&image_mat, &mut image_mat_bgra, imgproc::COLOR_BGR2BGRA, 4).unwrap();
    // let mut image_mat_rgba_32f = Mat::default();
    // Mat::convert_to(&image_mat_rgba, &mut image_mat_rgba_32f, CV_32FC4, 1.0/255.0, 0.0).unwrap();
    // highgui::imshow("tmp", &image_mat_rgba_32f).unwrap();
    // highgui::wait_key(1).unwrap();

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


    // images.remove(&handles.image_view);

    let handle = images.add(transcoder.live_image.clone());
    handles.image_view = handle;


    // let egui_texture_handle = ui_state
    //     .egui_texture_handle
    //     .get_or_insert_with(|| {
    //
    //         egui_ctx.ctx_mut().load_texture(
    //             "example-image",
    //             transcoder.live_image.clone().data,
    //             TextureFilter::Nearest,
    //         )
    //     })
    //     .clone();


    // egui_ctx.add_image(egui_texture_handle)
}