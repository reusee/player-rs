#![feature(globs)]

extern crate libc;
extern crate sdl2;
extern crate avformat;

mod video;

#[link(name="avformat")]
#[link(name="avcodec")]
#[link(name="avutil")]
#[link(name="swscale")]
extern {}

fn main() {
  // init
  unsafe {
    avformat::av_register_all();
  }

  // sdl
  let (width, height) = (1024, 768);
  sdl2::init([sdl2::InitVideo, sdl2::InitAudio, sdl2::InitTimer]);
  let window = match sdl2::video::Window::new("player",
    sdl2::video::PosCentered, sdl2::video::PosCentered,
    width, height, [sdl2::video::Maximized, sdl2::video::Resizable]) {
    Ok(w) => w,
    Err(e) => fail!(format!("create window {}", e)),
  };
  let renderer = match sdl2::render::Renderer::from_window(window, sdl2::render::DriverAuto,
    [sdl2::render::Accelerated]) {
    Ok(r) => r,
    Err(e) => fail!(format!("create renderer {}", e)),
  };
  let texture = match renderer.create_texture(sdl2::pixels::YV12, sdl2::render::AccessStreaming,
    width, height) {
    Ok(t) => t,
    Err(e) => fail!(format!("create texture {}", e)),
  };

  // video
  let video = match video::Video::new(~"m.mp4") {
    Ok(v) => v,
    Err(e) => fail!(e),
  };
  if video.video_streams.is_empty() || video.audio_streams.is_empty() {
    fail!("no video or no audio");
  }

  // decode
  let (timed_frame_in, timed_frame_out) = channel();
  let decoder = video.decode(*video.video_streams.get(0), *video.audio_streams.get(0),
    width, height,
    timed_frame_in);

  // render
}
