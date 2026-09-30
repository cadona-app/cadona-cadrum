#![cfg(not(feature = "pure"))]

use cadrum::{Error, Solid};
use glam::DVec3;
use std::error::Error as _;
use std::io::{self, Cursor, Read, Write};

#[derive(Clone, Copy, Debug)]
enum Format {
	Step,
	Brep,
}

impl Format {
	fn write(self, solid: &Solid, writer: &mut impl Write) -> Result<(), Error> {
		match self {
			Self::Step => Solid::write_step([solid], writer),
			Self::Brep => Solid::write_brep([solid], writer),
		}
	}

	fn read(self, reader: &mut impl Read) -> Result<Vec<Solid>, Error> {
		match self {
			Self::Step => Solid::read_step(reader),
			Self::Brep => Solid::read_brep(reader),
		}
	}
}

#[derive(Debug)]
struct InjectedFailure;

impl std::fmt::Display for InjectedFailure {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		formatter.write_str("injected stream failure")
	}
}

impl std::error::Error for InjectedFailure {}

fn injected_failure() -> io::Error {
	io::Error::new(io::ErrorKind::PermissionDenied, InjectedFailure)
}

fn assert_preserved_io_error(error: Error) {
	let source = error.source().expect("retained error source").downcast_ref::<io::Error>().expect("original I/O error");
	assert_eq!(source.kind(), io::ErrorKind::PermissionDenied);
	assert!(source.get_ref().expect("custom source").is::<InjectedFailure>());
}

#[derive(Clone, Copy)]
enum WriteFault {
	None,
	After(usize),
	Zero,
	Flush,
	Panic,
	FlushPanic,
}

struct ChunkWriter {
	bytes: Vec<u8>,
	flushed: Option<Vec<u8>>,
	fault: WriteFault,
	interrupt_once: bool,
}

impl ChunkWriter {
	fn new(fault: WriteFault) -> Self {
		Self { bytes: Vec::new(), flushed: None, fault, interrupt_once: true }
	}
}

impl Write for ChunkWriter {
	fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
		if self.interrupt_once {
			self.interrupt_once = false;
			return Err(io::ErrorKind::Interrupted.into());
		}
		let remaining = match self.fault {
			WriteFault::After(limit) if self.bytes.len() >= limit => return Err(injected_failure()),
			WriteFault::After(limit) => limit - self.bytes.len(),
			WriteFault::Zero => return Ok(0),
			WriteFault::Panic => panic!("injected write panic"),
			_ => usize::MAX,
		};
		let count = bytes.len().min(7).min(remaining);
		self.bytes.extend_from_slice(&bytes[..count]);
		Ok(count)
	}

	fn flush(&mut self) -> io::Result<()> {
		match self.fault {
			WriteFault::Flush => Err(injected_failure()),
			WriteFault::FlushPanic => panic!("injected flush panic"),
			_ => {
				self.flushed = Some(self.bytes.clone());
				Ok(())
			}
		}
	}
}

struct ChunkReader<'a> {
	bytes: Cursor<&'a [u8]>,
	fail_after: Option<usize>,
	interrupt_once: bool,
}

impl Read for ChunkReader<'_> {
	fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
		if self.interrupt_once {
			self.interrupt_once = false;
			return Err(io::ErrorKind::Interrupted.into());
		}
		let remaining = match self.fail_after {
			Some(limit) if self.bytes.position() as usize >= limit => return Err(injected_failure()),
			Some(limit) => limit - self.bytes.position() as usize,
			None => usize::MAX,
		};
		let length = bytes.len().min(11).min(remaining);
		self.bytes.read(&mut bytes[..length])
	}
}

fn cube() -> Solid {
	let solid = Solid::cube(DVec3::ZERO, DVec3::new(2.0, 3.0, 4.0));
	#[cfg(feature = "color")]
	let solid = solid.color(cadrum::Color::parse("#ff0000").expect("red"));
	solid
}

#[test]
fn short_interrupted_streams_round_trip_and_flush_the_complete_output() {
	for format in [Format::Step, Format::Brep] {
		let solid = cube();
		let mut writer = ChunkWriter::new(WriteFault::None);
		format.write(&solid, &mut writer).expect("complete output");
		assert_eq!(writer.flushed.as_deref(), Some(writer.bytes.as_slice()));
		let mut reader = ChunkReader { bytes: Cursor::new(writer.bytes.as_slice()), fail_after: None, interrupt_once: true };
		let restored = format.read(&mut reader).expect("short interrupted reads");
		assert_eq!(restored.len(), 1);
		assert!((restored[0].volume() - 24.0).abs() < 1.0e-7);
		assert_eq!(restored[0].iter_face().count(), 6);
		#[cfg(feature = "color")]
		assert!(restored[0].colormap().values().any(|color| *color == cadrum::Color::parse("#ff0000").expect("red")));
	}
}

#[test]
fn failing_readers_preserve_the_original_error_instead_of_reporting_eof() {
	for format in [Format::Step, Format::Brep] {
		let mut bytes = Vec::new();
		format.write(&cube(), &mut bytes).expect("source geometry");
		let mut reader = ChunkReader { bytes: Cursor::new(bytes.as_slice()), fail_after: Some(32), interrupt_once: false };
		assert_preserved_io_error(format.read(&mut reader).err().expect("reader failure"));
		let invalid_geometry = format.read(&mut [].as_slice()).err().expect("empty geometry");
		assert!(!matches!(invalid_geometry, Error::StreamIo { .. }));
	}
}

#[test]
fn failed_writes_and_final_flushes_never_report_success() {
	for format in [Format::Step, Format::Brep] {
		for fault in [WriteFault::After(0), WriteFault::After(37), WriteFault::Flush] {
			let mut writer = ChunkWriter::new(fault);
			assert_preserved_io_error(format.write(&cube(), &mut writer).expect_err("writer failure"));
			assert!(writer.flushed.is_none());
		}
		let mut writer = ChunkWriter::new(WriteFault::Zero);
		let error = format.write(&cube(), &mut writer).expect_err("zero-progress write");
		assert!(matches!(error, Error::StreamIo { source, .. } if source.kind() == io::ErrorKind::WriteZero));
	}
}

#[test]
fn unwinding_stream_panics_become_errors_without_crossing_the_foreign_boundary() {
	struct PanickingReader;
	impl Read for PanickingReader {
		fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
			panic!("injected read panic");
		}
	}
	for format in [Format::Step, Format::Brep] {
		let error = format.read(&mut PanickingReader).err().expect("read panic");
		assert!(matches!(error, Error::StreamIo { source, .. } if source.to_string() == "stream callback panicked"));
		for fault in [WriteFault::Panic, WriteFault::FlushPanic] {
			let error = format.write(&cube(), &mut ChunkWriter::new(fault)).expect_err("write panic");
			assert!(matches!(error, Error::StreamIo { source, .. } if source.to_string() == "stream callback panicked"));
		}
	}
}

#[cfg(feature = "color")]
#[test]
fn color_trailer_write_failure_preserves_its_cause() {
	let solid = cube();
	let mut bytes = Vec::new();
	Format::Brep.write(&solid, &mut bytes).expect("reference output");
	let trailer = bytes.windows(4).rposition(|window| window == b"CDCL").expect("color trailer");
	let mut writer = ChunkWriter::new(WriteFault::After(trailer + 2));
	assert_preserved_io_error(Format::Brep.write(&solid, &mut writer).expect_err("trailer failure"));
}
