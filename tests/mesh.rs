//! Mesh output tests: SVG (`Scene2D::write_svg`), GLB (`Mesh::write_gltf_binary`),
//! and PNG (`Scene2D::write_png`, gated on the `png` feature).
//!
//! Each test writes its artifact under `out/<test-name>.<ext>` for visual
//! inspection. Run PNG tests with the `png` feature (on by default).

use cadrum::{DVec3, Solid, Tessellation};

fn dvec3(x: f64, y: f64, z: f64) -> DVec3 {
	DVec3::new(x, y, z)
}

// ==================== Vertex normals (Mesh::normals) ====================

/// 頂点法線が三角形の平均ではなく元の曲面から来ていることを検証する。
/// 原点中心・半径 r の球なら、法線は必ず中心からの単位ベクトルに一致する。
/// 三角形法線の平均だと deflection のオーダー (1e-3 級) でずれるので、
/// この許容差では通らない。
#[test]
fn sphere_normals_come_from_the_surface() {
	let tess = Tessellation { deflection_linear: 0.1, relative_linear: false, ..Default::default() };
	let mesh = Solid::mesh(&[Solid::sphere(5.0)], tess).unwrap();

	assert_eq!(mesh.normals.len(), mesh.vertices.len(), "one normal per vertex");
	assert!(!mesh.normals.is_empty(), "sphere must produce vertices");

	for (v, n) in mesh.vertices.iter().zip(&mesh.normals) {
		assert!((n.length() - 1.0).abs() < 1e-6, "normal must be unit length at {v:?}: {n:?}");
		let radial = v.normalize();
		assert!(n.dot(radial) > 1.0 - 1e-6, "normal must be the exact surface normal at {v:?}: got {n:?}, expected {radial:?}");
	}
}

#[test]
fn mesh_rejects_non_finite_or_non_positive_deflection() {
	let solid = Solid::sphere(5.0);
	for deflection_linear in [f64::NAN, f64::INFINITY, 0.0, -0.1] {
		let result = Solid::mesh([&solid], Tessellation { deflection_linear, ..Default::default() });
		assert!(matches!(result, Err(cadrum::Error::InvalidInput(_))), "linear deflection {deflection_linear:?} must be rejected at the Rust boundary");
	}
	for deflection_angular in [f64::NAN, f64::INFINITY, 0.0, -0.1] {
		let result = Solid::mesh([&solid], Tessellation { deflection_angular, ..Default::default() });
		assert!(matches!(result, Err(cadrum::Error::InvalidInput(_))), "angular deflection {deflection_angular:?} must be rejected at the Rust boundary");
	}
}

#[test]
fn face_mesh_rejects_duplicate_face_selection() {
	let solid = Solid::cube(DVec3::ZERO, DVec3::splat(5.0));
	let result = solid.mesh_face_chunks(&[0, 0], Tessellation::default());
	assert!(matches!(result, Err(cadrum::Error::InvalidInput(_))));
}

// ==================== SVG (Scene2D::write_svg) ====================

mod svg {
	use super::*;

	fn svg_string(shape: &[Solid], direction: DVec3, tol: f64) -> String {
		let mut buf = Vec::new();
		// Pick a sensible up for each view: Z-up for oblique/side views,
		// Y-up when looking straight down Z (view_dir ‖ Z would make Z-up degenerate).
		let up = if direction.normalize().dot(DVec3::Z).abs() > 0.999 { DVec3::Y } else { DVec3::Z };
		cadrum::Solid::mesh(shape, cadrum::Tessellation { deflection_linear: tol, relative_linear: false, ..Default::default() }).and_then(|m| m.scene(cadrum::SceneOption { view: direction, up, ..Default::default() }).write_svg(&mut buf)).unwrap();
		String::from_utf8(buf).unwrap()
	}

	fn write(name: &str, svg: &str) {
		std::fs::create_dir_all("out").unwrap();
		std::fs::write(format!("out/{name}.svg"), svg).unwrap();
	}

	#[test]
	fn box_isometric() {
		let shape = [Solid::cube(DVec3::ZERO, DVec3::splat(10.0))];
		let svg = svg_string(&shape, dvec3(1.0, 1.0, 1.0).normalize(), 0.1);

		assert!(svg.starts_with("<svg"), "should start with <svg tag");
		assert!(svg.contains("</svg>"), "should end with </svg>");
		assert!(svg.contains("<polyline"), "should contain polyline elements");
		assert!(svg.contains("viewBox"), "should contain viewBox");
		let svg_tag = &svg[..svg.find('>').unwrap()];
		assert!(!svg_tag.contains(" width="), "should not contain fixed width (responsive)");

		write("svg_box_isometric", &svg);
		println!("SVG length: {} bytes", svg.len());
	}

