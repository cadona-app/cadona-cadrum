const ORIGINAL: &str = "        double angmax = Tol / (Vmax - Vmin);\n        gp_Dir D(Dn);\n        Essai = (D.IsNormal(AS.Direction(), angmax));";
const REPLACEMENT: &str = "        // Dv is the extrusion direction: Du cross Dv is normal by construction.\n        // Keep the basis-curve linear tolerance check below.\n        Essai = true;";

pub fn patch_extrusion_planarity(source: &str) -> Result<String, &'static str> {
	match (source.matches(ORIGINAL).count(), source.matches(REPLACEMENT).count()) {
		(1, 0) => Ok(source.replacen(ORIGINAL, REPLACEMENT, 1)),
		(0, 1) => Ok(source.to_owned()),
		_ => Err("OCCT extrusion planarity source changed; review the patch before building"),
	}
}
