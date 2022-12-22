use std::io;
use std::io::Write;
use adder_codec_rs::{Codec, SourceCamera};
use adder_codec_rs::framer::event_framer::Framer;
use adder_codec_rs::framer::scale_intensity::event_to_intensity;
use adder_codec_rs::raw::raw_stream::RawStream;
use bevy::asset::Assets;
use bevy::prelude::{Commands, Image, Res, ResMut};
use bevy::time::Time;
use bevy_egui::egui::{Ui};
use bevy::ecs::system::Resource;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::utils::tracing::event;
use bevy_egui::egui;
use opencv::core::{Mat, MatTraitConstManual, MatTraitManual};
use opencv::imgproc;
use crate::Images;
use crate::player::adder::AdderPlayer;

#[derive(Default)]
pub struct PlayerUiState {
    pub(crate) player: AdderPlayer,
    pub(crate) playback_speed: f32,
    pub(crate) playing: bool,
    pub(crate) current_frame: u32,
    pub(crate) total_frames: u32,
    pub(crate) current_time: f32,
    pub(crate) total_time: f32,
}

#[derive(Resource, Default)]
pub struct PlayerState {
    // pub(crate) transcoder: AdderTranscoder,
    pub ui_state: PlayerUiState,
    // pub(crate) ui_state_mem: UiStateMemory,
    // pub ui_info_state: InfoUiState,
}

impl PlayerState {
    // Fill in the side panel with sliders for playback speed and buttons for play/pause/stop
    pub fn side_panel_ui(
        &mut self,
        mut ui: &mut Ui,
    ) {
        ui.add(
            egui::Slider::new(&mut self.ui_state.playback_speed, 0.0..=10.0)
                .text("Playback speed"),
        );
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Play").clicked() {
                self.ui_state.playing = true;
            }
            if ui.button("Pause").clicked() {
                self.ui_state.playing = false;
            }
            if ui.button("Stop").clicked() {
                self.ui_state.playing = false;
                self.ui_state.current_frame = 0;
            }
        });



    }

    pub fn consume_source(
        &mut self,
        mut images: ResMut<Assets<Image>>,
        mut handles: ResMut<Images>,
        mut commands: Commands,
    ) {
        let stream = match &mut self.ui_state.player.input_stream {
            None => {
                return;
            }
            Some(s) => { s }
        };

        let frame_sequence = match &mut self.ui_state.player.frame_sequence {
            None => {
                return;
            }
            Some(s) => { s }
        };

        let frame_length = stream.tps as f64 / 30.0;    //TODO: temp
        {
        let display_mat = &mut self.ui_state.player.display_mat;

        loop {
            // if event_count % divisor == 0 {
            //     write!(
            //         handle,
            //         "\rPlaying back ADΔER file...{}%",
            //         (event_count * 100) / num_events as u64
            //     )?;
            //     handle.flush().unwrap();
            // }
            if self.ui_state.player.current_t as u128 > (self.ui_state.current_frame as u128 * frame_length as u128) {
                self.ui_state.current_frame += 1;
                break
                // let wait_time = max(
                //     ((1000.0 / args.playback_fps) as u128)
                //         .saturating_sub((Instant::now() - last_frame_displayed_ts).as_millis()),
                //     1,
                // ) as i32;
            }

            match stream.decode_event() {
                Ok(event) if event.d <= 0xFE => {
                    // event_count += 1;
                    let y = event.coord.y as i32;
                    let x = event.coord.x as i32;
                    let c = event.coord.c.unwrap_or(0) as i32;
                    if (y | x | c) == 0x0 {
                        self.ui_state.player.current_t += event.delta_t;
                    }

                    let frame_intensity = (event_to_intensity(&event) * stream.ref_interval as f64)
                        / match stream.source_camera {
                        SourceCamera::FramedU8 => u8::MAX as f64,
                        SourceCamera::FramedU16 => u16::MAX as f64,
                        SourceCamera::FramedU32 => u32::MAX as f64,
                        SourceCamera::FramedU64 => u64::MAX as f64,
                        SourceCamera::FramedF32 => {
                            todo!("Not yet implemented")
                        }
                        SourceCamera::FramedF64 => {
                            todo!("Not yet implemented")
                        }
                        SourceCamera::Dvs => u8::MAX as f64,
                        SourceCamera::DavisU8 => u8::MAX as f64,
                        SourceCamera::Atis => {
                            todo!("Not yet implemented")
                        }
                        SourceCamera::Asint => {
                            todo!("Not yet implemented")
                        }
                    } * 255.0;
                    unsafe {
                        let px: &mut u8 = display_mat.at_3d_unchecked_mut(y, x, c).unwrap();
                        *px = frame_intensity as u8;
                    }
                }
                Err(e) => {
                    // TODO: add loop toggle button
                    stream.set_input_stream_position(stream.header_size as u64).unwrap();
                    break;
                }
                _ => {}
            }
        }
    }

        let mut image_mat_bgra = Mat::default();
        imgproc::cvt_color(&self.ui_state.player.display_mat, &mut image_mat_bgra, imgproc::COLOR_BGR2BGRA, 4).unwrap();


        // TODO: refactor
        let image_bevy = Image::new(
            Extent3d {
                width: stream.width.into(),
                height: stream.height.into(),
                depth_or_array_layers: 1,
            },

            TextureDimension::D2,
            Vec::from(image_mat_bgra.data_bytes().unwrap()),
            TextureFormat::Bgra8UnormSrgb);
        self.ui_state.player.live_image = image_bevy;


        let handle = images.add(self.ui_state.player.live_image.clone());
        handles.image_view = handle;

        // last_frame_displayed_ts = Instant::now();
        // frame_count += 1;





        // let image_mat = &source.get_video().instantaneous_frame;
        //
        // // add alpha channel
        // let mut image_mat_bgra = Mat::default();
        // imgproc::cvt_color(&image_mat, &mut image_mat_bgra, imgproc::COLOR_BGR2BGRA, 4).unwrap();
        //
        // let image_bevy = Image::new(
        //     Extent3d {
        //         width: source.get_video().width.into(),
        //         height: source.get_video().height.into(),
        //         depth_or_array_layers: 1,
        //     },
        //
        //     TextureDimension::D2,
        //     Vec::from(image_mat_bgra.data_bytes().unwrap()),
        //     TextureFormat::Bgra8UnormSrgb);
        // self.transcoder.live_image = image_bevy;
        //
        //
        // let handle = images.add(self.transcoder.live_image.clone());
        // handles.image_view = handle;
    }

    pub fn central_panel_ui(
        &mut self,
        mut ui: &mut Ui,
        time: Res<Time>
    ) {

        ui.heading("Drag and drop your ADΔER file here (.adder)");
    }

    pub fn replace_player(&mut self, path_buf: &std::path::PathBuf) {
        self.ui_state.player = AdderPlayer::new(path_buf).unwrap();
        self.ui_state.current_frame = 1;

    }
}