	#[test]
	fn cylinder() {
		let shape = [Solid::cylinder(5.0, DVec3::Z * 10.0)];
		let svg = svg_string(&shape, dvec3(1.0, 0.5, 0.3).normalize(), 0.1);

		assert!(svg.contains("<polyline"));

		write("svg_cylinder", &svg);
	}

	#[test]
	fn has_hidden_lines() {
		let a = [Solid::cube(DVec3::ZERO, DVec3::splat(10.0))];
		let b = [Solid::cube(DVec3::ZERO, DVec3::splat(10.0)).translate(dvec3(5.0, 5.0, 0.0))];
		let shape: Vec<Solid> = (&a[0] + &b[0]).build_vec().unwrap();
		let svg = svg_string(&shape, dvec3(1.0, 1.0, 1.0).normalize(), 0.1);

		assert!(svg.contains("#bbb"), "should contain hidden line color");

		write("svg_has_hidden_lines", &svg);
	}

	#[test]
	fn rotated_sphere_preserves_projected_coverage() {
		fn coverage(svg: &str) -> f64 {
			svg.split("<polygon points=\"")
				.skip(1)
				.map(|part| {
					let points = part
						.split('"')
						.next()
						.unwrap()
						.split_whitespace()
						.map(|point| {
							let (x, y) = point.split_once(',').unwrap();
							(x.parse::<f64>().unwrap(), y.parse::<f64>().unwrap())
						})
						.collect::<Vec<_>>();
					assert!(points.len() >= 3);
					points.iter().zip(points.iter().cycle().skip(1)).take(points.len()).map(|(a, b)| a.0 * b.1 - a.1 * b.0).sum::<f64>().abs() * 0.5
				})
				.sum()
		}
		let shape = [Solid::sphere(5.0)];
		let original = svg_string(&shape, DVec3::X, 0.1);
		let rotated = svg_string(&shape.map(|s| s.rotate_y(std::f64::consts::PI)), DVec3::X, 0.1);
		let expected = std::f64::consts::PI * 25.0;
		let areas = [coverage(&original), coverage(&rotated)];
		for area in areas {
			assert!((area - expected).abs() / expected < 0.01, "sphere projection must cover its disk: {area}");
		}
		// Occlusion clipping may split equivalent projections into different polygon counts.
		assert!((areas[0] - areas[1]).abs() / expected < 0.001);
		write("svg_rotated_sphere_coverage", &original);
		write("svg_rotated_sphere_coverage_rotated", &rotated);
	}
}

// ==================== GLB (Mesh::write_gltf_binary) ====================

mod glb {
	use super::*;

	fn u32le(bytes: &[u8], off: usize) -> usize {
		u32::from_le_bytes(bytes[off..off + 4].try_into().unwrap()) as usize
	}

	/// Extract the JSON chunk of a GLB as a string.
	fn glb_json(glb: &[u8]) -> &str {
		let json_len = u32le(glb, 12);
		std::str::from_utf8(&glb[20..20 + json_len]).expect("JSON utf-8")
	}

	/// Mesh `solids`, serialize to GLB, write `out/<name>.glb`, and return the bytes.
	fn glb_to_file(solids: &[Solid], tess: Tessellation, name: &str) -> Vec<u8> {
		let mut buf = Vec::new();
		Solid::mesh(solids, tess).unwrap().write_gltf_binary(&mut buf).expect("glb write");
		std::fs::create_dir_all("out").unwrap();
		std::fs::write(format!("out/{name}.glb"), &buf).unwrap();
		buf
	}

