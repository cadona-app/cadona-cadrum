use std::collections::BTreeMap;

use cadrum::{CancellationToken, DVec3, MeshChunks, Solid, Tessellation};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct PositionKey([u64; 3]);

impl PositionKey {
	fn new(position: DVec3) -> Self {
		Self(position.to_array().map(|coordinate| if coordinate == 0.0 { 0 } else { coordinate.to_bits() }))
	}
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct SegmentKey {
	first: PositionKey,
	second: PositionKey,
}

impl SegmentKey {
	fn new(first: DVec3, second: DVec3) -> (Self, i32) {
		let first = PositionKey::new(first);
		let second = PositionKey::new(second);
		if first <= second {
			(Self { first, second }, 1)
		} else {
			(Self { first: second, second: first }, -1)
		}
	}
}

fn triangle_edge_uses(mesh: &MeshChunks) -> BTreeMap<SegmentKey, (usize, i32)> {
	let mut uses = BTreeMap::<SegmentKey, (usize, i32)>::new();
	for face in &mesh.faces {
		for triangle in face.indices.chunks_exact(3) {
			for [first, second] in [[triangle[0], triangle[1]], [triangle[1], triangle[2]], [triangle[2], triangle[0]]] {
				let (segment, direction) = SegmentKey::new(face.vertices[first as usize], face.vertices[second as usize]);
				let entry = uses.entry(segment).or_default();
				entry.0 += 1;
				entry.1 += direction;
			}
		}
	}
	uses
}

#[test]
fn structured_boolean_patch_preserves_bit_identical_shared_boundaries() {
	let cylinder = Solid::cylinder(10.0, DVec3::Z * 8.0);
	let box_tool = Solid::cube(DVec3::new(-12.0, -6.0, 0.0), DVec3::new(12.0, 6.0, 8.0)).translate(DVec3::new(2.0, -4.0, 0.0));
	let intersection = Solid::boolean_build_regularized_cancelable(&(&cylinder * &box_tool), &CancellationToken::new()).expect("intersect cylinder with an offset box").into_iter().next().expect("intersection produces one solid");
	let options = Tessellation { deflection_linear: 0.004, deflection_angular: 0.2, relative_linear: true, include_edges: true, parallel: true };

	let mesh = Solid::mesh_chunks([&intersection], options).expect("tessellate structured cylindrical Boolean face");

	assert_eq!(mesh.faces.len(), 4);
	assert_eq!(mesh.edges.len(), 6);
	let uses = triangle_edge_uses(&mesh);
	let invalid = uses.iter().filter(|(_, (count, direction))| *count != 2 || *direction != 0).take(8).collect::<Vec<_>>();
	assert!(invalid.is_empty(), "structured Boolean tessellation is cracked or inconsistently wound: {invalid:?}");
	for edge in &mesh.edges {
		for points in edge.points.windows(2) {
			let (segment, _) = SegmentKey::new(points[0], points[1]);
			if segment.first != segment.second {
				assert!(uses.contains_key(&segment), "semantic edge {} is not represented bit-identically by a surface boundary: {points:?}", edge.edge_index);
			}
		}
	}
}

#[test]
fn successive_fillets_keep_collapsed_corner_poles_closed() {
	let source = Solid::cube(DVec3::new(-10.0, -10.0, 0.0), DVec3::new(20.0, 10.0, 20.0));
	let topology = source.topology_snapshot_with_options(cadrum::TopologyQueryOptions::MEASUREMENT).unwrap();
	let cap = (0..topology.face_ids().len() as u32).find(|face| topology.face_facts(*face).unwrap().normal.is_some_and(|normal| normal[0] > 0.99)).unwrap();
	let edges = topology.face_edges(cap).unwrap().iter().map(|index| source.iter_edge().nth(*index as usize).unwrap());
	let rounded = source.fillet_edges(2.0, edges).unwrap();
	let topology = rounded.topology_snapshot_with_options(cadrum::TopologyQueryOptions::MEASUREMENT).unwrap();
	let chain = (0..topology.edge_ids().len() as u32)
		.filter(|index| {
			let facts = topology.edge_facts(*index).unwrap();
			facts.midpoint.is_some_and(|point| point[1] > 8.0 && point[2] > 18.0) && facts.tangent.is_some_and(|tangent| tangent[0].abs() > 0.1)
		})
		.collect::<Vec<_>>();
	assert_eq!(chain.len(), 2);
	for radius in [0.5, 1.0] {
		let edges = chain.iter().map(|index| rounded.iter_edge().nth(*index as usize).unwrap());
		let blended = rounded.fillet_edges(radius, edges).unwrap();
		assert!(blended.validate().unwrap().valid);
		for parallel in [false, true] {
			let options = Tessellation { deflection_linear: 0.01, deflection_angular: 0.25, relative_linear: false, include_edges: true, parallel };
			let mesh = Solid::mesh_chunks([&blended], options).expect("fillet corner poles must tessellate");
			let invalid = triangle_edge_uses(&mesh).into_iter().filter(|(_, (count, direction))| *count != 2 || *direction != 0).take(8).collect::<Vec<_>>();
			assert!(invalid.is_empty(), "fillet radius {radius} produced a cracked mesh: {invalid:?}");
			let volume = mesh
				.faces
				.iter()
				.flat_map(|face| {
					face.indices.chunks_exact(3).map(|triangle| {
						let [a, b, c] = [triangle[0], triangle[1], triangle[2]].map(|index| face.vertices[index as usize]);
						a.dot(b.cross(c)) / 6.0
					})
				})
				.sum::<f64>();
			assert!((volume - blended.volume()).abs() < blended.volume() * 0.001);
		}
	}
}
