use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};

use crate::encoder::AV1Encoder;
use crate::ffi::*;

/// Encoder configuration structure
///
/// This structure contains the encoder settings that have common representations
/// across all codecs. This doesn't imply that all codecs support all features,
/// however.
pub struct AV1EncoderConfig {
    cfg: aom_codec_enc_cfg,
}

unsafe impl Send for AV1EncoderConfig {} // TODO: Make sure it cannot be abused

impl AV1EncoderConfig {
    /// Create a new default configuration
    pub fn new() -> Result<AV1EncoderConfig, aom_codec_err_t::Type> {
        let mut cfg = MaybeUninit::uninit();
        let ret = unsafe { aom_codec_enc_config_default(aom_codec_av1_cx(), cfg.as_mut_ptr(), 0) };

        match ret {
            aom_codec_err_t::AOM_CODEC_OK => {
                let cfg = unsafe { cfg.assume_init() };
                Ok(AV1EncoderConfig { cfg })
            }
            _ => Err(ret),
        }
    }

    /// Return a newly allocated `AV1Encoder` using the current configuration
    pub fn get_encoder(&mut self) -> Result<AV1Encoder, aom_codec_err_t::Type> {
        AV1Encoder::new(self)
    }

    /// Get a reference to the underlying config structure
    pub fn cfg(&self) -> &aom_codec_enc_cfg {
        &self.cfg
    }

    /// Get a mutable reference to the underlying config structure
    pub fn cfg_mut(&mut self) -> &mut aom_codec_enc_cfg {
        &mut self.cfg
    }
}

/// # Generic settings (g)
impl AV1EncoderConfig {
    /// Algorithm specific "usage" value
    ///
    /// Algorithms may define multiple values for usage, which may convey the
    /// intent of how the application intends to use the stream. If this value
    /// is non-zero, consult the documentation for the codec to determine its
    /// meaning.
    pub fn g_usage(mut self, val: u32) -> Self {
        self.cfg.g_usage = val;
        self
    }

    /// Maximum number of threads to use
    ///
    /// For multi-threaded implementations, use no more than this number of
    /// threads. The codec may use fewer threads than allowed. The value
    /// 0 is equivalent to the value 1.
    pub fn g_threads(mut self, val: u32) -> Self {
        self.cfg.g_threads = val;
        self
    }

    /// Bitstream profile to use
    ///
    /// Some codecs support a notion of multiple bitstream profiles. Typically
    /// this maps to a set of features that are turned on or off. Often the
    /// profile to use is determined by the features of the intended decoder.
    /// Consult the documentation for the codec to determine the valid values
    /// for this parameter, or set to zero for a sane default.
    pub fn g_profile(mut self, val: u32) -> Self {
        self.cfg.g_profile = val;
        self
    }

    /// Width of the frame
    ///
    /// This value identifies the presentation resolution of the frame,
    /// in pixels. Note that the frames passed as input to the encoder must
    /// have this resolution. Frames will be presented by the decoder in this
    /// resolution, independent of any spatial resampling the encoder may do.
    pub fn g_w(mut self, val: u32) -> Self {
        self.cfg.g_w = val;
        self
    }

    /// Height of the frame
    ///
    /// This value identifies the presentation resolution of the frame,
    /// in pixels. Note that the frames passed as input to the encoder must
    /// have this resolution. Frames will be presented by the decoder in this
    /// resolution, independent of any spatial resampling the encoder may do.
    pub fn g_h(mut self, val: u32) -> Self {
        self.cfg.g_h = val;
        self
    }

    /// Max number of frames to encode
    ///
    /// If force video mode is off (the default) and g_limit is 1, the encoder
    /// will encode a still picture (still_picture is set to 1 in the sequence
    /// header OBU). If in addition full_still_picture_hdr is 0 (the default),
    /// the encoder will use a reduced header (reduced_still_picture_header is
    /// set to 1 in the sequence header OBU) for the still picture.
    pub fn g_limit(mut self, val: u32) -> Self {
        self.cfg.g_limit = val;
        self
    }

