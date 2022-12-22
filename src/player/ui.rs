use std::io;
use std::io::Write;
use std::time::Duration;
use adder_codec_rs::{Codec, SourceCamera};
use adder_codec_rs::framer::event_framer::Framer;
use adder_codec_rs::framer::scale_intensity::event_to_intensity;
use adder_codec_rs::raw::raw_stream::{RawStream, StreamError};
use adder_codec_rs::transcoder::source::video::InstantaneousViewMode;
use bevy::asset::Assets;
use bevy::prelude::{Commands, Image, Res, ResMut};
use bevy::time::Time;
use bevy_egui::egui::{RichText, Ui};
use bevy::ecs::system::Resource;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::utils::tracing::{enabled, event};
use bevy_egui::egui;
use opencv::core::{Mat, MatTrait, MatTraitConstManual, MatTraitManual};
use opencv::imgproc;
use rayon::current_num_threads;
use crate::{add_checkbox_row, add_radio_row, add_slider_row, Images, slider_pm};
use crate::player::adder::AdderPlayer;
use crate::Tabs::Player;

#[derive(PartialEq)]
pub struct PlayerUiSliders {
    pub(crate) playback_speed: f32,
    pub(crate) thread_count: usize,
}

impl Default for PlayerUiSliders {
    fn default() -> Self {
        Self {
            playback_speed: 1.0,
            thread_count: 4,
        }
    }
}

#[derive(PartialEq, Clone)]
enum ReconstructionMethod {
    Fast,
    Accurate,
}

pub struct PlayerUiState {
    pub(crate) playing: bool,
    pub(crate) looping: bool,
    pub(crate) view_mode: InstantaneousViewMode,
    reconstruction_method: ReconstructionMethod,
    pub(crate) current_frame: u32,
    pub(crate) total_frames: u32,
    pub(crate) current_time: f32,
    pub(crate) total_time: f32,
}

impl Default for PlayerUiState {
    fn default() -> Self {
        Self {
            playing: true,
            looping: true,
            view_mode: InstantaneousViewMode::Intensity,
            reconstruction_method: ReconstructionMethod::Accurate,
            current_frame: 0,
            total_frames: 0,
            current_time: 0.0,
            total_time: 0.0,
        }
    }
}

pub struct InfoUiState {
    pub events_per_sec: f64,
    pub events_ppc_per_sec: f64,
    pub events_ppc_total: f64,
    pub events_total: u64,
    pub source_name: RichText,
}

impl Default for InfoUiState {
    fn default() -> Self {
        InfoUiState {
            events_per_sec: 0.,
            events_ppc_per_sec: 0.,
            events_ppc_total: 0.0,
            events_total: 0,
            source_name: RichText::new("No file selected yet"),
        }
    }
}

impl InfoUiState {
    fn clear_stats (&mut self) {
        self.events_per_sec = 0.;
        self.events_ppc_per_sec = 0.;
        self.events_ppc_total = 0.0;
        self.events_total = 0;
    }
}

#[derive(Resource, Default)]
pub struct PlayerState {
    pub(crate) player: AdderPlayer,
    pub ui_state: PlayerUiState,
    pub ui_sliders: PlayerUiSliders,
    pub ui_sliders_drag: PlayerUiSliders,
    pub ui_info_state: InfoUiState,
    // pub(crate) ui_state_mem: UiStateMemory,
    // pub ui_info_state: InfoUiState,
}

impl PlayerState {
    // Fill in the side panel with sliders for playback speed and buttons for play/pause/stop
    pub fn side_panel_ui(
        &mut self,
        mut ui: &mut Ui,
        mut commands: Commands,
        images: &mut ResMut<Assets<Image>>,
    ) {
        ui.horizontal(|ui|{
            ui.heading("ADΔER Parameters");
            if ui.add(egui::Button::new("Reset params")).clicked() {
                self.ui_state = Default::default();
                self.ui_sliders = Default::default();
                if self.ui_sliders_drag != self.ui_sliders {
                    self.reset_update_adder_params()
                }
                self.ui_sliders_drag = Default::default();

            }
            if ui.add(egui::Button::new("Reset video")).clicked() {
                self.player = AdderPlayer::default();
                self.ui_state = Default::default();
                self.ui_sliders = Default::default();
                self.ui_sliders_drag = Default::default();
                self.ui_info_state = Default::default();
                self.reset_update_adder_params();
                commands.insert_resource(Images::default());
            }
        });
        egui::Grid::new("my_grid")
            .num_columns(2)
            .spacing([10.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                self.side_panel_grid_contents(ui);
            });
    }

