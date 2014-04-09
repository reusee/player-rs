use ffmpeg;
use std::ptr::{array_each_with_len, mut_null};

pub struct Video {
  format_context: *ffmpeg::AVFormatContext,
  streams: Vec<*ffmpeg::AVStream>,
  video_streams: Vec<*ffmpeg::AVStream>,
  audio_streams: Vec<*ffmpeg::AVStream>,
}

impl Video {
  pub fn new(filename: ~str) -> Result<Video, &'static str> { unsafe {
    // format context
    let mut format_context: *mut ffmpeg::AVFormatContext = mut_null();
    if ffmpeg::avformat_open_input(&mut format_context, filename.to_c_str().unwrap(), mut_null(), mut_null()) != 0 {
      return Err("open input");
    }
    if ffmpeg::avformat_find_stream_info(format_context, mut_null()) < 0 {
      return Err("find stream info error");
    }
    ffmpeg::av_dump_format(format_context, 0, filename.to_c_str().unwrap(), 0);

    // streams
    let mut streams = Vec::new();
    let mut video_streams = Vec::new();
    let mut audio_streams = Vec::new();
    array_each_with_len((*format_context).streams as **ffmpeg::AVStream,
      (*format_context).nb_streams as uint, {|stream| {
      streams.push(stream);
      let codec = (*stream).codec;
      match (*codec).codec_type {
        ffmpeg::AVMEDIA_TYPE_VIDEO => video_streams.push(stream),
        ffmpeg::AVMEDIA_TYPE_AUDIO => audio_streams.push(stream),
        _ => (), //TODO
      };
    }});
    if video_streams.is_empty() || audio_streams.is_empty() {
      return Err("no audio or no video");
    }

    // codecs
    for stream in streams.iter() {
      let codecCtx = (**stream).codec;
      let codec = ffmpeg::avcodec_find_decoder((*codecCtx).codec_id);
      if codec.is_null() {
        return Err("no decoder");
      }
      let mut options: *mut ffmpeg::AVDictionary = mut_null();
      if ffmpeg::avcodec_open2(codecCtx, &*codec, &mut options) < 0 {
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

}

impl Drop for Video {
  fn drop(&mut self) { unsafe {
    ffmpeg::avformat_close_input(&mut(self.format_context as *mut ffmpeg::AVFormatContext));
    //for stream in self.streams.iter() {
    //  ffmpeg::avcodec_close((**stream).codec);
    //}
  }}
}