	#[test]
	fn header_and_chunks() {
		let glb = glb_to_file(&[Solid::cube(DVec3::ZERO, DVec3::splat(10.0))], Default::default(), "glb_header_and_chunks");
		assert!(glb.len() >= 20, "GLB must hold a 12-byte header + JSON chunk header");

		// 12-byte header
		assert_eq!(&glb[0..4], b"glTF", "magic");
		assert_eq!(u32le(&glb, 4), 2, "version 2");
		assert_eq!(u32le(&glb, 8), glb.len(), "header length must equal file size");

		// JSON chunk
		let json_len = u32le(&glb, 12);
		assert_eq!(&glb[16..20], b"JSON", "chunk 0 type");
		assert_eq!(json_len % 4, 0, "JSON chunk 4-byte aligned");
		let json_end = 20 + json_len;
		let json = std::str::from_utf8(&glb[20..json_end]).expect("JSON utf-8");

		// BIN chunk
		let bin_len = u32le(&glb, json_end);
		assert_eq!(&glb[json_end + 4..json_end + 8], b"BIN\0", "chunk 1 type");
		assert_eq!(bin_len % 4, 0, "BIN chunk 4-byte aligned");
		assert!(bin_len > 0, "geometry buffer non-empty");
		assert_eq!(json_end + 8 + bin_len, glb.len(), "BIN chunk reaches EOF");

		// JSON content
		assert!(json.contains(r#""version":"2.0""#), "asset version");
		assert!(json.contains(r#""mode":4"#), "triangle primitive present");
		assert!(json.contains(r#""mode":1"#), "edge LINES primitive present");
		assert!(json.contains(r#""cadrum":"edges""#), "edge extras marker present");
		assert!(json.contains(r#""NORMAL":"#), "triangles must carry vertex normals");
	}

	/// A small mesh (≤ 65535 vertices) stores indices as UNSIGNED_SHORT (5123);
	/// UNSIGNED_INT (5125) must not appear (issue #181).
	#[test]
	fn small_u16_indices() {
		let glb = glb_to_file(&[Solid::cube(DVec3::ZERO, DVec3::splat(10.0))], Default::default(), "glb_small_u16_indices");
		let json = glb_json(&glb);
		assert!(json.contains(r#""componentType":5123"#), "small mesh must use UNSIGNED_SHORT index accessors");
		assert!(!json.contains(r#""componentType":5125"#), "small mesh must not emit UNSIGNED_INT index accessors");
	}

	/// A mesh exceeding 65535 vertices falls back to UNSIGNED_INT (5125) (issue #181).
	#[test]
	fn large_u32_indices() {
		let mut mesh = Solid::mesh([&Solid::cube(DVec3::ZERO, DVec3::ONE)], Tessellation::default()).expect("seed mesh");
		mesh.vertices.clear();
		mesh.normals.clear();
		mesh.indices.clear();
		mesh.face_ids.clear();
		mesh.edges.clear();
		for triangle in 0..21_846 {
			let x = f64::from(triangle % 256) * 2.0;
			let y = f64::from(triangle / 256) * 2.0;
			let first = mesh.vertices.len();
			mesh.vertices.extend([DVec3::new(x, y, 0.0), DVec3::new(x + 1.0, y, 0.0), DVec3::new(x, y + 1.0, 0.0)]);
			mesh.normals.extend([DVec3::Z; 3]);
			mesh.indices.extend([first, first + 1, first + 2]);
			mesh.face_ids.push(0);
		}
		assert!(mesh.vertices.len() > u16::MAX as usize);

		let mut glb = Vec::new();
		mesh.write_gltf_binary(&mut glb).expect("serialize large mesh");
		assert!(glb_json(&glb).contains(r#""componentType":5125"#), "mesh with >65535 vertices must use UNSIGNED_INT index accessors");
	}
}

// ==================== PNG (Scene2D::write_png via tiny-skia) ====================

#[cfg(feature = "png")]
mod png {
	use super::*;

	/// PNG signature: 89 50 4E 47 0D 0A 1A 0A
	const PNG_MAGIC: &[u8; 8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

	/// Read width/height from the IHDR chunk (bytes 16..24 of a PNG file).
	fn png_dimensions(buf: &[u8]) -> (u32, u32) {
		let w = u32::from_be_bytes(buf[16..20].try_into().unwrap());
		let h = u32::from_be_bytes(buf[20..24].try_into().unwrap());
		(w, h)
	}

	fn write(name: &str, buf: &[u8]) {
		std::fs::create_dir_all("out").unwrap();
		std::fs::write(format!("out/{name}.png"), buf).unwrap();
	}

	#[test]
	fn cylinder_shaded() {
		let shape = [Solid::cylinder(5.0, DVec3::Z * 10.0)];
		let mesh = Solid::mesh(&shape, cadrum::Tessellation { deflection_linear: 0.1, relative_linear: false, ..Default::default() }).unwrap();
		let scene = mesh.scene(cadrum::SceneOption { view: dvec3(1.0, 0.5, 0.3).normalize(), shading: true, ..Default::default() });

		let mut buf = Vec::new();
		scene.write_png([400, 600], &mut buf).unwrap();

		assert_eq!(&buf[0..8], PNG_MAGIC);
		assert_eq!(png_dimensions(&buf), (400, 600));

		write("png_cylinder_shaded", &buf);
	}

	#[test]
	fn dimensions_are_exact() {
		// User-specified [width, height] must appear verbatim in the IHDR,
		// regardless of viewbox aspect (letterboxed when aspects differ).
		let shape = [Solid::cube(DVec3::ZERO, DVec3::new(50.0, 10.0, 10.0))];
		let mesh = Solid::mesh(&shape, cadrum::Tessellation { deflection_linear: 0.5, relative_linear: false, ..Default::default() }).unwrap();
		let scene = mesh.scene(cadrum::SceneOption { view: DVec3::Z, up: DVec3::Y, hidden_edges: false, shading: false });

		for (i, dims) in [[500, 500], [800, 200], [200, 800]].into_iter().enumerate() {
			let mut buf = Vec::new();
			scene.write_png(dims, &mut buf).unwrap();
			assert_eq!(png_dimensions(&buf), (dims[0] as u32, dims[1] as u32));
			if i == 0 {
				write("png_dimensions_are_exact", &buf);
			}
		}
	}
}
