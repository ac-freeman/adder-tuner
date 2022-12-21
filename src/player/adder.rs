use std::error::Error;
use std::fmt;
use std::path::PathBuf;
use adder_codec_rs::Codec;
use adder_codec_rs::framer::event_framer::{Framer, FramerBuilder, FrameSequence};
use adder_codec_rs::framer::event_framer::FramerMode::INSTANTANEOUS;
use adder_codec_rs::raw::raw_stream::RawStream;
use bevy::prelude::Image;
use crate::player::ui::PlayerUiState;

#[derive(Default)]
pub struct AdderPlayer {
    pub(crate) frame_sequence: Option<FrameSequence<u8>>,
    pub(crate) input_stream: Option<RawStream>,
    pub(crate) live_image: Image,
}

#[derive(Debug)]
struct AdderPlayerError(String);

impl fmt::Display for AdderPlayerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ADDER player: {}", self.0)
    }
}

impl Error for AdderPlayerError {}

impl AdderPlayer {
    pub(crate) fn new(path_buf: &PathBuf) -> Result<Self, Box<dyn Error>> {
        match path_buf.extension() {
            None => {
                Err(Box::new(AdderPlayerError("Invalid file type".into())))
            }
            Some(ext) => {
                match ext.to_str() {
                    None => {Err(Box::new(AdderPlayerError("Invalid file type".into())))}
                    Some("adder") => {
                        let input_path = path_buf.to_str().unwrap().to_string();
                        let mut stream: RawStream = Codec::new();
                        stream.open_reader(input_path).expect("Invalid path");
                        stream.decode_header().expect("Invalid header");

                        let reconstructed_frame_rate = (stream.tps / stream.ref_interval) as f64;
                        println!("reconstructed_frame_rate: {}", reconstructed_frame_rate);
                        // For instantaneous reconstruction, make sure the frame rate matches the source video rate
                        assert_eq!(
                            stream.tps / stream.ref_interval,
                            reconstructed_frame_rate as u32
                        );

                        let mut frame_sequence: FrameSequence<u8> = FramerBuilder::new(
                            stream.height.into(),
                            stream.width.into(),
                            stream.channels.into(),
                            260,
                        )
                            .codec_version(stream.codec_version)
                            .time_parameters(stream.tps, stream.ref_interval, reconstructed_frame_rate)
                            .mode(INSTANTANEOUS)
                            .source(stream.get_source_type(), stream.source_camera)
                            .finish();

                        // ui_state.total_frames = frame_sequence.frame_count;

                        println!("Creating adder player");
                        Ok(AdderPlayer {
                            frame_sequence: Some(frame_sequence),
                            input_stream: Some(stream),
                            live_image: Default::default(),
                        })

                        // match FrameSequence::new(path_buf.to_str().unwrap().to_string(), current_frame) {
                        //     Ok(frame_sequence) => {
                        //         let mut adder_player = AdderPlayer {
                        //             frame_sequence: Some(frame_sequence),
                        //             input_stream: None,
                        //             live_image: Image::new_empty(),
                        //         };
                        //         adder_player.update_image(ui_state);
                        //         Ok(adder_player)
                        //     }
                        //     Err(e) => {
                        //         Err(Box::new(AdderPlayerError(format!("Error opening file: {}", e))))
                        //     }
                        // }
                    }
                    Some(_) => {Err(Box::new(AdderPlayerError("Invalid file type".into())))}
                }
            }
        }
    }
}