    /// Forced maximum width of the frame
    ///
    /// If this value is non-zero then it is used to force the maximum frame
    /// width written in write_sequence_header().
    pub fn g_forced_max_frame_width(mut self, val: u32) -> Self {
        self.cfg.g_forced_max_frame_width = val;
        self
    }

    /// Forced maximum height of the frame
    ///
    /// If this value is non-zero then it is used to force the maximum frame
    /// height written in write_sequence_header().
    pub fn g_forced_max_frame_height(mut self, val: u32) -> Self {
        self.cfg.g_forced_max_frame_height = val;
        self
    }

    /// Bit-depth of the codec
    ///
    /// This value identifies the bit_depth of the codec,
    /// Only certain bit-depths are supported as identified in the
    /// aom_bit_depth_t enum.
    pub fn g_bit_depth(mut self, val: aom_bit_depth_t) -> Self {
        self.cfg.g_bit_depth = val;
        self
    }

    /// Bit-depth of the input frames
    ///
    /// This value identifies the bit_depth of the input frames in bits.
    /// Note that the frames passed as input to the encoder must have
    /// this bit-depth.
    pub fn g_input_bit_depth(mut self, val: u32) -> Self {
        self.cfg.g_input_bit_depth = val;
        self
    }

    /// Stream timebase units
    ///
    /// Indicates the smallest interval of time, in seconds, used by the stream.
    /// For fixed frame rate material, or variable frame rate material where
    /// frames are timed at a multiple of a given clock (ex: video capture),
    /// the \ref RECOMMENDED method is to set the timebase to the reciprocal
    /// of the frame rate (ex: 1001/30000 for 29.970 Hz NTSC). This allows the
    /// pts to correspond to the frame number, which can be handy. For
    /// re-encoding video from containers with absolute time timestamps, the
    /// \ref RECOMMENDED method is to set the timebase to that of the parent
    /// container or multimedia framework (ex: 1/1000 for ms, as in FLV).
    pub fn g_timebase(mut self, val: aom_rational) -> Self {
        self.cfg.g_timebase = val;
        self
    }

    pub fn g_timebase_with<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut aom_rational),
    {
        f(&mut self.cfg.g_timebase);
        self
    }

    /// Enable error resilient modes.
    ///
    /// The error resilient bitfield indicates to the encoder which features
    /// it should enable to take measures for streaming over lossy or noisy
    /// links.
    pub fn g_error_resilient(mut self, val: aom_codec_er_flags_t) -> Self {
        self.cfg.g_error_resilient = val;
        self
    }

    /// Multi-pass Encoding Mode
    ///
    /// This value should be set to the current phase for multi-pass encoding.
    /// For single pass, set to #AOM_RC_ONE_PASS.
    pub fn g_pass(mut self, val: aom::aom_enc_pass::Type) -> Self {
        self.cfg.g_pass = val;
        self
    }

    /// Allow lagged encoding
    ///
    /// If set, this value allows the encoder to consume a number of input
    /// frames before producing output frames. This allows the encoder to
    /// base decisions for the current frame on future frames. This does
    /// increase the latency of the encoding pipeline, so it is not appropriate
    /// in all situations (ex: realtime encoding).
    ///
    /// Note that this is a maximum value -- the encoder may produce frames
    /// sooner than the given limit. Set this value to 0 to disable this
    /// feature.
    pub fn g_lag_in_frames(mut self, val: u32) -> Self {
        self.cfg.g_lag_in_frames = val;
        self
    }
}

/// # Rate control settings (rc)
impl AV1EncoderConfig {
    /// Temporal resampling configuration, if supported by the codec.
    ///
    /// Temporal resampling allows the codec to "drop" frames as a strategy to
    /// meet its target data rate. This can cause temporal discontinuities in
    /// the encoded video, which may appear as stuttering during playback. This
    /// trade-off is often acceptable, but for many applications is not. It can
    /// be disabled in these cases.
    ///
    /// Note that not all codecs support this feature. All aom AVx codecs do.
    /// For other codecs, consult the documentation for that algorithm.
    ///
    /// This threshold is described as a percentage of the target data buffer.
    /// When the data buffer falls below this percentage of fullness, a
    /// dropped frame is indicated. Set the threshold to zero (0) to disable
    /// this feature.
    pub fn rc_dropframe_thresh(mut self, val: u32) -> Self {
        self.cfg.rc_dropframe_thresh = val;
        self
    }

