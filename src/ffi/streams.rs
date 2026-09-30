use std::io::{self, Read, Write};
use std::panic::{catch_unwind, AssertUnwindSafe};

pub struct RustReader<'a> {
	inner: &'a mut dyn Read,
	error: Option<io::Error>,
}

pub struct RustWriter<'a> {
	inner: &'a mut dyn Write,
	error: Option<io::Error>,
}

pub(crate) fn with_reader<T>(reader: &mut dyn Read, call: impl for<'a> FnOnce(&mut RustReader<'a>) -> T) -> io::Result<T> {
	let mut stream = RustReader { inner: reader, error: None };
	let result = call(&mut stream);
	match stream.error {
		Some(error) => Err(error),
		None => Ok(result),
	}
}

pub(crate) fn with_writer<T>(writer: &mut dyn Write, call: impl for<'a> FnOnce(&mut RustWriter<'a>) -> T) -> io::Result<T> {
	let mut stream = RustWriter { inner: writer, error: None };
	let result = call(&mut stream);
	match stream.error {
		Some(error) => Err(error),
		None => Ok(result),
	}
}

// Unwinding callbacks become errors before returning to C++; panic=abort still aborts.
pub(crate) fn stream_io<T>(call: impl FnOnce() -> io::Result<T>) -> io::Result<T> {
	catch_unwind(AssertUnwindSafe(call)).unwrap_or_else(|_| Err(io::Error::other("stream callback panicked")))
}

pub fn rust_reader_read(reader: &mut RustReader<'_>, buf: &mut [u8]) -> usize {
	if reader.error.is_some() {
		return 0;
	}
	let result = stream_io(|| loop {
		match reader.inner.read(buf) {
			Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
			result => break result,
		}
	});
	match result {
		Ok(count) if count <= buf.len() => count,
		Ok(_) => {
			reader.error = Some(io::Error::new(io::ErrorKind::InvalidData, "reader returned more bytes than requested"));
			0
		}
		Err(error) => {
			reader.error = Some(error);
			0
		}
	}
}

pub fn rust_writer_write(writer: &mut RustWriter<'_>, buf: &[u8]) -> usize {
	if writer.error.is_some() {
		return 0;
	}
	match stream_io(|| writer.inner.write_all(buf)) {
		Ok(()) => buf.len(),
		Err(error) => {
			writer.error = Some(error);
			0
		}
	}
}
