use std::error::Error;
use std::ffi::OsStr;
use std::fmt;
use std::path::PathBuf;
use bevy::prelude::{Commands, Image, NodeBundle, Query, ResMut, Resource};
use adder_codec_rs::transcoder::source::framed_source::FramedSource;
use adder_codec_rs::transcoder::source::davis_source::DavisSource;
use adder_codec_rs::SourceCamera;
use adder_codec_rs::transcoder::source::framed_source::FramedSourceBuilder;
use adder_codec_rs::transcoder::source::video::Source;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy_egui::egui::TextureFilter;
use bevy_egui::EguiContext;
use crate::{Images, replace_adder_transcoder, UiState};
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
    pub(crate) fn new(path_buf: &PathBuf, ui_state: &ResMut<UiState>, current_frame: u32) -> Result<Self, Box<dyn Error>> {
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
                            .show_display(true)
                            .time_parameters(ui_state.delta_t_ref as u32, ui_state.delta_t_max as u32)
                            .finish() {
                            Ok(source) => {
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
                if source.scale != ui_state.scale {
                    let source_name = ui_state.source_name.clone();
                    let current_frame = source.get_video().in_interval_count + source.frame_idx_start;
                    replace_adder_transcoder(&mut commands, &mut ui_state, &PathBuf::from(source_name.text()), current_frame);
                    return;
                }
                source
            }
        }
    };


    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();

    source.consume(1, &pool).unwrap();

    let image_mat = &source.get_video().instantaneous_frame;

    // convert bgr u8 image to rgba u8 image
    let mut image_mat_rgba = Mat::default();
    imgproc::cvt_color(&image_mat, &mut image_mat_rgba, imgproc::COLOR_BGR2RGBA, 4).unwrap();
    // let mut image_mat_rgb_32f = Mat::default();
    // Mat::convert_to(&image_mat_rgba, &mut image_mat_rgb_32f, CV_32FC3, 1.0/255.0, 0.0).unwrap();
    let mut image_mat_rgba_32f = Mat::default();
    Mat::convert_to(&image_mat_rgba, &mut image_mat_rgba_32f, CV_32FC4, 1.0/255.0, 0.0).unwrap();
    highgui::imshow("tmp", &image_mat_rgba_32f).unwrap();
    highgui::wait_key(1).unwrap();

    let image_bevy = Image::new(
        Extent3d {
            width: source.get_video().width.into(),
            height: source.get_video().height.into(),
            depth_or_array_layers: 1,
        },

        TextureDimension::D2,
        Vec::from(image_mat_rgba_32f.data_bytes().unwrap()),
        TextureFormat::Rgba32Float);
    transcoder.live_image = image_bevy;


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