    /// Mode for spatial resampling, if supported by the codec.
    ///
    /// Spatial resampling allows the codec to compress a lower resolution
    /// version of the frame, which is then upscaled by the decoder to the
    /// correct presentation resolution. This increases visual quality at
    /// low data rates, at the expense of CPU time on the encoder/decoder.
    pub fn rc_resize_mode(mut self, val: u32) -> Self {
        self.cfg.rc_resize_mode = val;
        self
    }

    /// Frame resize denominator.
    ///
    /// The denominator for resize to use, assuming 8 as the numerator.
    ///
    /// Valid denominators are  8 - 16 for now.
    pub fn rc_resize_denominator(mut self, val: u32) -> Self {
        self.cfg.rc_resize_denominator = val;
        self
    }

    /// Keyframe resize denominator.
    ///
    /// The denominator for resize to use, assuming 8 as the numerator.
    ///
    /// Valid denominators are  8 - 16 for now.
    pub fn rc_resize_kf_denominator(mut self, val: u32) -> Self {
        self.cfg.rc_resize_kf_denominator = val;
        self
    }

    /// Frame super-resolution scaling mode.
    ///
    /// Similar to spatial resampling, frame super-resolution integrates
    /// upscaling after the encode/decode process. Taking control of upscaling and
    /// using restoration filters should allow it to outperform normal resizing.
    pub fn rc_superres_mode(mut self, val: aom::aom_superres_mode::Type) -> Self {
        self.cfg.rc_superres_mode = val;
        self
    }

    /// Frame super-resolution denominator.
    ///
    /// The denominator for superres to use. If fixed it will only change if the
    /// cumulative scale change over resizing and superres is greater than 1/2;
    /// this forces superres to reduce scaling.
    ///
    /// Valid denominators are 8 to 16.
    ///
    /// Used only by AOM_SUPERRES_FIXED.
    pub fn rc_superres_denominator(mut self, val: u32) -> Self {
        self.cfg.rc_superres_denominator = val;
        self
    }

    /// Keyframe super-resolution denominator.
    ///
    /// The denominator for superres to use. If fixed it will only change if the
    /// cumulative scale change over resizing and superres is greater than 1/2;
    /// this forces superres to reduce scaling.
    ///
    /// Valid denominators are 8 - 16 for now.
    pub fn rc_superres_kf_denominator(mut self, val: u32) -> Self {
        self.cfg.rc_superres_kf_denominator = val;
        self
    }

    /// Frame super-resolution q threshold.
    ///
    /// The q level threshold after which superres is used.
    /// Valid values are 1 to 63.
    ///
    /// Used only by AOM_SUPERRES_QTHRESH
    pub fn rc_superres_qthresh(mut self, val: u32) -> Self {
        self.cfg.rc_superres_qthresh = val;
        self
    }

    /// Keyframe super-resolution q threshold.
    ///
    /// The q level threshold after which superres is used for key frames.
    /// Valid values are 1 to 63.
    ///
    /// Used only by AOM_SUPERRES_QTHRESH
    pub fn rc_superres_kf_qthresh(mut self, val: u32) -> Self {
        self.cfg.rc_superres_kf_qthresh = val;
        self
    }

    /// Rate control algorithm to use.
    ///
    /// Indicates whether the end usage of this stream is to be streamed over
    /// a bandwidth constrained link, indicating that Constant Bit Rate (CBR)
    /// mode should be used, or whether it will be played back on a high
    /// bandwidth link, as from a local disk, where higher variations in
    /// bitrate are acceptable.
    pub fn rc_end_usage(mut self, val: aom::aom_rc_mode::Type) -> Self {
        self.cfg.rc_end_usage = val;
        self
    }

