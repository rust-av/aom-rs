//! Encoding functionality
//!
//!

use crate::common::AOMCodec;
use crate::ffi::*;

use std::mem::{self, MaybeUninit};
use std::ptr;

use crate::data::frame::{Frame, FrameBufferConv, MediaKind};
use crate::data::packet::Packet;
use crate::data::pixel::formats::YUV420;
use crate::data::pixel::Formaton;

#[derive(Clone, Debug, PartialEq)]
pub struct PSNR {
    pub samples: [u32; 4],
    pub sse: [u64; 4],
    pub psnr: [f64; 4],
}

/// Safe wrapper around `aom_codec_cx_pkt`
#[derive(Clone, Debug)]
pub enum AOMPacket {
    Packet(Packet),
    Stats(Vec<u8>),
    MBStats(Vec<u8>),
    PSNR(PSNR),
    Custom(Vec<u8>),
}

fn to_buffer(buf: aom_fixed_buf_t) -> Vec<u8> {
    let mut v: Vec<u8> = Vec::with_capacity(buf.sz);
    unsafe {
        ptr::copy_nonoverlapping(buf.buf as *const u8, v.as_mut_ptr(), buf.sz);
        v.set_len(buf.sz);
    }
    v
}

impl AOMPacket {
    fn new(pkt: aom_codec_cx_pkt) -> AOMPacket {
        match pkt.kind {
            aom_codec_cx_pkt_kind::AOM_CODEC_CX_FRAME_PKT => {
                let f = unsafe { pkt.data.frame };
                let mut p = Packet::with_capacity(f.sz);
                unsafe {
                    ptr::copy_nonoverlapping(f.buf as *const u8, p.data.as_mut_ptr(), f.sz);
                    p.data.set_len(f.sz);
                }
                p.t.pts = Some(f.pts);
                p.is_key = (f.flags & AOM_FRAME_IS_KEY) != 0;

                AOMPacket::Packet(p)
            }
            aom_codec_cx_pkt_kind::AOM_CODEC_STATS_PKT => {
                let b = to_buffer(unsafe { pkt.data.twopass_stats });
                AOMPacket::Stats(b)
            }
            aom_codec_cx_pkt_kind::AOM_CODEC_FPMB_STATS_PKT => {
                let b = to_buffer(unsafe { pkt.data.firstpass_mb_stats });
                AOMPacket::MBStats(b)
            }
            aom_codec_cx_pkt_kind::AOM_CODEC_PSNR_PKT => {
                let p = unsafe { pkt.data.psnr };

                AOMPacket::PSNR(PSNR {
                    samples: p.samples,
                    sse: p.sse,
                    psnr: p.psnr,
                })
            }
            aom_codec_cx_pkt_kind::AOM_CODEC_CUSTOM_PKT => {
                let b = to_buffer(unsafe { pkt.data.raw });
                AOMPacket::Custom(b)
            }
            _ => panic!("No packet defined"),
        }
    }
}

pub struct AV1EncoderConfig {
    pub cfg: aom_codec_enc_cfg,
}

unsafe impl Send for AV1EncoderConfig {} // TODO: Make sure it cannot be abused

// TODO: Extend
fn map_formaton(img: &mut aom_image, fmt: &Formaton) {
    if fmt == YUV420 {
        img.fmt = aom_img_fmt::AOM_IMG_FMT_I420;
    } else {
        unimplemented!();
    }
    img.bit_depth = 8;
    img.bps = 12;
    img.x_chroma_shift = 1;
    img.y_chroma_shift = 1;
    img.cp = fmt.get_primaries() as u32;
    img.tc = fmt.get_xfer() as u32;
    img.mc = fmt.get_matrix() as u32;
}

fn img_from_frame(frame: &Frame) -> aom_image {
    let mut img: aom_image = unsafe { mem::zeroed() };

    if let MediaKind::Video(ref v) = frame.kind {
        map_formaton(&mut img, &v.format);
        img.w = v.width as u32;
        img.h = v.height as u32;
        img.d_w = v.width as u32;
        img.d_h = v.height as u32;
    }
    // populate the buffers
    for i in 0..frame.buf.count() {
        let s: &[u8] = frame.buf.as_slice(i).unwrap();
        img.planes[i] = s.as_ptr() as *mut u8;
        img.stride[i] = frame.buf.linesize(i).unwrap() as i32;
    }

    img
}

// TODO: provide a builder?