    pub fn side_panel_grid_contents(&mut self, ui: &mut Ui) {

        let mut need_to_update =
            add_slider_row(true, "Playback speed:", ui, &mut self.ui_sliders.playback_speed, &mut self.ui_sliders_drag.playback_speed, 0.1..=15.0, 0.1);

        ui.add_enabled(true, egui::Label::new("Playback controls:"));
        ui.horizontal(|ui| {
            if self.ui_state.playing {
                if ui.button("⏸").clicked() {
                    println!("Pause clicked");
                    self.ui_state.playing = false;
                }
            } else {
                if ui.button("▶").clicked() {
                    self.ui_state.playing = true;
                }
            }
            // TODO: remove this?
            if ui.button("⏹").clicked() {
                self.ui_state.playing = false;
                need_to_update = true;
            }

            if ui.button("⏮").clicked() {
                self.ui_state.playing = true;
                need_to_update = true;
            }
        });
        ui.end_row();

        // TODO: decoding is single-threaded for now
        add_slider_row(false, "Thread count:", ui, &mut self.ui_sliders.thread_count, &mut self.ui_sliders_drag.thread_count, 1..=(current_num_threads()-1).max(4), 1);
        need_to_update |= add_checkbox_row(true, "Loop:", "Loop playback?", ui, &mut self.ui_state.looping);    // TODO: add more sliders

        // TODO
        need_to_update |= add_radio_row(false, "View mode:",
                                        vec![
                                            ("Intensity", InstantaneousViewMode::Intensity,),
                                            ("D", InstantaneousViewMode::D,),
                                            ("Δt", InstantaneousViewMode::DeltaT,)
                                        ],
                                        ui, &mut self.ui_state.view_mode);
        need_to_update |= add_radio_row(true, "Reconstruction method:",
                                        vec![
                                            ("Fast", ReconstructionMethod::Fast,),
                                            ("Accurate", ReconstructionMethod::Accurate,),
                                        ],
                                        ui, &mut self.ui_state.reconstruction_method);



        if need_to_update {
            self.reset_update_adder_params()
        }

    }

    pub fn consume_source(
        &mut self,
        mut images: ResMut<Assets<Image>>,
        mut handles: ResMut<Images>,
        mut commands: Commands,
    ) {
        if !self.ui_state.playing {
            return;
        }

        let stream = match &mut self.player.input_stream {
            None => {
                return;
            }
            Some(s) => { s }
        };

        // Reset the stats if we're starting a new looped playback of the video
        if let Ok(pos) = stream.get_input_stream_position() {
            if pos == stream.header_size as u64 {
                self.player.frame_sequence.as_mut().unwrap().frames_written = 0;
                self.ui_info_state.clear_stats();
                self.ui_state.current_time = 0.0;
                self.ui_state.total_time = 0.0;
                self.ui_state.current_frame = 0;
                self.ui_state.total_frames = 0;
                self.player.current_t_ticks = 0;
            }
        }

        match self.ui_state.reconstruction_method {
            ReconstructionMethod::Fast => {
                self.consume_source_fast(images, handles, commands);
            }
            ReconstructionMethod::Accurate => {
                self.consume_source_accurate(images, handles, commands);
            }
        }
    }

    fn consume_source_fast(
        &mut self,
        mut images: ResMut<Assets<Image>>,
        mut handles: ResMut<Images>,
        mut commands: Commands,
    ) {
        if self.ui_state.current_frame == 0 {
            self.ui_state.current_frame = 1;    // TODO: temporary hack
        }
        if !self.ui_state.playing {
            return;
        }
        let stream = match &mut self.player.input_stream {
            None => {
                return;
            }
            Some(s) => { s }
        };

        let frame_sequence = match &mut self.player.frame_sequence {
            None => {
                return;
            }
            Some(s) => { s }
        };

        let frame_length = stream.ref_interval as f64 * self.ui_sliders.playback_speed as f64;    //TODO: temp
        {
        let display_mat = &mut self.player.display_mat;

        loop {
            // if event_count % divisor == 0 {
            //     write!(
            //         handle,
            //         "\rPlaying back ADΔER file...{}%",
            //         (event_count * 100) / num_events as u64
            //     )?;
            //     handle.flush().unwrap();
            // }
            if self.player.current_t_ticks as u128 > (self.ui_state.current_frame as u128 * frame_length as u128) {
                println!("Frame {}", self.ui_state.current_frame);
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
                        self.player.current_t_ticks += event.delta_t;
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

                    let db = display_mat.data_bytes_mut().unwrap();
                    db[(y*stream.width as i32*stream.channels as i32 + x* stream.channels as i32 + c) as usize] = frame_intensity as u8;
                    // unsafe {
                    //     let px: &mut u8 = display_mat.at_3d_unchecked_mut(y, x, c).unwrap();
                    //     *px = frame_intensity as u8;
                    // }
                }
                Err(e) => {
                    match stream.set_input_stream_position(stream.header_size as u64) {
                        Ok(_) => {}
                        Err(ee) => {eprintln!("{}", ee)}
                    };
                    self.player.frame_sequence = Some(self.player.framer_builder.clone().unwrap().finish());
                    if !self.ui_state.looping {
                        self.ui_state.playing = false;
                    }
                    self.player.current_t_ticks = 0;
                    return;

                }
                _ => {}
            }
        }
    }