    /// Two-pass stats buffer.
    ///
    /// A buffer containing all of the stats packets produced in the first
    /// pass, concatenated.
    pub fn rc_twopass_stats_in(mut self, val: aom_fixed_buf_t) -> Self {
        self.cfg.rc_twopass_stats_in = val;
        self
    }

    /// first pass mb stats buffer.
    ///
    /// A buffer containing all of the first pass mb stats packets produced
    /// in the first pass, concatenated.
    pub fn rc_firstpass_mb_stats_in(mut self, val: aom_fixed_buf_t) -> Self {
        self.cfg.rc_firstpass_mb_stats_in = val;
        self
    }

    /// Target data rate
    ///
    /// Target bitrate to use for this stream, in kilobits per second.
    pub fn rc_target_bitrate(mut self, val: u32) -> Self {
        self.cfg.rc_target_bitrate = val;
        self
    }
}

/// # Quantizer settings
impl AV1EncoderConfig {
    /// Minimum (Best Quality) Quantizer
    ///
    /// The quantizer is the most direct control over the quality of the
    /// encoded image. The range of valid values for the quantizer is codec
    /// specific. Consult the documentation for the codec to determine the
    /// values to use. To determine the range programmatically, call
    /// aom_codec_enc_config_default() with a usage value of 0.
    pub fn rc_min_quantizer(mut self, val: u32) -> Self {
        self.cfg.rc_min_quantizer = val;
        self
    }

    /// Maximum (Worst Quality) Quantizer
    ///
    /// The quantizer is the most direct control over the quality of the
    /// encoded image. The range of valid values for the quantizer is codec
    /// specific. Consult the documentation for the codec to determine the
    /// values to use. To determine the range programmatically, call
    /// aom_codec_enc_config_default() with a usage value of 0.
    pub fn rc_max_quantizer(mut self, val: u32) -> Self {
        self.cfg.rc_max_quantizer = val;
        self
    }
}

/// # Bitrate tolerance
impl AV1EncoderConfig {
    /// Rate control adaptation undershoot control
    ///
    /// This value, controls the tolerance of the VBR algorithm to undershoot
    /// and is used as a trigger threshold for more aggressive adaptation of Q.
    ///
    /// Valid values in the range 0-100.
    pub fn rc_undershoot_pct(mut self, val: u32) -> Self {
        self.cfg.rc_undershoot_pct = val;
        self
    }

    /// Rate control adaptation overshoot control
    ///
    /// This value, controls the tolerance of the VBR algorithm to overshoot
    /// and is used as a trigger threshold for more aggressive adaptation of Q.
    ///
    /// Valid values in the range 0-100.
    pub fn rc_overshoot_pct(mut self, val: u32) -> Self {
        self.cfg.rc_overshoot_pct = val;
        self
    }
}

/// # Decoder buffer model parameters
impl AV1EncoderConfig {
    /// Decoder Buffer Size
    ///
    /// This value indicates the amount of data that may be buffered by the
    /// decoding application. Note that this value is expressed in units of
    /// time (milliseconds). For example, a value of 5000 indicates that the
    /// client will buffer (at least) 5000ms worth of encoded data. Use the
    /// target bitrate (#rc_target_bitrate) to convert to bits/bytes, if
    /// necessary.
    pub fn rc_buf_sz(mut self, val: u32) -> Self {
        self.cfg.rc_buf_sz = val;
        self
    }

    /// Decoder Buffer Initial Size
    ///
    /// This value indicates the amount of data that will be buffered by the
    /// decoding application prior to beginning playback. This value is
    /// expressed in units of time (milliseconds). Use the target bitrate
    /// (#rc_target_bitrate) to convert to bits/bytes, if necessary.
    pub fn rc_buf_initial_sz(mut self, val: u32) -> Self {
        self.cfg.rc_buf_initial_sz = val;
        self
    }

    /// Decoder Buffer Optimal Size
    ///
    /// This value indicates the amount of data that the encoder should try
    /// to maintain in the decoder's buffer. This value is expressed in units
    /// of time (milliseconds). Use the target bitrate (#rc_target_bitrate)
    /// to convert to bits/bytes, if necessary.
    pub fn rc_buf_optimal_sz(mut self, val: u32) -> Self {
        self.cfg.rc_buf_optimal_sz = val;
        self
    }
}