/// AV1 Encoder setup facility
impl AV1EncoderConfig {
    /// Create a new default configuration
    pub fn new() -> Result<AV1EncoderConfig, aom_codec_err_t::Type> {
        let mut cfg = MaybeUninit::uninit();
        let ret = unsafe { aom_codec_enc_config_default(aom_codec_av1_cx(), cfg.as_mut_ptr(), 0) };

        match ret {
            aom_codec_err_t::AOM_CODEC_OK => {
                let cfg = unsafe { cfg.assume_init() };
                Ok(AV1EncoderConfig { cfg })
            },
            _ => Err(ret),
        }
    }

    /// Return a newly allocated `AV1Encoder` using the current configuration
    pub fn get_encoder(&mut self) -> Result<AV1Encoder, aom_codec_err_t::Type> {
        AV1Encoder::new(self)
    }
}

/// AV1 Encoder
pub struct AV1Encoder {
    pub(crate) ctx: aom_codec_ctx_t,
    pub(crate) iter: aom_codec_iter_t,
}

unsafe impl Send for AV1Encoder {} // TODO: Make sure it cannot be abused

impl AV1Encoder {
    /// Create a new encoder using the provided configuration
    ///
    /// You may use `get_encoder` instead.
    pub fn new(cfg: &mut AV1EncoderConfig) -> Result<AV1Encoder, aom_codec_err_t::Type> {
        let mut ctx = MaybeUninit::uninit();
        let ret = unsafe {
            aom_codec_enc_init_ver(
                ctx.as_mut_ptr(),
                aom_codec_av1_cx(),
                &cfg.cfg,
                0,
                AOM_ENCODER_ABI_VERSION as i32,
            )
        };

        match ret {
            aom_codec_err_t::AOM_CODEC_OK => {
                let ctx = unsafe { ctx.assume_init() };
                Ok(AV1Encoder {
                    ctx,
                    iter: ptr::null(),
                })
            },
            _ => Err(ret),
        }
    }

    /// Update the encoder parameters after-creation
    ///
    /// It calls `aom_codec_control_`
    pub fn control(
        &mut self,
        id: aome_enc_control_id::Type,
        val: i32,
    ) -> Result<(), aom_codec_err_t::Type> {
        let ret = unsafe { aom_codec_control_(&mut self.ctx, id as i32, val) };

        match ret {
            aom_codec_err_t::AOM_CODEC_OK => Ok(()),
            _ => Err(ret),
        }
    }

    // TODO: Cache the image information
    //
    /// Send an uncompressed frame to the encoder
    ///
    /// Call [`get_packet`] to receive the compressed data.
    ///
    /// It calls `aom_codec_encode`.
    ///
    /// [`get_packet`]: #method.get_packet
    pub fn encode(&mut self, frame: &Frame) -> Result<(), aom_codec_err_t::Type> {
        let img = img_from_frame(frame);

        let ret = unsafe { aom_codec_encode(&mut self.ctx, &img, frame.t.pts.unwrap(), 1, 0) };

        self.iter = ptr::null();

        match ret {
            aom_codec_err_t::AOM_CODEC_OK => Ok(()),
            _ => Err(ret),
        }
    }

    /// Notify the encoder that no more data will be sent
    ///
    /// Call [`get_packet`] to receive the compressed data.
    ///
    /// It calls `aom_codec_encode` with NULL arguments.
    ///
    /// [`get_packet`]: #method.get_packet
    pub fn flush(&mut self) -> Result<(), aom_codec_err_t::Type> {
        let ret = unsafe { aom_codec_encode(&mut self.ctx, ptr::null_mut(), 0, 1, 0) };

        self.iter = ptr::null();

        match ret {
            aom_codec_err_t::AOM_CODEC_OK => Ok(()),
            _ => Err(ret),
        }
    }

    /// Retrieve the compressed data
    ///
    /// To be called until it returns `None`.
    ///
    /// It calls `aom_codec_get_cx_data`.
    pub fn get_packet(&mut self) -> Option<AOMPacket> {
        let pkt = unsafe { aom_codec_get_cx_data(&mut self.ctx, &mut self.iter) };

        if pkt.is_null() {
            None
        } else {
            Some(AOMPacket::new(unsafe { *pkt }))
        }
    }
}

impl Drop for AV1Encoder {
    fn drop(&mut self) {
        unsafe { aom_codec_destroy(&mut self.ctx) };
    }
}

