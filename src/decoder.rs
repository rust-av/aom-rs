//! AO1 Decoder
//!
//!

use crate::ffi::*;
use std::marker::PhantomData;
use std::mem::{zeroed, MaybeUninit};
use std::os::raw;
use std::ptr;
use std::sync::Arc;

use crate::common::AOMCodec;
use av_data::frame::{Frame, FrameBufferCopy, FrameType, VideoInfo};
use av_data::pixel::formats::YUV420;

fn frame_from_img(img: aom_image_t) -> Frame {
    let f = match img.fmt {
        aom_img_fmt::AOM_IMG_FMT_I42016 => YUV420,
        ob_fmt => panic!("Received unknown format: {}", ob_fmt),
    };

    f.set_primaries_from_u32(img.cp as u32);
    f.set_xfer_from_u32(img.tc as u32);
    f.set_matrix_from_u32(img.mc as u32);

    let v = VideoInfo::new(
        img.d_w as usize,
        img.d_h as usize,
        false,
        FrameType::OTHER,
        Arc::new(*f),
    );

    let mut f = Frame::new_default_frame(v, None);

    let src = img
        .planes
        .iter()
        .zip(img.stride.iter())
        .map(|(v, l)| unsafe { std::slice::from_raw_parts(*v as *const u8, *l as usize) });

    let linesize = img.stride.iter().map(|l| *l as usize);

    f.copy_from_slice(src, linesize);
    f
}

/// AV1 Decoder
pub struct AV1Decoder<T> {
    pub(crate) ctx: aom_codec_ctx,
    pub(crate) iter: aom_codec_iter_t,
    private_data: PhantomData<T>,
}

unsafe impl<T: Send> Send for AV1Decoder<T> {} // TODO: Make sure it cannot be abused
unsafe impl<T: Sync> Sync for AV1Decoder<T> {} // TODO: Make sure it cannot be abused

impl<T> AV1Decoder<T> {
    /// Create a new decoder
    pub fn new() -> Result<AV1Decoder<T>, aom_codec_err_t::Type> {
        let mut ctx = MaybeUninit::uninit();
        let cfg = unsafe { zeroed() };

        let ret = unsafe {
            aom_codec_dec_init_ver(
                ctx.as_mut_ptr(),
                aom_codec_av1_dx(),
                &cfg as *const aom_codec_dec_cfg_t,
                0,
                AOM_DECODER_ABI_VERSION as i32,
            )
        };
        match ret {
            aom_codec_err_t::AOM_CODEC_OK => {
                let ctx = unsafe { ctx.assume_init() };
                Ok(AV1Decoder {
                    ctx,
                    iter: ptr::null(),
                    private_data: PhantomData,
                })
            }
            _ => Err(ret),
        }
    }

    /// Feed some compressed data to the encoder
    ///
    /// The `data` slice is sent to the decoder alongside the optional
    /// `private` struct.
    ///
    /// The [`get_frame`] method must be called to retrieve the decompressed
    /// frame, do not call this method again before calling [`get_frame`].
    ///
    /// It matches a call to `aom_codec_decode`.
    ///
    /// [`get_frame`]: #method.get_frame
    pub fn decode<O>(&mut self, data: &[u8], private: O) -> Result<(), aom_codec_err_t::Type>
    where
        O: Into<Option<T>>,
    {
        let priv_data = private
            .into()
            .map(|v| Box::into_raw(Box::new(v)))
            .unwrap_or(ptr::null_mut());
        let ret = unsafe {
            aom_codec_decode(
                &mut self.ctx,
                data.as_ptr(),
                data.len(),
                priv_data as *mut raw::c_void, // mem::transmute(priv_data)
            )
        };

        // Safety measure to not call get_frame on an invalid iterator
        self.iter = ptr::null();

        match ret {
            aom_codec_err_t::AOM_CODEC_OK => Ok(()),
            _ => {
                let _ = unsafe { Box::from_raw(priv_data) };
                Err(ret)
            }
        }
    }

    /// Notify the decoder to return any pending frame
    ///
    /// The [`get_frame`] method must be called to retrieve the decompressed
    /// frame.
    ///
    /// It matches a call to `aom_codec_decode` with NULL arguments.
    ///
    /// [`get_frame`]: #method.get_frame
    pub fn flush(&mut self) -> Result<(), aom_codec_err_t::Type> {
        let ret = unsafe { aom_codec_decode(&mut self.ctx, ptr::null(), 0, ptr::null_mut()) };

        self.iter = ptr::null();

        match ret {
            aom_codec_err_t::AOM_CODEC_OK => Ok(()),
            _ => Err(ret),
        }
    }

