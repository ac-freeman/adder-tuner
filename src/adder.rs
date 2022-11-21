use std::error::Error;
use std::ffi::OsStr;
use std::fmt;
use std::path::PathBuf;
use bevy::prelude::{Commands, Query, ResMut, Resource};
use adder_codec_rs::transcoder::source::framed_source::FramedSource;
use adder_codec_rs::transcoder::source::davis_source::DavisSource;
use adder_codec_rs::SourceCamera;
use adder_codec_rs::transcoder::source::framed_source::FramedSourceBuilder;
use adder_codec_rs::transcoder::source::video::Source;
use crate::{replace_adder_transcoder, UiState};


#[derive(Resource, Default)]
pub struct AdderTranscoder {
    framed_source: Option<FramedSource>,
    davis_source: Option<DavisSource>,
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
                                })
                            }
                            Err(e) => {
                                Err(Box::new(AdderTranscoderError("Invalid file type".into())))
                            }
                        }


                    }
                    Some("aedat4") => {
                        Ok(AdderTranscoder {
                            framed_source: None,
                            davis_source: None,
                        })
                    }
                    Some(_) => {Err(Box::new(AdderTranscoderError("Invalid file type".into())))}
                }
            }
        }
    }
}

pub(crate) fn consume_source( mut ui_state: ResMut<UiState>,
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
                    // commands.remove_resource::<AdderTranscoder>();
                    // commands.insert_resource
                    // (
                    //     transcoder
                    // );
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
}