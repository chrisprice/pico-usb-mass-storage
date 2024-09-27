//! ## Lifted from "A Rust crate & cli to convert bytes into human-readable values."

//! It can return either KiB/MiB/GiB/TiB or KB/MB/GB/TB by disabling the `si-units` feature.
//!
//! > 1 KiB = 1024 B, 1 KB = 1000 B
//!
//! It supports from 0 bytes to several yottabytes (I cannot tell how many because I have to use `u128`s
//! to fit a single YB)
//!
//! For more info, check out the [README.md](https://sr.ht/~f9/human_bytes)

#[cfg(not(feature = "si-units"))]
// Just be future-proof
const SUFFIX: [&str; 9] = ["B", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];

#[cfg(feature = "si-units")]
// Just be future-proof
const SUFFIX: [&str; 9] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB", "ZiB", "YiB"];

#[cfg(not(feature = "si-units"))]
const UNIT: f64 = 1000.0;

#[cfg(feature = "si-units")]
const UNIT: f64 = 1024.0;

pub fn write<T: Into<f64>>(mut writer: impl core::fmt::Write, bytes: T) {
    let mut size = bytes.into();
    let mut base = 0;
    while size >= UNIT {
        base += 1;
        size /= UNIT;
    }
    let _ = write!(writer, "{:.1}", size);
    let _ = writer.write_str(SUFFIX[base]);
}