    /// Retrieve decoded frames
    ///
    /// Should be called repeatedly until it returns `None`.
    ///
    /// It matches a call to `aom_codec_get_frame`.
    pub fn get_frame(&mut self) -> Option<(Frame, Option<Box<T>>)> {
        let img = unsafe { aom_codec_get_frame(&mut self.ctx, &mut self.iter) };

        if img.is_null() {
            None
        } else {
            let im = unsafe { *img };
            let priv_data = if im.user_priv.is_null() {
                None
            } else {
                let p: *mut T = im.user_priv as *mut T;
                Some(unsafe { Box::from_raw(p) })
            };
            let frame = frame_from_img(im);
            Some((frame, priv_data))
        }
    }
}

impl<T> Drop for AV1Decoder<T> {
    fn drop(&mut self) {
        unsafe { aom_codec_destroy(&mut self.ctx) };
    }
}

impl<T> AOMCodec for AV1Decoder<T> {
    fn get_context(&mut self) -> &mut aom_codec_ctx {
        &mut self.ctx
    }
}

#[cfg(feature = "codec-trait")]
mod decoder_trait {
    use super::*;
    use av_codec::decoder::*;
    use av_codec::error::*;
    use av_data::frame::ArcFrame;
    use av_data::packet::Packet;
    use av_data::timeinfo::TimeInfo;
    use std::sync::Arc;

    pub struct Des {
        descr: Descr,
    }

    impl Descriptor for Des {
        type OutputDecoder = AV1Decoder<TimeInfo>;

        fn create(&self) -> Self::OutputDecoder {
            AV1Decoder::new().unwrap()
        }

        fn describe(&self) -> &Descr {
            &self.descr
        }
    }

    impl Decoder for AV1Decoder<TimeInfo> {
        fn set_extradata(&mut self, _extra: &[u8]) {
            // No-op
        }
        fn send_packet(&mut self, pkt: &Packet) -> Result<()> {
            self.decode(&pkt.data, pkt.t.clone())
                .map_err(|err| Error::Unsupported(format!("{}", err)))
        }
        fn receive_frame(&mut self) -> Result<ArcFrame> {
            self.get_frame()
                .map(|(mut f, t)| {
                    f.t = t.map(|b| *b).unwrap();
                    Arc::new(f)
                })
                .ok_or(Error::MoreDataNeeded)
        }
        fn flush(&mut self) -> Result<()> {
            self.flush()
                .map_err(|err| Error::Unsupported(format!("{}", err)))
        }
        fn configure(&mut self) -> Result<()> {
            Ok(())
        }
    }

    /// AV1 Decoder
    ///
    /// To be used with [av-codec](https://docs.rs/av-codec) `Context`.
    pub const AV1_DESCR: &Des = &Des {
        descr: Descr {
            codec: "av1",
            name: "aom",
            desc: "libaom AV1 decoder",
            mime: "video/AV1",
        },
    };
}

#[cfg(feature = "codec-trait")]
pub use self::decoder_trait::AV1_DESCR;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn init() {
        let mut d = AV1Decoder::<()>::new().unwrap();

        println!("{}", d.error_to_str());
    }

    use super::super::encoder::tests as enc;
    use super::super::encoder::AOMPacket;
    use av_data::rational::*;
    use av_data::timeinfo::TimeInfo;
    #[test]
    fn decode() {
        let w = 200;
        let h = 200;

        let t = TimeInfo {
            pts: Some(0),
            dts: Some(0),
            duration: Some(1),
            timebase: Some(Rational64::new(1, 1000)),
            user_private: None,
        };

        let mut e = enc::setup(w, h, &t);
        let mut f = enc::setup_frame(w, h, &t);

        let mut d = AV1Decoder::<()>::new().unwrap();
        let mut out = 0;

        for i in 0..100 {
            e.encode(&f).unwrap();
            f.t.pts = Some(i);

            // println!("{:#?}", f);
            loop {
                let p = e.get_packet();

                match p {
                    Some(AOMPacket::Packet(ref pkt)) => {
                        d.decode(&pkt.data, None).unwrap();

                        // No multiframe expected.
                        if let Some(f) = d.get_frame() {
                            out = 1;
                            println!("{:#?}", f);
                        }
                    }
                    _ => break,
                }
            }
        }

        if out != 1 {
            panic!("No frame decoded");
        }
    }
}
