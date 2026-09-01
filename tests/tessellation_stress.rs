use std::collections::BTreeMap;
use std::f64::consts::PI;

use cadrum::{DVec3, MeshChunks, Solid, Tessellation};

const CORPUS_SEED: u64 = 0x8f43_15c2_a96e_7d01;
const MAX_CORPUS_TRIANGLES: usize = 500_000;
const MAX_CORPUS_VERTICES: usize = 350_000;
const MAX_CORPUS_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ExactPoint([u64; 3]);

impl ExactPoint {
	fn new(point: DVec3) -> Self {
		Self(point.to_array().map(|coordinate| if coordinate == 0.0 { 0 } else { coordinate.to_bits() }))
	}
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ExactSegment {
	first: ExactPoint,
	second: ExactPoint,
}

impl ExactSegment {
	fn new(first: DVec3, second: DVec3) -> Option<(Self, i32)> {
		let first = ExactPoint::new(first);
		let second = ExactPoint::new(second);
		match first.cmp(&second) {
			std::cmp::Ordering::Less => Some((Self { first, second }, 1)),
			std::cmp::Ordering::Greater => Some((Self { first: second, second: first }, -1)),
			std::cmp::Ordering::Equal => None,
		}
	}
}

#[derive(Clone, Copy, Debug)]
enum AnalyticPrimitive {
	Box { dimensions: DVec3 },
	Cylinder { radius: f64, height: f64 },
	Sphere { radius: f64 },
	Cone { base_radius: f64, tip_radius: f64, height: f64 },
	Torus { major_radius: f64, minor_radius: f64 },
}

impl AnalyticPrimitive {
	fn solid(self) -> Solid {
		match self {
			Self::Box { dimensions } => Solid::cube(-dimensions * 0.5, dimensions * 0.5),
			Self::Cylinder { radius, height } => Solid::cylinder(radius, DVec3::Z * height),
			Self::Sphere { radius } => Solid::sphere(radius),
			Self::Cone { base_radius, tip_radius, height } => Solid::cone(base_radius, tip_radius, DVec3::Z * height),
			Self::Torus { major_radius, minor_radius } => Solid::torus(major_radius, minor_radius, DVec3::Z),
		}
	}
}

#[derive(Clone, Copy, Debug)]
struct SeededGenerator(u64);

impl SeededGenerator {
	fn new(seed: u64) -> Self {
		Self(seed)
	}

	fn next_u64(&mut self) -> u64 {
		let mut value = self.0;
		value ^= value >> 12;
		value ^= value << 25;
		value ^= value >> 27;
		self.0 = value;
		value.wrapping_mul(0x2545_f491_4f6c_dd1d)
	}

	fn unit(&mut self) -> f64 {
		const DENOMINATOR: f64 = (1_u64 << 53) as f64;
		((self.next_u64() >> 11) as f64 + 0.5) / DENOMINATOR
	}

	fn range(&mut self, minimum: f64, maximum: f64) -> f64 {
		minimum + self.unit() * (maximum - minimum)
	}

