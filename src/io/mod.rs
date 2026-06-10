//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Stream I/O (`wxInputStream`, `wxOutputStream`, …).

mod buffered_input_stream;
mod buffered_output_stream;
mod file_input_stream;
mod ffile_input_stream;
mod ffile_output_stream;
mod file_output_stream;
mod filter_input_stream;
mod filter_output_stream;
mod memory_input_stream;
mod memory_stream;
mod stream;
mod stream_base;
mod stream_error;
mod counting_output_stream;
mod counting_input_stream;
mod tee_input_stream;
mod stream_buffer;
mod text_stream;
mod wx_ffile;
mod zlib_input_stream;
mod zlib_output_stream;

pub use buffered_input_stream::BufferedInputStream;
pub use buffered_output_stream::BufferedOutputStream;
pub use ffile_input_stream::FFileInputStream;
pub use ffile_output_stream::FFileOutputStream;
pub use file_input_stream::FileInputStream;
pub use file_output_stream::FileOutputStream;
pub use filter_input_stream::FilterInputStream;
pub use filter_output_stream::FilterOutputStream;
pub use memory_input_stream::MemoryInputStream;
pub use memory_stream::MemoryOutputStream;
pub use stream::{WxInputStream, WxOutputStream};
pub use stream_base::{InputStreamExt, OutputStreamExt, StreamBase};
pub use stream_error::StreamError;
pub use counting_output_stream::CountingOutputStream;
pub use counting_input_stream::CountingInputStream;
pub use tee_input_stream::TeeInputStream;
pub use stream_buffer::StreamBuffer;
pub use text_stream::{TextInputStream, TextOutputStream};
pub use wx_ffile::{FileOffset, WxFFile};
pub use zlib_input_stream::ZlibInputStream;
pub use zlib_output_stream::ZlibOutputStream;
