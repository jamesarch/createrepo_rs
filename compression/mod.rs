pub mod gzip;
pub mod bzip2;
pub mod zstd;
pub mod xz;

pub use gzip::{compress as gzip_compress, decompress as gzip_decompress};
pub use bzip2::{compress as bzip2_compress, decompress as bzip2_decompress};
pub use zstd::{compress as zstd_compress, decompress as zstd_decompress};
pub use xz::{compress as xz_compress, decompress as xz_decompress};