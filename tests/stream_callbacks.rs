#[path = "../src/ffi/streams.rs"]
mod streams;

use std::io::{self, Read, Write};

const TEST_OS_ERROR: i32 = 13;

struct FailingStream {
	calls: usize,
}

impl Read for FailingStream {
	fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
		self.calls += 1;
		Err(io::Error::from_raw_os_error(TEST_OS_ERROR))
	}
}

impl Write for FailingStream {
	fn write(&mut self, _: &[u8]) -> io::Result<usize> {
		self.calls += 1;
		Err(io::Error::from_raw_os_error(TEST_OS_ERROR))
	}

	fn flush(&mut self) -> io::Result<()> {
		panic!("the callback adapter must not flush the caller's writer");
	}
}

#[test]
fn failed_callbacks_retain_the_first_error_without_retrying_the_stream() {
	let mut reader = FailingStream { calls: 0 };
	let error = streams::with_reader(&mut reader, |stream| {
		assert_eq!(streams::rust_reader_read(stream, &mut [0; 8]), 0);
		assert_eq!(streams::rust_reader_read(stream, &mut [0; 8]), 0);
	})
	.expect_err("read failure");
	assert_eq!(reader.calls, 1);
	assert_eq!(error.raw_os_error(), Some(TEST_OS_ERROR));
	let mut writer = FailingStream { calls: 0 };
	let error = streams::with_writer(&mut writer, |stream| {
		assert_eq!(streams::rust_writer_write(stream, b"first"), 0);
		assert_eq!(streams::rust_writer_write(stream, b"second"), 0);
	})
	.expect_err("write failure");
	assert_eq!(writer.calls, 1);
	assert_eq!(error.raw_os_error(), Some(TEST_OS_ERROR));
}

#[test]
fn eof_has_no_error_and_impossible_read_counts_never_reach_cpp() {
	let mut empty = &b""[..];
	assert_eq!(streams::with_reader(&mut empty, |stream| streams::rust_reader_read(stream, &mut [0; 8])).expect("clean EOF"), 0);
	struct InvalidCount;
	impl Read for InvalidCount {
		fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
			Ok(buffer.len() + 1)
		}
	}
	let error = streams::with_reader(&mut InvalidCount, |stream| streams::rust_reader_read(stream, &mut [0; 8])).expect_err("invalid count");
	assert_eq!(error.kind(), io::ErrorKind::InvalidData);
}