	fn direction(&mut self) -> DVec3 {
		loop {
			let candidate = DVec3::new(self.range(-1.0, 1.0), self.range(-1.0, 1.0), self.range(-1.0, 1.0));
			if let Some(direction) = candidate.try_normalize() {
				return direction;
			}
		}
	}
}

fn corpus_options(parallel: bool) -> Tessellation {
	Tessellation { deflection_linear: 0.01, deflection_angular: 0.25, relative_linear: true, include_edges: true, parallel }
}

fn seeded_primitives(generator: &mut SeededGenerator) -> [AnalyticPrimitive; 5] {
	[AnalyticPrimitive::Box { dimensions: DVec3::new(generator.range(2.5, 9.0), generator.range(3.0, 11.0), generator.range(1.5, 7.0)) }, AnalyticPrimitive::Cylinder { radius: generator.range(1.5, 5.0), height: generator.range(4.0, 13.0) }, AnalyticPrimitive::Sphere { radius: generator.range(2.0, 6.0) }, AnalyticPrimitive::Cone { base_radius: generator.range(3.0, 6.0), tip_radius: generator.range(0.7, 2.0), height: generator.range(5.0, 14.0) }, AnalyticPrimitive::Torus { major_radius: generator.range(5.0, 9.0), minor_radius: generator.range(0.8, 2.0) }]
}

fn placed_corpus() -> Vec<Solid> {
	let mut generator = SeededGenerator::new(CORPUS_SEED);
	let primitives = seeded_primitives(&mut generator);
	let scales = [0.001, 0.1, 1.0, 10.0, 1_000.0];
	let translation_scales = [100.0, 1.0e8, 10_000.0, 1.0e6, 1.0e8];

	primitives
		.into_iter()
		.enumerate()
		.map(|(index, primitive)| {
			let axis = generator.direction();
			let angle = generator.range(-PI, PI);
			let translation = generator.direction() * translation_scales[index];
			primitive.solid().scale(DVec3::ZERO, scales[index]).located(axis, angle, translation)
		})
		.collect()
}

fn assert_valid_solid(name: &str, solid: &Solid) {
	let validation = solid.validate().unwrap_or_else(|error| panic!("{name}: validate exact fixture: {error:?}"));
	assert!(validation.valid, "{name}: seeded fixture is not a valid exact solid: {validation:?}");
}

fn assert_finite_nondegenerate_watertight_oriented(name: &str, chunks: &MeshChunks) {
	let mut edge_uses = BTreeMap::<ExactSegment, (usize, i32)>::new();
	let mut triangle_count = 0;
	for face in &chunks.faces {
		assert_eq!(face.vertices.len(), face.normals.len(), "{name}: face {} has mismatched positions and normals", face.face_index);
		assert!(face.indices.len().is_multiple_of(3), "{name}: face {} has a partial triangle", face.face_index);
		assert!(face.vertices.iter().all(|position| position.is_finite()), "{name}: face {} has a non-finite position", face.face_index);
		assert!(face.normals.iter().all(|normal| normal.is_finite() && (normal.length() - 1.0).abs() <= 1.0e-6), "{name}: face {} has a non-finite or non-unit exact-surface normal", face.face_index);

		for (triangle_index, triangle) in face.indices.chunks_exact(3).enumerate() {
			assert!(triangle.iter().all(|index| (*index as usize) < face.vertices.len()), "{name}: face {} triangle {triangle_index} has an invalid index", face.face_index);
			let positions = [triangle[0], triangle[1], triangle[2]].map(|index| face.vertices[index as usize]);
			let first = positions[1] - positions[0];
			let second = positions[2] - positions[0];
			let double_area = first.cross(second).length();
			let longest_squared = [first, second, positions[2] - positions[1]].into_iter().map(DVec3::length_squared).fold(0.0_f64, f64::max);
			assert!(longest_squared > 0.0 && double_area > longest_squared * 1.0e-12, "{name}: face {} triangle {triangle_index} is degenerate: {positions:?}", face.face_index);

			let averaged_normal = triangle.iter().map(|index| face.normals[*index as usize]).sum::<DVec3>().normalize();
			assert!(first.cross(second).normalize().dot(averaged_normal) > 0.0, "{name}: face {} triangle {triangle_index} is wound against its exact-surface normal", face.face_index);

			for [start, end] in [[0, 1], [1, 2], [2, 0]] {
				let (segment, orientation) = ExactSegment::new(positions[start], positions[end]).expect("non-degenerate triangle cannot contain a collapsed edge");
				let uses = edge_uses.entry(segment).or_default();
				uses.0 += 1;
				uses.1 += orientation;
			}
			triangle_count += 1;
		}
	}

	assert!(triangle_count > 0, "{name}: tessellator returned no triangles");
	let invalid_edges = edge_uses.iter().filter(|(_, (uses, orientation_balance))| *uses != 2 || *orientation_balance != 0).take(8).collect::<Vec<_>>();
	assert!(invalid_edges.is_empty(), "{name}: tessellation is cracked, non-manifold, or inconsistently wound: {invalid_edges:?}");
	for edge in &chunks.edges {
		assert!(edge.points.len() >= 2, "{name}: semantic edge {} has fewer than two samples", edge.edge_index);
		assert!(edge.points.iter().all(|point| point.is_finite()), "{name}: semantic edge {} has a non-finite sample", edge.edge_index);
	}
}

fn encode_mesh(chunks: &MeshChunks) -> Vec<u8> {
	fn push_u32(bytes: &mut Vec<u8>, value: u32) {
		bytes.extend_from_slice(&value.to_le_bytes());
	}

	fn push_usize(bytes: &mut Vec<u8>, value: usize) {
		bytes.extend_from_slice(&(value as u64).to_le_bytes());
	}

	fn push_point(bytes: &mut Vec<u8>, point: DVec3) {
		for coordinate in point.to_array() {
			bytes.extend_from_slice(&coordinate.to_bits().to_le_bytes());
		}
	}

	let mut bytes = Vec::new();
	push_usize(&mut bytes, chunks.faces.len());
	for face in &chunks.faces {
		push_u32(&mut bytes, face.face_index);
		push_usize(&mut bytes, face.vertices.len());
		for point in &face.vertices {
			push_point(&mut bytes, *point);
		}
		push_usize(&mut bytes, face.normals.len());
		for normal in &face.normals {
			push_point(&mut bytes, *normal);
		}
		push_usize(&mut bytes, face.indices.len());
		for index in &face.indices {
			push_u32(&mut bytes, *index);
		}
	}
	push_usize(&mut bytes, chunks.edges.len());
	for edge in &chunks.edges {
		push_u32(&mut bytes, edge.edge_index);
		push_usize(&mut bytes, edge.points.len());
		for point in &edge.points {
			push_point(&mut bytes, *point);
		}
	}
	bytes
}

fn assert_same_mesh_layout(name: &str, actual: &MeshChunks, expected: &MeshChunks) {
	assert_eq!(actual.faces.len(), expected.faces.len(), "{name}: face count changed");
	for (actual, expected) in actual.faces.iter().zip(&expected.faces) {
		assert_eq!(actual.face_index, expected.face_index, "{name}: face ordering changed");
		assert_eq!(actual.vertices.len(), expected.vertices.len(), "{name}: face {} vertex count changed", actual.face_index);
		assert_eq!(actual.indices.len(), expected.indices.len(), "{name}: face {} index count changed", actual.face_index);
		if actual.indices != expected.indices {
			let mismatch = actual.indices.iter().zip(&expected.indices).position(|(actual, expected)| actual != expected);
			panic!("{name}: face {} connectivity changed at index offset {mismatch:?}", actual.face_index);
		}
	}
}

#[test]
fn seeded_analytic_corpus_is_bounded_watertight_and_schedule_deterministic() {
	let solids = placed_corpus();
	let names = ["box", "cylinder", "sphere", "truncated cone", "torus"];
	let mut triangle_count = 0;
	let mut vertex_count = 0;
	let mut encoded_byte_count = 0;
	for (index, (name, solid)) in names.into_iter().zip(&solids).enumerate() {
		let fixture_name = format!("placed seeded {name} fixture {index}");
		assert_valid_solid(&fixture_name, solid);
		let serial = Solid::mesh_chunks([solid], corpus_options(false)).unwrap_or_else(|error| panic!("{fixture_name}: serial tessellation failed: {error:?}"));
		let parallel = Solid::mesh_chunks([solid], corpus_options(true)).unwrap_or_else(|error| panic!("{fixture_name}: parallel tessellation failed: {error:?}"));
		let serial_bytes = encode_mesh(&serial);
		let parallel_bytes = encode_mesh(&parallel);
		assert_eq!(parallel_bytes, serial_bytes, "{fixture_name}: serial and parallel schedules changed the exact tessellation bytes");
		assert_finite_nondegenerate_watertight_oriented(&fixture_name, &serial);
		triangle_count += serial.faces.iter().map(|face| face.indices.len() / 3).sum::<usize>();
		vertex_count += serial.faces.iter().map(|face| face.vertices.len()).sum::<usize>();
		encoded_byte_count += serial_bytes.len();
	}

	assert!(triangle_count <= MAX_CORPUS_TRIANGLES, "seeded analytic corpus exceeded its {MAX_CORPUS_TRIANGLES}-triangle resource budget: {triangle_count}");
	assert!(vertex_count <= MAX_CORPUS_VERTICES, "seeded analytic corpus exceeded its {MAX_CORPUS_VERTICES}-vertex resource budget: {vertex_count}");
	assert!(encoded_byte_count <= MAX_CORPUS_BYTES, "seeded analytic corpus exceeded its {MAX_CORPUS_BYTES}-byte encoded resource budget: {encoded_byte_count}");
}

#[test]
fn relative_tessellation_layout_is_equivariant_across_scale_and_rigid_placement() {
	let mut generator = SeededGenerator::new(CORPUS_SEED ^ 0x6a09_e667_f3bc_c909);
	for (index, primitive) in seeded_primitives(&mut generator).into_iter().enumerate() {
		let source = primitive.solid();
		let scale = [0.001, 0.1, 1.0, 10.0, 1_000.0][index];
		let transformed = primitive.solid().scale(DVec3::ZERO, scale).located(generator.direction(), generator.range(-PI, PI), generator.direction() * 1.0e8);
		assert_valid_solid(&format!("source analytic fixture {index}"), &source);
		assert_valid_solid(&format!("transformed analytic fixture {index}"), &transformed);

		let source_mesh = Solid::mesh_chunks([&source], corpus_options(false)).unwrap_or_else(|error| panic!("tessellate source analytic fixture {index}: {error:?}"));
		let transformed_mesh = Solid::mesh_chunks([&transformed], corpus_options(false)).unwrap_or_else(|error| panic!("tessellate transformed analytic fixture {index}: {error:?}"));
		assert_same_mesh_layout(&format!("analytic fixture {index}"), &transformed_mesh, &source_mesh);
		assert_finite_nondegenerate_watertight_oriented(&format!("source analytic fixture {index}"), &source_mesh);
		assert_finite_nondegenerate_watertight_oriented(&format!("transformed analytic fixture {index}"), &transformed_mesh);
	}
}