/// # 2 pass rate control prameters
impl AV1EncoderConfig {
    /// Two-pass mode CBR/VBR bias
    ///
    /// Bias, expressed on a scale of 0 to 100, for determining target size
    /// for the current frame. The value 0 indicates the optimal CBR mode
    /// value should be used. The value 100 indicates the optimal VBR mode
    /// value should be used. Values in between indicate which way the
    /// encoder should "lean."
    pub fn rc_2pass_vbr_bias_pct(mut self, val: u32) -> Self {
        self.cfg.rc_2pass_vbr_bias_pct = val;
        self
    }

    /// Two-pass mode per-GOP minimum bitrate
    ///
    /// This value, expressed as a percentage of the target bitrate, indicates
    /// the minimum bitrate to be used for a single GOP (aka "section")
    pub fn rc_2pass_vbr_minsection_pct(mut self, val: u32) -> Self {
        self.cfg.rc_2pass_vbr_minsection_pct = val;
        self
    }

    /// Two-pass mode per-GOP maximum bitrate
    ///
    /// This value, expressed as a percentage of the target bitrate, indicates
    /// the maximum bitrate to be used for a single GOP (aka "section")
    pub fn rc_2pass_vbr_maxsection_pct(mut self, val: u32) -> Self {
        self.cfg.rc_2pass_vbr_maxsection_pct = val;
        self
    }
}

/// # Keyframing settings (kf)
impl AV1EncoderConfig {
    /// Option to enable forward reference key frame
    pub fn fwd_kf_enabled(mut self, val: bool) -> Self {
        self.cfg.fwd_kf_enabled = val as i32;
        self
    }

    /// Keyframe placement mode
    ///
    /// This value indicates whether the encoder should place keyframes at a
    /// fixed interval, or determine the optimal placement automatically
    /// (as governed by the `kf_min_dist` and `kf_max_dist` parameters)
    pub fn kf_mode(mut self, val: aom::aom_kf_mode::Type) -> Self {
        self.cfg.kf_mode = val;
        self
    }

    /// Keyframe minimum interval
    ///
    /// This value, expressed as a number of frames, prevents the encoder from
    /// placing a keyframe nearer than `kf_min_dist` to the previous keyframe. At
    /// least `kf_min_dist` frames non-keyframes will be coded before the next
    /// keyframe. Set `kf_min_dist` equal to `kf_max_dist` for a fixed interval.
    pub fn kf_min_dist(mut self, val: u32) -> Self {
        self.cfg.kf_min_dist = val;
        self
    }

    /// Keyframe maximum interval
    ///
    /// This value, expressed as a number of frames, forces the encoder to code
    /// a keyframe if one has not been coded in the last `kf_max_dist` frames.
    /// A value of `0` implies all frames will be keyframes. Set `kf_min_dist`
    /// equal to kf_max_dist for a fixed interval.
    pub fn kf_max_dist(mut self, val: u32) -> Self {
        self.cfg.kf_max_dist = val;
        self
    }

    /// S-Frame interval
    ///
    /// This value, expressed as a number of frames, forces the encoder to code
    /// an S-Frame every `sframe_dist` frames.
    pub fn sframe_dist(mut self, val: u32) -> Self {
        self.cfg.sframe_dist = val;
        self
    }

    /// S-Frame insertion mode
    ///
    /// This value must be set to 1 or 2, and tells the encoder how to insert
    /// S-Frames. It will only have an effect if `sframe_dist != 0`.
    ///
    /// If altref is enabled:
    ///   - `sframe_mode == 1`: The considered frame will be made into an
    ///     S-Frame only if it is an altref frame
    ///   - `sframe_mode == 2`: The next altref frame will be made into an
    ///     S-Frame.
    ///
    /// Otherwise: the considered frame will be made into an S-Frame.
    pub fn sframe_mode(mut self, val: u32) -> Self {
        self.cfg.sframe_mode = val;
        self
    }

