# adder-viz
[![Crates.io](https://img.shields.io/crates/v/adder-viz)](https://crates.io/crates/adder-viz)
[![Downloads](https://img.shields.io/crates/dr/adder-viz)](https://crates.io/crates/adder-viz)

A GUI project to make it easier to tune the parameters of ADΔER transcoding.

![](https://github.com/ac-freeman/adder-tuner/blob/main/examples/screenshot.png)

## Dependencies

You may need to install the Bevy dependencies described [here](https://bevyengine.org/learn/book/getting-started/setup/) and install OpenCV as described [here](https://github.com/twistedfall/opencv-rust).

## Installation

`cargo install adder-viz`

# Usage

Run `adder-viz` in the terminal and the above window will open. Drag and drop your video of choice from a file manager, and the ADΔER transcode process will begin automatically. Currently, it only supports .mp4 video sources and .aedat4 DAVIS 346 camera sources. Some parameter adjustments, such as the video scale, require the transcode process to be relaunched, which causes a noticeable slowdown in the UI for a moment. The program can also playback `.adder` files, which you can even generate on the Transcode tab.

Check out the [Wiki](https://github.com/ac-freeman/adder-viz/wiki) for more detailed information and background on the ADΔER framework.

# TODO
 - [ ] Separate the concerns of UI rendering and retrieving information from the codec. Will require a major refactor.
 - [ ] Allow the output of framed video from the playback tab
 - [ ] Allow scrubbing to different time points in playback tab
 - [ ] Allow scrubbing to different time points in transcode tab. Scrubbing to a new time will restart the transcode process from that time, overwriting any previous data.
