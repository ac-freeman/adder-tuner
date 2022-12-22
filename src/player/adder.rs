use std::error::Error;
use std::fmt;
use std::path::PathBuf;
use adder_codec_rs::{Codec, DeltaT};
use adder_codec_rs::framer::event_framer::{Framer, FramerBuilder, FrameSequence};
use adder_codec_rs::framer::event_framer::FramerMode::INSTANTANEOUS;
use adder_codec_rs::raw::raw_stream::RawStream;
use bevy::prelude::Image;
use opencv::core::{create_continuous, CV_64F, CV_64FC3, CV_8UC1, CV_8UC3, Mat};
use crate::player::ui::PlayerUiState;

#[derive(Default)]
pub struct AdderPlayer {
    pub(crate) framer_builder: Option<FramerBuilder>,
    pub(crate) frame_sequence: Option<FrameSequence<u8>>,   // TODO: remove this
    pub(crate) input_stream: Option<RawStream>,
    pub(crate) current_t_ticks: DeltaT,
    pub(crate) display_mat: Mat,
    pub(crate) live_image: Image,
}

unsafe impl Sync for AdderPlayer {}

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
                    None => {
                        Err(Box::new(AdderPlayerError("Invalid file type".into())))
                    }
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

                        let framer_builder: FramerBuilder = FramerBuilder::new(
                            stream.height.into(),
                            stream.width.into(),
                            stream.channels.into(),
                            260,
                        )
                            .codec_version(stream.codec_version)
                            .time_parameters(stream.tps, stream.ref_interval, reconstructed_frame_rate)
                            .mode(INSTANTANEOUS)
                            .source(stream.get_source_type(), stream.source_camera);

                        let mut frame_sequence: FrameSequence<u8> = framer_builder
                            .clone()
                            .finish();



                        let mut display_mat = Mat::default();
                        match stream.channels {
                            1 => {
                                println!("1 channel!");
                                create_continuous(
                                    stream.height as i32,
                                    stream.width as i32,
                                    CV_8UC1,
                                    &mut display_mat,
                                )
                                    .unwrap();
                            }
                            3 => {
                                println!("3 channel!");
                                create_continuous(
                                    stream.height as i32,
                                    stream.width as i32,
                                    CV_8UC3,
                                    &mut display_mat,
                                )
                                    .unwrap();
                            }
                            _ => {
                                return Err(Box::new(AdderPlayerError("Bad number of channels".into())));
                            }
                        }

                        println!("Creating adder player");
                        Ok(AdderPlayer {
                            framer_builder: Some(framer_builder),
                            frame_sequence: Some(frame_sequence),
                            input_stream: Some(stream),
                            current_t_ticks: 0,
                            live_image: Default::default(),
                            display_mat,
                        })
                    }
                    Some(_) => {Err(Box::new(AdderPlayerError("Invalid file type".into())))}
                }
            }
        }
    }
}