        let mut image_mat_bgra = Mat::default();
        imgproc::cvt_color(&self.player.display_mat, &mut image_mat_bgra, imgproc::COLOR_BGR2BGRA, 4).unwrap();


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
        self.player.live_image = image_bevy;


        let handle = images.add(self.player.live_image.clone());
        handles.image_view = handle;
    }

    pub fn consume_source_accurate(
        &mut self,
        mut images: ResMut<Assets<Image>>,
        mut handles: ResMut<Images>,
        mut commands: Commands,
    ) {
        let stream = match &mut self.player.input_stream {
            None => {
                return;
            }
            Some(s) => { s }
        };

        let frame_sequence = match &mut self.player.frame_sequence {
            None => {
                return;
            }
            Some(s) => { s }
        };

        let display_mat = &mut self.player.display_mat;

        if frame_sequence.is_frame_0_filled().unwrap() {
            let mut idx = 0;
            for chunk_num in 0..frame_sequence.get_frame_chunks_num() {
                match frame_sequence.pop_next_frame_for_chunk(chunk_num) {
                    Some(arr) => {
                        for px in arr.iter() {
                            match px {
                                Some(event) =>  {
                                    let db = display_mat.data_bytes_mut().unwrap();
                                    db[idx] = *event;
                                    idx += 1;
                                },
                                None => { },
                            };
                        }
                    }
                    None => {
                        println!("Couldn't pop chunk {}!", chunk_num)
                    }
                }
            }
            frame_sequence.frames_written += 1;
            self.player.current_t_ticks += frame_sequence.tpf;
            let duration = Duration::from_nanos(((self.player.current_t_ticks as f64 / stream.tps as f64) * 1.0e9) as u64);
            println!("duration {:?}", to_string(duration));

            let mut image_mat_bgra = Mat::default();
            imgproc::cvt_color(display_mat, &mut image_mat_bgra, imgproc::COLOR_BGR2BGRA, 4).unwrap();


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
            self.player.live_image = image_bevy;


            let handle = images.add(self.player.live_image.clone());
            handles.image_view = handle;
        }





        loop {
            match stream.decode_event() {
                Ok(mut event) => {
                    self.ui_info_state.events_total += 1;
                    if frame_sequence.ingest_event(&mut event) {

                        break;
                    }
                }
                Err(_e) => {
                    println!("restarting");
                    stream.set_input_stream_position(stream.header_size as u64).unwrap();
                    self.player.frame_sequence = Some(self.player.framer_builder.clone().unwrap().finish());
                    if !self.ui_state.looping {
                        self.ui_state.playing = false;
                    }
                    return;
                }
            }
        }
    }

    pub fn central_panel_ui(
        &mut self,
        mut ui: &mut Ui,
        time: Res<Time>
    ) {

        ui.heading("Drag and drop your ADΔER file here (.adder)");

        ui.label(self.ui_info_state.source_name.clone());

        if let Some(stream) =  &self.player.input_stream {
            let duration = Duration::from_nanos((
                (self.player.current_t_ticks as f64 / stream.tps as f64) * 1.0e9) as u64);
            self.ui_info_state.events_per_sec = self.ui_info_state.events_total as f64 / duration.as_secs() as f64;
            self.ui_info_state.events_ppc_total = self.ui_info_state.events_total as f64 / stream.width as f64 / stream.height as f64 / stream.channels as f64;
            self.ui_info_state.events_ppc_per_sec = self.ui_info_state.events_ppc_total / duration.as_secs() as f64;
        }



        // TODO: make fps accurate and meaningful here
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

    fn reset_update_adder_params(&mut self) {

        self.ui_state.current_frame = match self.ui_state.reconstruction_method {
            ReconstructionMethod::Fast => { 1}
            ReconstructionMethod::Accurate => { 0}
        };
        self.ui_state.total_frames = 0;
        self.ui_state.current_time = 0.0;
        self.ui_state.total_time = 0.0;
        let path_buf = match &self.player.path_buf {
            None => {
                return;
            }
            Some(p) => {
                p
            }
        };


        self.player = AdderPlayer::new(path_buf, self.ui_sliders.playback_speed).unwrap();

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





        // video.instantaneous_view_mode = self.ui_state.view_mode_radio_state;
    }


    pub fn replace_player(&mut self, path_buf: &std::path::PathBuf) {
        self.player = AdderPlayer::new(path_buf, self.ui_sliders.playback_speed).unwrap();
        self.ui_info_state.source_name = RichText::from(path_buf.to_str().unwrap().to_string());
        self.ui_state.current_frame = 1;

    }
}

fn to_string(duration: Duration) -> String {
    let hours = duration.as_secs() / 3600;
    let mins = (duration.as_secs() % 3600) / 60;
    let secs = duration.as_secs() % 60;
    let nanos = duration.subsec_nanos();
    format!("{}:{}:{}.{:09}", hours, mins, secs, nanos)
}
