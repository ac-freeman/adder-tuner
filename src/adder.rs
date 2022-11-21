use std::error::Error;
use std::ffi::OsStr;
use std::fmt;
use std::path::PathBuf;
use bevy::prelude::Resource;
use adder_codec_rs::transcoder::source::framed_source::FramedSource;
use adder_codec_rs::transcoder::source::davis_source::DavisSource;


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
    pub fn new(path_buf: &PathBuf) -> Result<Self, Box<dyn Error>> {
        match path_buf.extension() {
            None => {
                Err(Box::new(AdderTranscoderError("Invalid file type".into())))
            }
            Some(ext) => {
                match ext.to_str() {
                    None => {Err(Box::new(AdderTranscoderError("Invalid file type".into())))}
                    Some("mp4") => {
                        Ok(AdderTranscoder {
                            framed_source: None,
                            davis_source: None,
                        })
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