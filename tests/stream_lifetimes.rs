#![cfg(not(target_arch = "wasm32"))]

use std::path::PathBuf;
use std::process::Command;

struct CompileDirectory(PathBuf);

impl Drop for CompileDirectory {
	fn drop(&mut self) {
		let _ = std::fs::remove_dir_all(&self.0);
	}
}

#[test]
fn stream_wrappers_cannot_escape_or_exchange_borrows_between_scopes() {
	let directory = CompileDirectory(std::env::temp_dir().join(format!("cadrum-stream-lifetimes-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("clock").as_nanos())));
	std::fs::create_dir(&directory.0).expect("isolated compile directory");
	let module = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/ffi/streams.rs");
	let cases = [
		("valid", "let mut input = &b\"input\"[..]; streams::with_reader(&mut input, |reader| streams::rust_reader_read(reader, &mut [0; 2])).unwrap(); let mut output = Vec::new(); streams::with_writer(&mut output, |writer| streams::rust_writer_write(writer, b\"output\")).unwrap();", true),
		("escape_reader", "let mut input = &b\"input\"[..]; let _escaped = streams::with_reader(&mut input, |reader| reader);", false),
		("escape_writer", "let mut output = Vec::new(); let _escaped = streams::with_writer(&mut output, |writer| writer);", false),
		("swap_readers", "let mut input = &b\"input\"[..]; streams::with_reader(&mut input, |outer| { let mut inner_input = &b\"inner\"[..]; streams::with_reader(&mut inner_input, |inner| std::mem::swap(outer, inner)) });", false),
		("swap_writers", "let mut output = Vec::new(); streams::with_writer(&mut output, |outer| { let mut inner_output = Vec::new(); streams::with_writer(&mut inner_output, |inner| std::mem::swap(outer, inner)) });", false),
	];
	for (name, body, should_compile) in cases {
		let path = directory.0.join(format!("{name}.rs"));
		std::fs::write(&path, format!("#[path = {module:?}] mod streams;\nfn main() {{ {body} }}")).expect("probe source");
		let result = Command::new("rustc").args(["--edition=2021", "--emit=metadata", "--error-format=json", "-A", "dead_code"]).arg(&path).arg("--out-dir").arg(&directory.0).output().expect("compile probe");
		let diagnostic = String::from_utf8_lossy(&result.stderr);
		assert_eq!(result.status.success(), should_compile, "{name}: {diagnostic}");
		if !should_compile {
			assert!(diagnostic.contains("lifetime may not live long enough") || diagnostic.contains("borrowed data escapes"), "{name}: unexpected compilation failure: {diagnostic}");
		}
	}
}
