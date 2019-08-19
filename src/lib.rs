#![allow(non_upper_case_globals)]

extern crate aom_sys as ffi;
extern crate av_data as data;

#[cfg(feature = "codec-trait")]
extern crate av_codec as codec;

pub mod common;
pub mod decoder;
pub mod encoder;
