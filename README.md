A GUI project to make it easier to tune the parameters of ADΔER transcoding.

I also just wanted to learn the Bevy and egui libraries, and this is a good excuse.

![](https://github.com/ac-freeman/adder-tuner/blob/main/examples/screenshot.png)

# Installation

`cargo install adder-viz`

# Usage

Run `adder-viz` in the terminal and the above window will open. Drag and drop your video of choice from a file manager, and the ADΔER transcode process will begin automatically. Currently, it only supports framed video sources (specifically .mp4 files), although event video sources will be supported soon. Some parameter adjustments, such as the video scale, require the transcode process to be relaunched, which causes a noticeable slowdown in the UI for a moment.