impl AOMCodec for AV1Encoder {
    fn get_context(&mut self) -> &mut aom_codec_ctx {
        &mut self.ctx
    }
}

#[cfg(feature = "codec-trait")]
mod encoder_trait {
    use super::*;
    use crate::codec::encoder::*;
    use crate::codec::error::*;
    use crate::data::frame::ArcFrame;
    use crate::data::params::{CodecParams, MediaKind, VideoInfo};
    use crate::data::value::Value;

    struct Des {
        descr: Descr,
    }

    struct Enc {
        cfg: AV1EncoderConfig,
        enc: Option<AV1Encoder>,
    }

    impl Descriptor for Des {
        fn create(&self) -> Box<dyn Encoder> {
            Box::new(Enc {
                cfg: AV1EncoderConfig::new().unwrap(),
                enc: None,
            })
        }

        fn describe(&self) -> &Descr {
            &self.descr
        }
    }

    impl Encoder for Enc {
        fn configure(&mut self) -> Result<()> {
            if self.enc.is_none() {
                self.cfg
                    .get_encoder()
                    .map(|enc| {
                        self.enc = Some(enc);
                    })
                    .map_err(|_err| Error::ConfigurationIncomplete)
            } else {
                unimplemented!()
            }
        }

        // TODO: have it as default impl?
        fn get_extradata(&self) -> Option<Vec<u8>> {
            None
        }

        fn send_frame(&mut self, frame: &ArcFrame) -> Result<()> {
            let enc = self.enc.as_mut().unwrap();
            enc.encode(frame).map_err(|e| match e {
                _ => unimplemented!(),
            })
        }

        fn receive_packet(&mut self) -> Result<Packet> {
            let enc = self.enc.as_mut().unwrap();

            if let Some(p) = enc.get_packet() {
                match p {
                    AOMPacket::Packet(pkt) => Ok(pkt),
                    _ => unimplemented!(),
                }
            } else {
                Err(Error::MoreDataNeeded)
            }
        }

        fn flush(&mut self) -> Result<()> {
            let enc = self.enc.as_mut().unwrap();
            enc.flush().map_err(|e| match e {
                _ => unimplemented!(),
            })
        }

        fn set_option<'a>(&mut self, key: &str, val: Value<'a>) -> Result<()> {
            match (key, val) {
                ("w", Value::U64(v)) => self.cfg.cfg.g_w = v as u32,
                ("h", Value::U64(v)) => self.cfg.cfg.g_h = v as u32,
                ("qmin", Value::U64(v)) => self.cfg.cfg.rc_min_quantizer = v as u32,
                ("qmax", Value::U64(v)) => self.cfg.cfg.rc_max_quantizer = v as u32,
                ("timebase", Value::Pair(num, den)) => {
                    self.cfg.cfg.g_timebase.num = num as i32;
                    self.cfg.cfg.g_timebase.den = den as i32;
                }
                _ => unimplemented!(),
            }

            Ok(())
        }

        fn get_params(&self) -> Result<CodecParams> {
            use std::sync::Arc;
            Ok(CodecParams {
                kind: Some(MediaKind::Video(VideoInfo {
                    height: self.cfg.cfg.g_h as usize,
                    width: self.cfg.cfg.g_w as usize,
                    format: Some(Arc::new(*YUV420)), // TODO: support more formats
                })),
                codec_id: Some("av1".to_owned()),
                extradata: None,
                bit_rate: 0, // TODO: expose the information
                convergence_window: 0,
                delay: 0,
            })
        }

        fn set_params(&mut self, params: &CodecParams) -> Result<()> {
            if let Some(MediaKind::Video(ref info)) = params.kind {
                self.cfg.cfg.g_w = info.width as u32;
                self.cfg.cfg.g_h = info.height as u32;
            }
            Ok(())
        }
    }

    /// AV1 Encoder
    ///
    /// To be used with [av-codec](https://docs.rs/av-codec) `Encoder Context`.
    pub const AV1_DESCR: &dyn Descriptor = &Des {
        descr: Descr {
            codec: "av1",
            name: "aom",
            desc: "libaom AV1 encoder",
            mime: "video/AV1",
        },
    };
}