    /// Tile coding mode
    ///
    /// This value indicates the tile coding mode.
    /// One of either `Normal` or `LargeScale`.
    pub fn large_scale_tile(mut self, val: TileCodingMode) -> Self {
        self.cfg.large_scale_tile = val as u32;
        self
    }

    /// Monochrome mode
    ///
    /// If this is `true`, the encoder will generate a monochrome stream
    /// with no chroma planes.
    pub fn monochrome(mut self, val: bool) -> Self {
        self.cfg.monochrome = val as u32;
        self
    }

    /// full_still_picture_hdr
    ///
    /// If this is nonzero, the encoder will generate a full header
    /// (reduced_still_picture_header is set to 0 in the sequence header OBU) even
    /// for still picture encoding. If this is zero (the default), a reduced
    /// header (reduced_still_picture_header is set to 1 in the sequence header
    /// OBU) is used for still picture encoding. This flag has no effect when a
    /// regular video with more than a single frame is encoded.
    pub fn full_still_picture_hdr(mut self, val: u32) -> Self {
        self.cfg.full_still_picture_hdr = val;
        self
    }

    /// Bitstream syntax mode
    ///
    /// This value indicates the bitstream syntax mode.
    /// - `false` indicates bitstream is saved as Section 5 bitstream.
    /// - `true` indicates the bitstream is saved in Annex-B format.
    pub fn save_as_annexb(mut self, val: bool) -> Self {
        self.cfg.save_as_annexb = val as u32;
        self
    }

    /// Number of explicit tile widths specified
    ///
    /// This value indicates the number of tile widths specified
    /// A value of 0 implies no tile widths are specified.
    /// Tile widths are given in the array tile_widths[]
    pub fn tile_width_count(mut self, val: i32) -> Self {
        self.cfg.tile_width_count = val;
        self
    }

    /// Number of explicit tile heights specified
    ///
    /// This value indicates the number of tile heights specified
    /// A value of 0 implies no tile heights are specified.
    /// Tile heights are given in the array tile_heights[]
    pub fn tile_height_count(mut self, val: i32) -> Self {
        self.cfg.tile_height_count = val;
        self
    }

    /// Array of specified tile widths
    ///
    /// This array specifies tile widths (and may be empty)
    /// The number of widths specified is given by tile_width_count
    pub fn tile_widths(mut self, val: [i32; MAX_TILE_WIDTHS as usize]) -> Self {
        self.cfg.tile_widths = val;
        self
    }

    /// Array of specified tile heights
    ///
    /// This array specifies tile heights (and may be empty)
    /// The number of heights specified is given by tile_height_count
    pub fn tile_heights(mut self, val: [i32; MAX_TILE_HEIGHTS as usize]) -> Self {
        self.cfg.tile_heights = val;
        self
    }

    /// Whether encoder should use fixed QP offsets.
    ///
    /// If `true`, encoder will use fixed QP offsets for frames
    /// at different levels of the pyramid.
    /// If `false`, encoder will NOT use fixed QP offsets.
    /// Note: This option is only relevant for --end-usage=q.
    pub fn use_fixed_qp_offsets(mut self, val: bool) -> Self {
        self.cfg.use_fixed_qp_offsets = val as u32;
        self
    }

    #[deprecated(since = "0.3.1", note = "DO NOT USE. To be removed in libaom v4.0.0")]
    pub fn fixed_qp_offsets(mut self, val: [i32; 5]) -> Self {
        self.cfg.fixed_qp_offsets = val;
        self
    }

    /// Options defined per config file
    pub fn encoder_cfg(mut self, val: cfg_options_t) -> Self {
        self.cfg.encoder_cfg = val;
        self
    }
}

impl Deref for AV1EncoderConfig {
    type Target = aom_codec_enc_cfg;

    fn deref(&self) -> &Self::Target {
        &self.cfg
    }
}

impl DerefMut for AV1EncoderConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.cfg
    }
}

/// Tile coding mode
#[repr(u32)]
pub enum TileCodingMode {
    /// Normal non-large-scale tile coding
    Normal = 0,
    /// Large-scale tile coding
    LargeScale = 1,
}
