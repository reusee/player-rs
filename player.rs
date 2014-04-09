#![feature(globs)]

extern crate libc;

mod ffmpeg;
mod video;
mod sdl;

#[link(name="avformat")]
#[link(name="avcodec")]
#[link(name="avutil")]
#[link(name="swscale")]
extern {}

fn main() {
  // init
  unsafe {
    ffmpeg::av_register_all();
  }

  // video
  let video = match video::Video::new(~"m.mp4") {
    Ok(v) => v,
    Err(e) => fail!(e),
  };
  drop(video);
}