#[cfg(feature = "codec-trait")]
pub use self::encoder_trait::AV1_DESCR;

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    #[test]
    fn init() {
        let mut c = AV1EncoderConfig::new().unwrap();
        let mut e = c.get_encoder().unwrap();
        println!("{}", e.error_to_str());
    }

    use crate::data::rational::*;
    use crate::data::timeinfo::TimeInfo;
    pub fn setup(w: u32, h: u32, t: &TimeInfo) -> AV1Encoder {
        let mut c = AV1EncoderConfig::new().unwrap();
        if (w % 2) != 0 || (h % 2) != 0 {
            panic!("Invalid frame size: w: {} h: {}", w, h);
        }
        c.cfg.g_w = w;
        c.cfg.g_h = h;
        c.cfg.g_timebase.num = *t.timebase.unwrap().numer() as i32;
        c.cfg.g_timebase.den = *t.timebase.unwrap().denom() as i32;
        c.cfg.rc_min_quantizer = 0;
        c.cfg.rc_min_quantizer = 0;
        c.cfg.g_threads = 4;
        c.cfg.g_pass = aom_enc_pass::AOM_RC_ONE_PASS;
        c.cfg.rc_end_usage = aom_rc_mode::AOM_CQ;

        let mut enc = c.get_encoder().unwrap();

        enc.control(aome_enc_control_id::AOME_SET_CQ_LEVEL, 4)
            .unwrap();

        enc
    }

    pub fn setup_frame(w: u32, h: u32, t: &TimeInfo) -> Frame {
        use crate::data::frame::*;
        use crate::data::pixel::formats;
        use std::sync::Arc;

        let v = VideoInfo {
            pic_type: PictureType::UNKNOWN,
            width: w as usize,
            height: h as usize,
            format: Arc::new(*formats::YUV420),
        };

        new_default_frame(v, Some(t.clone()))
    }

    #[test]
    fn encode() {
        let w = 200;
        let h = 200;

        let t = TimeInfo {
            pts: Some(0),
            dts: Some(0),
            duration: Some(1),
            timebase: Some(Rational64::new(1, 1000)),
            user_private: None,
        };

        let mut e = setup(w, h, &t);
        let mut f = setup_frame(w, h, &t);

        let mut out = 0;
        // TODO write some pattern
        for i in 0..100 {
            e.encode(&f).unwrap();
            f.t.pts = Some(i);
            // println!("{:#?}", f);
            loop {
                let p = e.get_packet();

                if p.is_none() {
                    break;
                } else {
                    out = 1;
                    // println!("{:#?}", p.unwrap());
                }
            }
        }

        if out != 1 {
            panic!("No packet produced");
        }
    }

    #[cfg(all(test, feature = "codec-trait"))]
    #[test]
    fn encode_codec_trait() {
        use super::AV1_DESCR;
        use crate::codec::encoder::*;
        use crate::codec::error::*;
        use std::sync::Arc;

        let encoders = Codecs::from_list(&[AV1_DESCR]);
        let mut ctx = Context::by_name(&encoders, "av1").unwrap();
        let w = 200;
        let h = 200;

        ctx.set_option("w", u64::from(w)).unwrap();
        ctx.set_option("h", u64::from(h)).unwrap();
        ctx.set_option("timebase", (1, 1000)).unwrap();
        ctx.set_option("qmin", 0u64).unwrap();
        ctx.set_option("qmax", 0u64).unwrap();

        let t = TimeInfo {
            pts: Some(0),
            dts: Some(0),
            duration: Some(1),
            timebase: Some(Rational64::new(1, 1000)),
            user_private: None,
        };

        ctx.configure().unwrap();
        let mut f = Arc::new(setup_frame(w, h, &t));
        let mut out = 0;
        for i in 0..100 {
            Arc::get_mut(&mut f).unwrap().t.pts = Some(i);

            println!("Sending {}", i);
            ctx.send_frame(&f).unwrap();

            loop {
                match ctx.receive_packet() {
                    Ok(p) => {
                        println!("{:#?}", p);
                        out = 1
                    }
                    Err(e) => match e {
                        Error::MoreDataNeeded => break,
                        _ => unimplemented!(),
                    },
                }
            }
        }

        ctx.flush().unwrap();

        loop {
            match ctx.receive_packet() {
                Ok(p) => {
                    println!("{:#?}", p);
                    out = 1
                }
                Err(e) => match e {
                    Error::MoreDataNeeded => break,
                    _ => unimplemented!(),
                },
            }
        }

        if out != 1 {
            panic!("No packet produced");
        }
    }
}
