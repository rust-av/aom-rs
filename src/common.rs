use crate::ffi::aom::*;
use std::ffi::CStr;

pub trait AOMCodec {
    fn get_context(&mut self) -> &mut aom_codec_ctx;

    /// Return a human-readable representation of the last error occurred
    fn error_to_str(&mut self) -> String {
        unsafe {
            let c_str = aom_codec_error(self.get_context());

            CStr::from_ptr(c_str).to_string_lossy().into_owned()
        }
    }
}
