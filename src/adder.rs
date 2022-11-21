use std::error::Error;
use std::ffi::OsStr;
use std::fmt;
use std::path::PathBuf;
use bevy::prelude::{ResMut, Resource};
use adder_codec_rs::transcoder::source::framed_source::FramedSource;
use adder_codec_rs::transcoder::source::davis_source::DavisSource;
use adder_codec_rs::SourceCamera;
use adder_codec_rs::transcoder::source::framed_source::FramedSourceBuilder;
use crate::UiState;


#[derive(Resource)]
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
    pub(crate) fn new(path_buf: &PathBuf, ui_state: &ResMut<UiState>) -> Result<Self, Box<dyn Error>> {
        match path_buf.extension() {
            None => {
                Err(Box::new(AdderTranscoderError("Invalid file type".into())))
            }
            Some(ext) => {
                match ext.to_str() {
                    None => {Err(Box::new(AdderTranscoderError("Invalid file type".into())))}
                    Some("mp4") => {
                        match FramedSourceBuilder::new(
                            "/media/andrew/ExternalM2/LAS/GH010017.mp4".to_string(),
                            SourceCamera::FramedU8,
                        )
                            .frame_start(0)
                            .scale(1.0)
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

// trait AdderTranscoder {
//     fn new() ->
// }