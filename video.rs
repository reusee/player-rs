extern crate time;
extern crate avformat;
extern crate avcodec;
extern crate avutil;
extern crate swscale;

use std::ptr::{array_each_with_len, mut_null, null};
use std::cast::transmute;
use std::mem::size_of;

pub struct Video {
  pub format_context: *avformat::AVFormatContext,
  pub streams: Vec<*avformat::AVStream>,
  pub video_streams: Vec<*avformat::AVStream>,
  pub audio_streams: Vec<*avformat::AVStream>,
}

impl Video {
  pub fn new(filename: ~str) -> Result<Video, &'static str> { unsafe {
    // format context
    let mut format_context: *mut avformat::AVFormatContext = mut_null();
    if avformat::avformat_open_input(&mut format_context, filename.to_c_str().unwrap(), mut_null(), mut_null()) != 0 {
      return Err("open input");
    }
    if avformat::avformat_find_stream_info(format_context, mut_null()) < 0 {
      return Err("find stream info error");
    }
    avformat::av_dump_format(format_context, 0, filename.to_c_str().unwrap(), 0);

    // streams
    let mut streams = Vec::new();
    let mut video_streams = Vec::new();
    let mut audio_streams = Vec::new();
    array_each_with_len((*format_context).streams as **avformat::AVStream,
      (*format_context).nb_streams as uint, {|stream| {
      streams.push(stream);
      let codec = (*stream).codec;
      match (*codec).codec_type {
        avutil::AVMEDIA_TYPE_VIDEO => video_streams.push(stream),
        avutil::AVMEDIA_TYPE_AUDIO => audio_streams.push(stream),
        _ => (), //TODO
      };
    }});

    // codecs
    for stream in streams.iter() {
      let codecCtx = (**stream).codec;
      let codec = avcodec::avcodec_find_decoder((*codecCtx).codec_id);
      if codec.is_null() {
        return Err("no decoder");
      }
      let mut options: *mut avutil::AVDictionary = mut_null();
      if avcodec::avcodec_open2(codecCtx, &*codec, &mut options) < 0 {
        return Err("open codec error");
      }
    }

    Ok(Video{
      format_context: &*format_context,
      streams: streams,
      video_streams: video_streams,
      audio_streams: audio_streams,
    })
  }}

  pub fn decode(&self, video_stream: *avformat::AVStream, audio_stream: *avformat::AVStream,
    width: int, height: int, timed_frame_in: Sender<*avcodec::AVFrame>) -> Decoder { unsafe {
    let vcodec_ctx = (*video_stream).codec;
    let acodec_ctx = (*audio_stream).codec;

    // frame pool
    let (pool_in, pool_out) = channel();
    let pool_size = 16;
    let buffer_size = avcodec::avpicture_get_size(avutil::PIX_FMT_YUV420P,
      width as i32, height as i32);
    let mut frames = Vec::new();
    let mut buffers = Vec::new();
    for _ in range(0, pool_size) {
      let frame = avcodec::avcodec_alloc_frame();
      frames.push(frame);
      let buffer = avutil::av_malloc(buffer_size as u64);
      buffers.push(buffer as *mut u8);
      avcodec::avpicture_fill(frame as *mut avcodec::AVPicture, buffer as *u8,
        avutil::PIX_FMT_YUV420P, width as i32, height as i32);
      pool_in.send(frame);
    }

    let decoder = Decoder{
      pool_in: pool_in,
      frames: frames,
      buffers: buffers,
      start_time: time::now(),
    };

    // decode
    let (frame_in, frame_out) = channel();
    let format_context = self.format_context as *mut avformat::AVFormatContext ;
    spawn(proc() {
      // scale context
      let scale_context = swscale::sws_getCachedContext(mut_null(),
        (*vcodec_ctx).width, (*vcodec_ctx).height, (*vcodec_ctx).pix_fmt,
        width as i32, height as i32, avutil::PIX_FMT_YUV420P,
        0x200, mut_null(), mut_null(), null());
      if scale_context.is_null() {
        fail!("get scale context");
      }

      let mut packet: *mut avcodec::AVPacket = unsafe {
        transmute(avutil::av_malloc(size_of::<avcodec::AVPacket>() as u64))
      };
      avcodec::av_init_packet(packet);
      let mut frame_finished: i32 = -1;
      let plane_size: int;
      let vframe = avcodec::avcodec_alloc_frame();
      let aframe = avcodec::avcodec_alloc_frame();
      let mut pts: f64 = 0f64;

      while avformat::av_read_frame(format_context, packet) >= 0 {
        avcodec::av_free_packet(packet);
        if (*packet).stream_index == (*video_stream).index { // video
          if avcodec::avcodec_decode_video2(vcodec_ctx, vframe, &mut frame_finished, &*packet) < 0 {
            continue
          }
          if (*packet).dts != avutil::AV_NOPTS_VALUE { // get pts
            pts = (*packet).dts as f64;
          } else { // more pts info
            fail!("FIXME: no pts info") //TODO
          }
          pts *= avutil::av_q2d((*video_stream).time_base) * 1000 as f64;
          if frame_finished > 0 { // got frame
            let buf_frame = pool_out.recv();
            swscale::sws_scale(scale_context, &((*vframe).data[0]), &((*vframe).linesize[0]),
              0, (*vcodec_ctx).height,
              &mut((*buf_frame).data[0] as *u8), &((*buf_frame).linesize[0]));
            (*buf_frame).pts = pts as i64;
            frame_in.send(buf_frame);
          }
        } else {
        //TODO audio and others
        }
      }
    });

    return decoder
  }}

}

impl Drop for Video {
  fn drop(&mut self) { unsafe {
    avformat::avformat_close_input(&mut(self.format_context as *mut avformat::AVFormatContext));
    //for stream in self.streams.iter() {
    //  ffmpeg::avcodec_close((**stream).codec);
    //}
  }}
}

struct Decoder {
  pool_in: Sender<*mut avcodec::AVFrame>,
  frames: Vec<*mut avcodec::AVFrame>,
  buffers: Vec<*mut u8>,
  start_time: time::Tm,
}
