#include <BRepBuilderAPI_FindPlane.hxx>
#include <Geom_ConicalSurface.hxx>
#include <Geom_TrimmedCurve.hxx>
#include <Geom2d_BSplineCurve.hxx>
#include <Geom2dConvert.hxx>
#include <GeomAPI.hxx>
#include <Geom2dAPI_Interpolate.hxx>
#include <BRepOffset_MakeOffset.hxx>
#include <BRepAlgoAPI_Common.hxx>
#include <BRepAlgoAPI_BooleanOperation.hxx>
#include "cadrum/src/ffi.rs.h"

#ifndef __wasm__
#include <chrono>
#include <mutex>
#endif

// ==================== OCCT headers (impl only — not exposed via wrapper.h) ====================
//
// Grouped by responsibility. Anything used in wrapper.h is included there;
// here we only pull in what the implementations need.

// --- Standard / exceptions ---
#include <Standard_Failure.hxx>
#include <Standard_OutOfMemory.hxx>
#include <Message_ProgressIndicator.hxx>
#include <Message_ProgressRange.hxx>
#include <Message_ProgressScope.hxx>

// --- Topology types & navigation ---
#include <TopoDS.hxx>
#include <TopoDS_Compound.hxx>
#include <TopAbs_ShapeEnum.hxx>
#include <TopExp.hxx>
#include <TopTools_IndexedMapOfShape.hxx>
#include <TopExp_Explorer.hxx>
#include <TopLoc_Location.hxx>
#include <NCollection_IndexedMap.hxx>
#include <NCollection_List.hxx>
#include <TopTools_ShapeMapHasher.hxx>

// --- Geometry primitives (gp / Geom / 2d) ---
#include <gp_Ax1.hxx>
#include <gp_Ax2.hxx>
#include <gp_Circ.hxx>
#include <gp_Pnt.hxx>
#include <gp_Dir.hxx>
#include <gp_Pln.hxx>
#include <gp_Trsf.hxx>
#include <gp_Pnt2d.hxx>
#include <Geom_Surface.hxx>
#include <Geom_RectangularTrimmedSurface.hxx>
#include <GeomConvert.hxx>
#include <Geom_CylindricalSurface.hxx>
#include <GeomLib_IsPlanarSurface.hxx>
#include <GeomAbs_Shape.hxx>
#include <Geom2d_Line.hxx>
#include <GC_MakeArcOfCircle.hxx>

// --- BRep builders (faces / wires / edges / solid primitives) ---
#include <BRep_Builder.hxx>
#include <BRep_Tool.hxx>
#include <BRepLib.hxx>
#include <BRepBuilderAPI_Copy.hxx>
#include <BRepBuilderAPI_MakeFace.hxx>
#include <BRepBuilderAPI_MakePolygon.hxx>
#include <BRepBuilderAPI_MakeEdge.hxx>
#include <BRepBuilderAPI_MakeWire.hxx>
#include <BRepBuilderAPI_MakeSolid.hxx>
#include <BRepBuilderAPI_MakeVertex.hxx>
#include <BRepBuilderAPI_Sewing.hxx>
#include <BRepBuilderAPI_Transform.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <BRepClass3d_SolidClassifier.hxx>
#include <BRepClass_FaceClassifier.hxx>
#include <BRepExtrema_DistShapeShape.hxx>
#include <BRepExtrema_ExtPF.hxx>
#include <BRepFeat_SplitShape.hxx>
#include <BRepLProp_SLProps.hxx>
#include <BRepAdaptor_Surface.hxx>
#include <BRepPrimAPI_MakeBox.hxx>
#include <BRepPrimAPI_MakeCone.hxx>
#include <BRepPrimAPI_MakeCylinder.hxx>
#include <BRepPrimAPI_MakeHalfSpace.hxx>
#include <BRepPrimAPI_MakeSphere.hxx>
#include <BRepProj_Projection.hxx>
#include <BOPAlgo_Splitter.hxx>
#include <BOPAlgo_BuilderFace.hxx>
#include <BRepTopAdaptor_FClass2d.hxx>
#include <gp_Elips.hxx>
#include <BRepAlgo_NormalProjection.hxx>
#include <BRepPrimAPI_MakePrism.hxx>
#include <BRepPrimAPI_MakeRevol.hxx>
#include <BRepPrimAPI_MakeTorus.hxx>

// --- Boolean operations & shape cleanup ---
#include <BOPAlgo_CellsBuilder.hxx>
#include <BRepAlgoAPI_Section.hxx>
#include <BRepAdaptor_Curve.hxx>
#include <BRepAdaptor_Curve2d.hxx>
#include <GCPnts_QuasiUniformDeflection.hxx>
#include <ShapeAnalysis_FreeBounds.hxx>
#include <ShapeAnalysis_Surface.hxx>
#include <TopTools_HSequenceOfShape.hxx>
#include <BRepTools_WireExplorer.hxx>
#include <BRepTools.hxx>
#include <ShapeUpgrade_UnifySameDomain.hxx>
#include <BRepTools_History.hxx>

// --- Sweep / pipe / loft ---
#include <BRepFilletAPI_MakeFillet.hxx>
#include <BRepFilletAPI_MakeChamfer.hxx>
#include <BRepOffsetAPI_MakeOffsetShape.hxx>
#include <BRepOffsetAPI_MakePipeShell.hxx>
#include <BRepOffsetAPI_MakeThickSolid.hxx>
#include <BRepOffsetAPI_ThruSections.hxx>
#include <BRepOffset_Mode.hxx>
#include <GeomAbs_JoinType.hxx>

// --- Mesh, classification, mass / surface properties ---
#include <BRepLib_ToolTriangulatedShape.hxx>
#include <BRepMesh_IncrementalMesh.hxx>
#include <BRepBndLib.hxx>
#include <Bnd_Box.hxx>
#include <BRepGProp.hxx>
#include <GProp_GProps.hxx>
#include <IMeshTools_Parameters.hxx>
#include <Poly_PolygonOnTriangulation.hxx>
#include <Poly_Triangulation.hxx>

// --- Curve adaptation / approximation ---
#include <BRepAdaptor_Curve.hxx>
#include <GCPnts_AbscissaPoint.hxx>
#include <GCPnts_TangentialDeflection.hxx>
#include <GeomAPI_Interpolate.hxx>
#include <GeomAPI_ProjectPointOnSurf.hxx>
#include <GeomAPI_PointsToBSplineSurface.hxx>
#include <GeomAPI_ProjectPointOnCurve.hxx>
#include <Geom_BSplineCurve.hxx>
#include <Geom_BSplineSurface.hxx>
#include <NCollection_Array2.hxx>
#include <NCollection_HArray1.hxx>
#include <Precision.hxx>

// --- I/O (BREP / STEP / progress) ---
// STEP-specific headers are only needed by the non-color STEP path
// (`read_step_stream` / `write_step_stream`); with color, STEP routes
// through XCAF in the FEATURE_COLOR section below.
#include <BinTools.hxx>
#ifndef FEATURE_COLOR
#include <STEPControl_Reader.hxx>
#include <STEPControl_Writer.hxx>
#endif
#include <Message.hxx>

// --- C++ standard library ---
#include <istream>
#include <ostream>
#include <sstream>
#include <streambuf>
#include <cmath>
#include <cstring>
#include <cstdint>
#include <algorithm>
#include <initializer_list>
#include <limits>
#include <new>
#include <stdexcept>
#include <unordered_map>
#include <unordered_set>
#include <array>
#include <vector>

namespace cadrum {

class RustProgressIndicator final : public Message_ProgressIndicator {
public:
    explicit RustProgressIndicator(const CancellationToken& progress)
        : progress_(progress) {}

protected:
    bool UserBreak() override {
        return rust_progress_cancelled(progress_);
    }

    void Show(const Message_ProgressScope&, const bool) override {
        rust_progress_set(progress_, GetPosition());
    }

private:
    const CancellationToken& progress_;
};

struct NativeDiagnosticState {
    std::string operation;
    std::string stage;
    std::string exception_type;
    std::string message;
    uint32_t category = 0;
    int32_t status = 0;
    bool present = false;
};

static thread_local NativeDiagnosticState operation_diagnostic;

void clear_operation_diagnostic() {
    operation_diagnostic = NativeDiagnosticState{};
}

static void record_standard_failure(
    const char* operation,
    const char* stage,
    uint32_t category,
    const Standard_Failure& failure) {
    operation_diagnostic.operation = operation;
    operation_diagnostic.stage = stage;
    operation_diagnostic.exception_type = failure.ExceptionType();
    operation_diagnostic.message = failure.what();
    operation_diagnostic.category = category;
    operation_diagnostic.status = 0;
    operation_diagnostic.present = true;
}

static void record_input_failure(const char* operation, const char* message) {
    operation_diagnostic.operation = operation;
    operation_diagnostic.stage = "validate_input";
    operation_diagnostic.message = message;
    operation_diagnostic.category = 2;
    operation_diagnostic.status = 0;
    operation_diagnostic.present = true;
}

static void record_resource_failure(const char* operation, const char* message) {
    operation_diagnostic.operation = operation;
    operation_diagnostic.stage = "resource_limit";
    operation_diagnostic.message = message;
    operation_diagnostic.category = 5;
    operation_diagnostic.status = 0;
    operation_diagnostic.present = true;
}

static void record_stage_failure(
    const char* operation,
    const char* stage,
    const char* message) {
    if (operation_diagnostic.present) return;
    operation_diagnostic.operation = operation;
    operation_diagnostic.stage = stage;
    operation_diagnostic.message = message;
    operation_diagnostic.category = 7;
    operation_diagnostic.status = 0;
    operation_diagnostic.present = true;
}

class ScopedFailureDiagnostic final {
public:
    ScopedFailureDiagnostic(
        const char* operation,
        const char*& stage,
        bool& success,
        const CancellationToken& progress)
        : operation_(operation),
          stage_(stage),
          success_(success),
          progress_(progress) {}

    ~ScopedFailureDiagnostic() {
        if (!success_ && !rust_progress_cancelled(progress_)) {
            record_stage_failure(
                operation_,
                stage_,
                "operation returned without completing its current stage");
        }
    }

private:
    const char* operation_;
    const char*& stage_;
    bool& success_;
    const CancellationToken& progress_;
};

OperationDiagnosticData take_operation_diagnostic() {
    OperationDiagnosticData result;
    result.operation = rust::String(operation_diagnostic.operation);
    result.stage = rust::String(operation_diagnostic.stage);
    result.exception_type = rust::String(operation_diagnostic.exception_type);
    result.message = rust::String(operation_diagnostic.message);
    result.category = operation_diagnostic.category;
    result.status = operation_diagnostic.status;
    result.present = operation_diagnostic.present;
    clear_operation_diagnostic();
    return result;
}

// OCCT defaults to a stdout printer that emits "Statistics on Transfer" banners on STEP read/write.
// Clear all printers at load time per the documented recommendation.
// ******        Statistics on Transfer (Write)                 ******
static const int _silence_occt_default_printer = []() {
    Message::DefaultMessenger()->ChangePrinters().Clear();
    return 0;
}();

// ==================== Shape Constructors ====================

std::unique_ptr<TopoDS_Shape> make_half_space(
    double ox, double oy, double oz,
    double nx, double ny, double nz)
{
    try {
        const double len = std::sqrt(nx*nx + ny*ny + nz*nz);
        if (!std::isfinite(ox) || !std::isfinite(oy) || !std::isfinite(oz)
            || !std::isfinite(len) || len < Precision::Confusion()) {
            record_input_failure(__func__, "origin must be finite and normal must be finite and nonzero");
            return nullptr;
        }
        gp_Pnt origin(ox, oy, oz);
        gp_Dir normal(nx, ny, nz);
        gp_Pln plane(origin, normal);

        BRepBuilderAPI_MakeFace face_maker(plane);
        TopoDS_Face face = face_maker.Face();

        // Reference point is on the SAME side as the normal.
        // BRepPrimAPI_MakeHalfSpace fills the ref_point side,
        // so the solid occupies the half-space where the normal points.
        gp_Pnt ref_point(ox + nx/len, oy + ny/len, oz + nz/len);

        BRepPrimAPI_MakeHalfSpace maker(face, ref_point);
        return std::make_unique<TopoDS_Shape>(maker.Solid());
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return nullptr;
    }
}

std::unique_ptr<TopoDS_Shape> make_box(
    double x1, double y1, double z1,
    double x2, double y2, double z2)
{
    try {
        if (!std::isfinite(x1) || !std::isfinite(y1) || !std::isfinite(z1)
            || !std::isfinite(x2) || !std::isfinite(y2) || !std::isfinite(z2)) {
            record_input_failure(__func__, "box corners must be finite");
            return nullptr;
        }
        double minx = std::min(x1, x2);
        double miny = std::min(y1, y2);
        double minz = std::min(z1, z2);
        double maxx = std::max(x1, x2);
        double maxy = std::max(y1, y2);
        double maxz = std::max(z1, z2);

        gp_Pnt p_min(minx, miny, minz);
        double dx = maxx - minx;
        double dy = maxy - miny;
        double dz = maxz - minz;
        if (dx < Precision::Confusion() || dy < Precision::Confusion()
            || dz < Precision::Confusion()) {
            record_input_failure(__func__, "box dimensions must be positive");
            return nullptr;
        }

        BRepPrimAPI_MakeBox maker(p_min, dx, dy, dz);
        return std::make_unique<TopoDS_Shape>(maker.Shape());
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return nullptr;
    }
}

std::unique_ptr<TopoDS_Shape> make_cylinder(
    double px, double py, double pz,
    double dx, double dy, double dz,
    double radius, double height)
{
    try {
        const double direction_length = std::sqrt(dx*dx + dy*dy + dz*dz);
        if (!std::isfinite(px) || !std::isfinite(py) || !std::isfinite(pz)
            || !std::isfinite(direction_length) || !std::isfinite(radius)
            || !std::isfinite(height) || direction_length < Precision::Confusion()
            || radius < Precision::Confusion() || height < Precision::Confusion()) {
            record_input_failure(__func__, "cylinder axis, radius, and height must be finite and positive");
            return nullptr;
        }
        gp_Pnt center(px, py, pz);
        gp_Dir direction(dx, dy, dz);
        gp_Ax2 axis(center, direction);

        BRepPrimAPI_MakeCylinder maker(axis, radius, height);
        return std::make_unique<TopoDS_Shape>(maker.Shape());
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return nullptr;
    }
}

std::unique_ptr<TopoDS_Shape> make_sphere(
    double cx, double cy, double cz,
    double radius)
{
    try {
        if (!std::isfinite(cx) || !std::isfinite(cy) || !std::isfinite(cz)
            || !std::isfinite(radius) || radius < Precision::Confusion()) {
            record_input_failure(__func__, "sphere center and radius must be finite and radius positive");
            return nullptr;
        }
        gp_Pnt center(cx, cy, cz);
        BRepPrimAPI_MakeSphere maker(center, radius);
        return std::make_unique<TopoDS_Shape>(maker.Shape());
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return nullptr;
    }
}

std::unique_ptr<TopoDS_Shape> make_cone(
    double px, double py, double pz,
    double dx, double dy, double dz,
    double r1, double r2, double height)
{
    try {
        const double direction_length = std::sqrt(dx*dx + dy*dy + dz*dz);
        if (!std::isfinite(px) || !std::isfinite(py) || !std::isfinite(pz)
            || !std::isfinite(direction_length) || !std::isfinite(r1)
            || !std::isfinite(r2) || !std::isfinite(height)
            || direction_length < Precision::Confusion() || r1 < 0.0 || r2 < 0.0
            || std::max(r1, r2) < Precision::Confusion()
            || height < Precision::Confusion()) {
            record_input_failure(__func__, "cone axis, radii, and height are invalid");
            return nullptr;
        }
        gp_Pnt center(px, py, pz);
        gp_Dir direction(dx, dy, dz);
        gp_Ax2 axis(center, direction);
        BRepPrimAPI_MakeCone maker(axis, r1, r2, height);
        return std::make_unique<TopoDS_Shape>(maker.Shape());
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return nullptr;
    }
}

std::unique_ptr<TopoDS_Shape> make_torus(
    double px, double py, double pz,
    double dx, double dy, double dz,
    double r1, double r2)
{
    try {
        const double direction_length = std::sqrt(dx*dx + dy*dy + dz*dz);
        if (!std::isfinite(px) || !std::isfinite(py) || !std::isfinite(pz)
            || !std::isfinite(direction_length) || !std::isfinite(r1)
            || !std::isfinite(r2) || direction_length < Precision::Confusion()
            || r1 < Precision::Confusion() || r2 < Precision::Confusion()
            || r2 >= r1) {
            record_input_failure(__func__, "torus axis and radii are invalid");
            return nullptr;
        }
        gp_Pnt center(px, py, pz);
        gp_Dir direction(dx, dy, dz);
        gp_Ax2 axis(center, direction);
        BRepPrimAPI_MakeTorus maker(axis, r1, r2);
        return std::make_unique<TopoDS_Shape>(maker.Shape());
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return nullptr;
    }
}

std::unique_ptr<TopoDS_Shape> make_empty() {
    TopoDS_Compound compound;
    BRep_Builder builder;
    builder.MakeCompound(compound);
    return std::make_unique<TopoDS_Shape>(compound);
}

std::unique_ptr<TopoDS_Shape> deep_copy(const TopoDS_Shape& shape) {
    BRepBuilderAPI_Copy copier(shape, true, false);
    return std::make_unique<TopoDS_Shape>(copier.Shape());
}

static bool indexed_subshape(
    const TopoDS_Shape& shape,
    uint32_t kind,
    uint32_t index,
    TopoDS_Shape& result) {
    TopAbs_ShapeEnum shape_kind;
    switch (kind) {
        case 0: shape_kind = TopAbs_FACE; break;
        case 1: shape_kind = TopAbs_EDGE; break;
        case 2: shape_kind = TopAbs_VERTEX; break;
        default: return false;
    }
    NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> shapes;
    TopExp::MapShapes(shape, shape_kind, shapes);
    if (index >= static_cast<uint32_t>(shapes.Extent())) return false;
    result = shapes(static_cast<int>(index + 1));
    return true;
}

rust::Vec<double> topology_bounds(const TopoDS_Shape& shape, uint32_t kind, uint32_t index) {
    rust::Vec<double> result;
    try {
        TopoDS_Shape entity;
        if (!indexed_subshape(shape, kind, index, entity)) return result;
        Bnd_Box box;
        // Camera bounds must not depend on cached display triangulation.
        BRepBndLib::AddOptimal(entity, box, false, false);
        if (box.IsVoid() || box.IsOpen()) return result;
        double xmin, ymin, zmin, xmax, ymax, zmax;
        box.Get(xmin, ymin, zmin, xmax, ymax, zmax);
        for (double value : {xmin, ymin, zmin, xmax, ymax, zmax}) {
            if (!std::isfinite(value)) return {};
            result.push_back(value);
        }
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
    }
    return result;
}

TopologyDistanceData topology_distance(
    const TopoDS_Shape& first,
    uint32_t first_kind,
    uint32_t first_index,
    const TopoDS_Shape& second,
    uint32_t second_kind,
    uint32_t second_index) {
    TopologyDistanceData result{};
    try {
        TopoDS_Shape first_entity;
        TopoDS_Shape second_entity;
        if (!indexed_subshape(first, first_kind, first_index, first_entity) ||
            !indexed_subshape(second, second_kind, second_index, second_entity)) {
            operation_diagnostic.operation = "topology_distance";
            operation_diagnostic.stage = "resolve_inputs";
            operation_diagnostic.message = "topology entity index is outside the source shape";
            operation_diagnostic.category = 2;
            operation_diagnostic.present = true;
            return result;
        }
        BRepExtrema_DistShapeShape extrema(first_entity, second_entity);
        extrema.Perform();
        if (!extrema.IsDone() || extrema.NbSolution() < 1) {
            operation_diagnostic.operation = "topology_distance";
            operation_diagnostic.stage = "native";
            operation_diagnostic.message = "OCCT found no minimum-distance solution";
            operation_diagnostic.category = 3;
            operation_diagnostic.present = true;
            return result;
        }
        const gp_Pnt first_point = extrema.PointOnShape1(1);
        const gp_Pnt second_point = extrema.PointOnShape2(1);
        result.distance = extrema.Value();
        result.first_x = first_point.X();
        result.first_y = first_point.Y();
        result.first_z = first_point.Z();
        result.second_x = second_point.X();
        result.second_y = second_point.Y();
        result.second_z = second_point.Z();
        result.success = std::isfinite(result.distance) && result.distance >= 0.0;
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
    }
    return result;
}

TopologyDistanceData shape_boundary_distance(
    const TopoDS_Shape& first,
    const TopoDS_Shape& second) {
    TopologyDistanceData result{};
    try {
        auto boundary = [](const TopoDS_Shape& shape) {
            TopoDS_Compound faces;
            BRep_Builder builder;
            builder.MakeCompound(faces);
            for (TopExp_Explorer explorer(shape, TopAbs_FACE); explorer.More(); explorer.Next()) {
                builder.Add(faces, explorer.Current());
            }
            return faces;
        };
        const TopoDS_Compound first_boundary = boundary(first);
        const TopoDS_Compound second_boundary = boundary(second);
        BRepExtrema_DistShapeShape extrema(first_boundary, second_boundary);
        extrema.Perform();
        if (!extrema.IsDone() || extrema.NbSolution() < 1) {
            operation_diagnostic.operation = __func__;
            operation_diagnostic.stage = "native";
            operation_diagnostic.message = "OCCT found no boundary-distance solution";
            operation_diagnostic.category = 3;
            operation_diagnostic.present = true;
            return result;
        }
        const gp_Pnt first_point = extrema.PointOnShape1(1);
        const gp_Pnt second_point = extrema.PointOnShape2(1);
        result.distance = extrema.Value();
        result.first_x = first_point.X();
        result.first_y = first_point.Y();
        result.first_z = first_point.Z();
        result.second_x = second_point.X();
        result.second_y = second_point.Y();
        result.second_z = second_point.Z();
        result.success = std::isfinite(result.distance) && result.distance >= 0.0;
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
    }
    return result;
}

// ==================== STEP read post-processing ====================

// Recover Solids from a STEP-read Compound that has disjoint shells / loose
// faces (multi-color export from SolveSpace etc.). Returns the original
// compound if no orphan faces are found (= valid STEP, zero overhead).
//
// If `colorMap` is non-null, remaps its keys for faces whose TShape* changed
// during sewing (only applicable when called from the color path).
//
// See #129 for the reproducer and root-cause analysis.
//
// Design notes:
//   - has_orphans を残す理由: SewedShape().IsNull() でも判定可能だが、
//     (a) 空入力 Perform() を回避、(b) 「sewing 不要」と「sewing 失敗」を区別、
//     の 2 点で明示フラグ優位。
//   - Solid 配下の face を sewer に入れない理由: 既存 valid Solid の face は
//     TShape* preserve したい (colormap キー保持 + 既存挙動互換)。
//   - TopoDS_Iterator (immediate children) ではなく TopExp_Explorer (再帰) を
//     使う理由: STEP のツリー構造で valid Solid が深い所に埋まり、その兄弟に
//     orphan face がある混在ケース (例: Compound { sub { Solid + face×6 } })
//     を救うため。
//   - tolerance = Precision::Confusion(): 重複 EDGE_CURVE は座標完全一致なので
//     最厳設定で十分。緩めると意図しない縫合リスクが増す。
static TopoDS_Shape try_sew_orphan_faces(
    const TopoDS_Shape& compound,
    std::unordered_map<uint64_t, std::array<float, 3>>* colorMap)
{
    // 1. 既存 Solid と配下 face TShape* 集合を回収
    std::unordered_set<const TopoDS_TShape*> in_solid;
    std::vector<TopoDS_Shape> existing_solids;
    for (TopExp_Explorer sx(compound, TopAbs_SOLID); sx.More(); sx.Next()) {
        existing_solids.push_back(sx.Current());
        for (TopExp_Explorer fx(sx.Current(), TopAbs_FACE); fx.More(); fx.Next()) {
            in_solid.insert(fx.Current().TShape().get());
        }
    }

    // 2. 孤立 face を Sewing に投入
    BRepBuilderAPI_Sewing sewer(Precision::Confusion());
    bool has_orphans = false;
    std::vector<TopoDS_Shape> orphan_faces;  // color remap 用に保持
    for (TopExp_Explorer fx(compound, TopAbs_FACE); fx.More(); fx.Next()) {
        if (in_solid.count(fx.Current().TShape().get()) == 0) {
            sewer.Add(fx.Current());
            orphan_faces.push_back(fx.Current());
            has_orphans = true;
        }
    }

    // 3. 正常 STEP は素通し (= zero-overhead)
    if (!has_orphans) return compound;

    // 4. 縫合 → Shell ごとに MakeSolid → 新 compound 構築
    sewer.Perform();
    TopoDS_Shape sewn = sewer.SewedShape();

    BRep_Builder bb;
    TopoDS_Compound new_compound;
    bb.MakeCompound(new_compound);
    for (const auto& s : existing_solids) bb.Add(new_compound, s);
    for (TopExp_Explorer sx(sewn, TopAbs_SHELL); sx.More(); sx.Next()) {
        BRepBuilderAPI_MakeSolid mk(TopoDS::Shell(sx.Current()));
        if (mk.IsDone()) bb.Add(new_compound, mk.Solid());
    }

    // 5. colormap キー remap (color path のみ)
    if (colorMap) {
        for (const auto& old_face : orphan_faces) {
            uint64_t old_id = reinterpret_cast<uint64_t>(old_face.TShape().get());
            auto it = colorMap->find(old_id);
            if (it == colorMap->end()) continue;
            if (sewer.IsModified(old_face)) {
                uint64_t new_id = reinterpret_cast<uint64_t>(
                    sewer.Modified(old_face).TShape().get());
                (*colorMap)[new_id] = it->second;
            }
        }
    }

    return new_compound;
}

// ==================== Compound Decompose/Compose ====================

std::unique_ptr<std::vector<TopoDS_Shape>> decompose_into_solids(const TopoDS_Shape& shape) {
    auto result = std::make_unique<std::vector<TopoDS_Shape>>();
    for (TopExp_Explorer ex(shape, TopAbs_SOLID); ex.More(); ex.Next()) {
        result->push_back(ex.Current());  // shallow handle copy
    }
    return result;
}

void compound_add(TopoDS_Shape& compound, const TopoDS_Shape& child) {
    BRep_Builder builder;
    builder.Add(compound, child);
}

std::unique_ptr<TopoDS_Shape> build_compound() {
    auto compound = std::make_unique<TopoDS_Compound>();
    BRep_Builder builder;
    builder.MakeCompound(*compound);
    return std::unique_ptr<TopoDS_Shape>(std::move(compound));
}

void compound_add_edge(TopoDS_Shape& compound, const TopoDS_Edge& child) {
    BRep_Builder builder;
    builder.Add(compound, child);
}

std::unique_ptr<TopoDS_Shape> split_solid_with_projected_edges(const TopoDS_Shape& solid,
    const TopoDS_Shape& tool_edges, double dx, double dy, double dz)
{
    try {
        gp_Dir direction(dx, dy, dz);

        // Project every tool edge onto the solid's surfaces along the given
        // direction. BRepProj_Projection yields one wire per hit region; a
        // single edge can land on several faces (e.g. front and back of the
        // solid), and the splitter wants all of them.
        TopoDS_Compound projected;
        BRep_Builder builder;
        builder.MakeCompound(projected);
        bool any_projected = false;
        for (TopExp_Explorer ex(tool_edges, TopAbs_EDGE); ex.More(); ex.Next()) {
            BRepProj_Projection projector(TopoDS::Edge(ex.Current()), solid, direction);
            for (; projector.More(); projector.Next()) {
                const TopoDS_Wire& wire = projector.Current();
                if (!wire.IsNull()) {
                    builder.Add(projected, wire);
                    any_projected = true;
                }
            }
        }
        if (!any_projected) return nullptr;

        BOPAlgo_Splitter splitter;
        splitter.AddArgument(solid);
        splitter.AddTool(projected);
        splitter.Perform();
        if (splitter.HasErrors()) return nullptr;
        const TopoDS_Shape& result = splitter.Shape();
        if (result.IsNull()) return nullptr;

        // Deep-copy so the result shares no geometry handles with the
        // operator object or inputs (same rule as the boolean builders).
        BRepBuilderAPI_Copy copier(result);
        if (!copier.IsDone()) return nullptr;
        return std::make_unique<TopoDS_Shape>(copier.Shape());
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return nullptr;
    }
}

std::unique_ptr<TopoDS_Shape> project_shape_to_plane(const TopoDS_Shape& shape,
    double ox, double oy, double oz, double nx, double ny, double nz)
{
    try {
        gp_Pln plane(gp_Pnt(ox, oy, oz), gp_Dir(nx, ny, nz));
        // An unbounded plane face makes the projector emit edges with
        // infinite parameters ("BRep_Builder::Infinite parameter"), so bound
        // the face generously around the shape's own extent.
        Bnd_Box bounds;
        // Exact B-rep geometry is authoritative. Do not let a stale imported
        // Poly_Triangulation influence the projection tool's working extent.
        BRepBndLib::Add(shape, bounds, false);
        if (bounds.IsVoid()) return nullptr;
        gp_Pnt bounds_min = bounds.CornerMin();
        gp_Pnt bounds_max = bounds.CornerMax();
        double reach = bounds_min.Distance(bounds_max)
            + gp_Pnt(ox, oy, oz).Distance(gp_Pnt(
                (bounds_min.X() + bounds_max.X()) * 0.5,
                (bounds_min.Y() + bounds_max.Y()) * 0.5,
                (bounds_min.Z() + bounds_max.Z()) * 0.5))
            + 1.0;
        BRepBuilderAPI_MakeFace make_face(plane, -reach, reach, -reach, reach);
        if (!make_face.IsDone()) return nullptr;
        // BRepProj_Projection only accepts a wire or edge as the projected
        // shape, so feed it the shape's edges one at a time. Duplicate edges
        // (each shared by two faces) collapse via the IndexedMap.
        TopTools_IndexedMapOfShape unique_edges;
        TopExp::MapShapes(shape, TopAbs_EDGE, unique_edges);
        TopoDS_Compound projected;
        BRep_Builder builder;
        builder.MakeCompound(projected);
        bool any_projected = false;
        for (Standard_Integer index = 1; index <= unique_edges.Extent(); ++index) {
            BRepProj_Projection projector(TopoDS::Edge(unique_edges(index)), make_face.Face(), gp_Dir(nx, ny, nz));
            for (; projector.More(); projector.Next()) {
                const TopoDS_Wire& wire = projector.Current();
                if (!wire.IsNull()) {
                    builder.Add(projected, wire);
                    any_projected = true;
                }
            }
        }
        if (!any_projected) return nullptr;
        // Deep-copy: the projection result shares geometry handles with the
        // operator objects (same rule as the boolean builders).
        BRepBuilderAPI_Copy copier(projected);
        if (!copier.IsDone()) return nullptr;
        return std::make_unique<TopoDS_Shape>(copier.Shape());
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return nullptr;
    }
}

std::unique_ptr<TopoDS_Edge> edge_ellipse(double major_radius, double minor_radius,
    double xx, double xy, double xz, double nx, double ny, double nz)
{
    try {
        if (minor_radius < Precision::Confusion() || major_radius < minor_radius) return nullptr;
        gp_Dir normal(nx, ny, nz);
        gp_Dir major_dir(xx, xy, xz);
        gp_Ax2 ax2(gp_Pnt(0.0, 0.0, 0.0), normal, major_dir);
        gp_Elips ellipse(ax2, major_radius, minor_radius);
        BRepBuilderAPI_MakeEdge edgeMaker(ellipse);
        if (!edgeMaker.IsDone()) return nullptr;
        return std::make_unique<TopoDS_Edge>(edgeMaker.Edge());
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return nullptr;
    }
}

// ==================== Builders (solid → solid with history) ====================
// Bug 1 fix: All boolean results are deep-copied via BRepBuilderAPI_Copy
// so the result shares no Handle<Geom_XXX> with the input shapes.
// This prevents STATUS_HEAP_CORRUPTION when shapes are dropped in any order.
//
// Cross-section face collection: Modified() is called BEFORE BRepBuilderAPI_Copy
// because the copy severs the history table. Each collected face is then
// individually deep-copied so it is independent of the operator object.
//
// Why Modified() and not Generated():
//   The cross-section face is the tool's boundary face trimmed (bounded) to fit
//   inside the shape operand.  OCCT records this as Modified(tool_face) because
//   the face still represents the same plane — it just has smaller bounds.
//   Generated(tool_face) returns empty because no wholly NEW face was created.
//
// out_history is built from composable relay maps shared by boolean (copy-based)
// and shell/fillet/chamfer (no copy):
//   relay_from_builder  {result/pre → src}  (all builders)
//   relay_from_pair     {post → pre}        (copy-based only)
//   relay_into_history  compose → flat [post, src] pairs
// relay_into_history emits only the outermost map's keys (the real result
// faces), so no bogus pre/src-only ids leak in.

// {result/pre → src}: Modified() empty ⇒ identity, else each split target → src.
// Modified()/IsDeleted() are non-const, so Builder& (not const).
template <typename Builder>
static void relay_from_builder(
    Builder& builder,
    const TopoDS_Shape& src,
    std::unordered_map<uint64_t, uint64_t>& relay)
{
    for (TopExp_Explorer ex(src, TopAbs_FACE); ex.More(); ex.Next()) {
        const TopoDS_Shape& sf = ex.Current();
        uint64_t src_id = reinterpret_cast<uint64_t>(sf.TShape().get());
        if (builder.IsDeleted(sf)) continue;
        const NCollection_List<TopoDS_Shape>& mods = builder.Modified(sf);
        if (mods.IsEmpty()) {
            relay[src_id] = src_id;
        } else {
            for (NCollection_List<TopoDS_Shape>::Iterator it(mods); it.More(); it.Next()) {
                uint64_t pre_id = reinterpret_cast<uint64_t>(it.Value().TShape().get());
                relay[pre_id] = src_id;
            }
        }
    }
}

// {post → pre} by index (BRepBuilderAPI_Copy preserves traversal order).
static void relay_from_pair(
    const TopoDS_Shape& pre_shape,
    const TopoDS_Shape& post_shape,
    std::unordered_map<uint64_t, uint64_t>& relay)
{
    NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> pre_map, post_map;
    TopExp::MapShapes(pre_shape, TopAbs_FACE, pre_map);
    TopExp::MapShapes(post_shape, TopAbs_FACE, post_map);
    // pre_map and post_map have the same size because the copy preserves topology.
    for (int i = 1; i <= pre_map.Extent(); ++i) {
        uint64_t pre_id = reinterpret_cast<uint64_t>(pre_map(i).TShape().get());
        uint64_t post_id = reinterpret_cast<uint64_t>(post_map(i).TShape().get());
        relay[post_id] = pre_id;
    }
}

// Emit [post, src] pairs. relay2==null: flatten relay1 (its keys are final).
// relay2!=null: iterate relay2 keys (post) and resolve post→pre→src via relay1.
static void relay_into_history(
    const std::unordered_map<uint64_t, uint64_t>* relay1,
    const std::unordered_map<uint64_t, uint64_t>* relay2,
    rust::Vec<uint64_t>& out)
{
    if (relay2 == nullptr) {
        for (const auto& kv : *relay1) {
            out.push_back(kv.first);
            out.push_back(kv.second);
        }
    } else {
        for (const auto& kv : *relay2) {
            auto it = relay1->find(kv.second);
            if (it == relay1->end()) continue;
            out.push_back(kv.first);
            out.push_back(it->second);
        }
    }
}

// Topology history exposed to Plex uses artifact-local ordinals instead of
// process-local TShape addresses. This also preserves the operand namespace
// and relations generated across dimensions (for example, an edge generating
// a fillet face).
enum class HistoryKind : uint32_t { Face = 0, Edge = 1, Vertex = 2 };
enum class HistoryRelation : uint32_t { Unchanged = 0, Modified = 1, Generated = 2 };

struct HistoryMaps {
    NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> faces;
    NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> edges;
    NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> vertices;

    explicit HistoryMaps(const TopoDS_Shape& shape) {
        TopExp::MapShapes(shape, TopAbs_FACE, faces);
        TopExp::MapShapes(shape, TopAbs_EDGE, edges);
        TopExp::MapShapes(shape, TopAbs_VERTEX, vertices);
    }

    const NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher>& map(
        HistoryKind kind) const
    {
        switch (kind) {
            case HistoryKind::Face: return faces;
            case HistoryKind::Edge: return edges;
            case HistoryKind::Vertex: return vertices;
        }
        return faces;
    }
};

static bool history_kind(const TopoDS_Shape& shape, HistoryKind& kind) {
    switch (shape.ShapeType()) {
        case TopAbs_FACE: kind = HistoryKind::Face; return true;
        case TopAbs_EDGE: kind = HistoryKind::Edge; return true;
        case TopAbs_VERTEX: kind = HistoryKind::Vertex; return true;
        default: return false;
    }
}

static void append_history_relation(
    HistoryData& out,
    const HistoryMaps& result_maps,
    const TopoDS_Shape& result,
    HistoryRelation relation,
    uint32_t operand,
    HistoryKind source_kind,
    uint32_t source_index)
{
    HistoryKind result_kind;
    if (!history_kind(result, result_kind)) return;
    const int result_index = result_maps.map(result_kind).FindIndex(result);
    if (result_index <= 0) return;
    out.relations.push_back(static_cast<uint32_t>(result_kind));
    out.relations.push_back(static_cast<uint32_t>(result_index - 1));
    out.relations.push_back(static_cast<uint32_t>(relation));
    out.relations.push_back(operand);
    out.relations.push_back(static_cast<uint32_t>(source_kind));
    out.relations.push_back(source_index);
}

static void append_generated_shapes(
    HistoryData& out,
    const HistoryMaps& result_maps,
    const NCollection_List<TopoDS_Shape>& generated,
    uint32_t operand,
    HistoryKind source_kind,
    uint32_t source_index)
{
    for (NCollection_List<TopoDS_Shape>::Iterator it(generated); it.More(); it.Next()) {
        const TopoDS_Shape& generated_shape = it.Value();
        HistoryKind generated_kind;
        if (history_kind(generated_shape, generated_kind)) {
            append_history_relation(out, result_maps, generated_shape,
                HistoryRelation::Generated, operand, source_kind, source_index);
            continue;
        }
        for (HistoryKind kind : {HistoryKind::Face, HistoryKind::Edge, HistoryKind::Vertex}) {
            const TopAbs_ShapeEnum shape_kind = kind == HistoryKind::Face
                ? TopAbs_FACE
                : (kind == HistoryKind::Edge ? TopAbs_EDGE : TopAbs_VERTEX);
            for (TopExp_Explorer ex(generated_shape, shape_kind); ex.More(); ex.Next()) {
                append_history_relation(out, result_maps, ex.Current(),
                    HistoryRelation::Generated, operand, source_kind, source_index);
            }
        }
    }
}

template <typename Builder>
static void append_builder_topology_history(
    Builder& builder,
    const TopoDS_Shape& input,
    uint32_t operand,
    const HistoryMaps& result_maps,
    HistoryData& out)
{
    const HistoryMaps input_maps(input);
    for (HistoryKind source_kind : {HistoryKind::Face, HistoryKind::Edge, HistoryKind::Vertex}) {
        const auto& sources = input_maps.map(source_kind);
        for (int source_ordinal = 1; source_ordinal <= sources.Extent(); ++source_ordinal) {
            const TopoDS_Shape& source = sources(source_ordinal);
            const uint32_t source_index = static_cast<uint32_t>(source_ordinal - 1);
            bool related = false;

            const NCollection_List<TopoDS_Shape>& modified = builder.Modified(source);
            for (NCollection_List<TopoDS_Shape>::Iterator it(modified); it.More(); it.Next()) {
                append_history_relation(out, result_maps, it.Value(),
                    HistoryRelation::Modified, operand, source_kind, source_index);
                related = true;
            }

            const NCollection_List<TopoDS_Shape>& generated = builder.Generated(source);
            if (!generated.IsEmpty()) {
                append_generated_shapes(out, result_maps, generated, operand,
                    source_kind, source_index);
                related = true;
            }

            if (!related && result_maps.map(source_kind).FindIndex(source) > 0) {
                append_history_relation(out, result_maps, source,
                    HistoryRelation::Unchanged, operand, source_kind, source_index);
                related = true;
            }

            if (!related || builder.IsDeleted(source)) {
                out.deleted.push_back(operand);
                out.deleted.push_back(static_cast<uint32_t>(source_kind));
                out.deleted.push_back(source_index);
            }
        }
    }
}

static void append_identity_topology_history(
    const TopoDS_Shape& input,
    uint32_t operand,
    const HistoryMaps& result_maps,
    HistoryData& out)
{
    const HistoryMaps input_maps(input);
    for (HistoryKind kind : {HistoryKind::Face, HistoryKind::Edge, HistoryKind::Vertex}) {
        const auto& inputs = input_maps.map(kind);
        const auto& results = result_maps.map(kind);
        const int count = std::min(inputs.Extent(), results.Extent());
        for (int ordinal = 1; ordinal <= count; ++ordinal) {
            append_history_relation(out, result_maps, results(ordinal),
                HistoryRelation::Unchanged, operand, kind,
                static_cast<uint32_t>(ordinal - 1));
        }
    }
}

// Complete operation history only through exact occurrence identity. Unlike
// append_identity_topology_history this never assumes that traversal ordinals
// correspond, so it is safe for builders that reorder or insert topology.
static void append_shared_topology_history(
    const TopoDS_Shape& input,
    uint32_t operand,
    const HistoryMaps& result_maps,
    HistoryData& out)
{
    const HistoryMaps input_maps(input);
    for (HistoryKind kind : {HistoryKind::Face, HistoryKind::Edge, HistoryKind::Vertex}) {
        const auto& inputs = input_maps.map(kind);
        for (int ordinal = 1; ordinal <= inputs.Extent(); ++ordinal) {
            const TopoDS_Shape& source = inputs(ordinal);
            if (result_maps.map(kind).FindIndex(source) <= 0) continue;
            append_history_relation(out, result_maps, source,
                HistoryRelation::Unchanged, operand, kind,
                static_cast<uint32_t>(ordinal - 1));
        }
    }
}

static bool result_has_topology_relation(
    const HistoryData& out,
    HistoryKind result_kind,
    uint32_t result_index)
{
    const uint32_t kind_value = static_cast<uint32_t>(result_kind);
    for (size_t offset = 0; offset + 5 < out.relations.size(); offset += 6) {
        if (out.relations[offset] == kind_value
            && out.relations[offset + 1] == result_index) {
            return true;
        }
    }
    return false;
}

// Some OCCT builders intentionally expose only partial per-subshape history.
// For topology they do not classify, record the explicit operation-level
// generator set rather than guessing a one-to-one relationship by traversal
// order. Consumers can resolve the truthful many-to-many relation with their
// geometric and adjacency discriminators.
static void append_aggregate_generated_history(
    const TopoDS_Shape& input,
    uint32_t operand,
    const HistoryMaps& result_maps,
    HistoryKind result_kind,
    std::initializer_list<HistoryKind> source_kinds,
    HistoryData& out)
{
    const HistoryMaps input_maps(input);
    const auto& results = result_maps.map(result_kind);
    for (int result_ordinal = 1; result_ordinal <= results.Extent(); ++result_ordinal) {
        const uint32_t result_index = static_cast<uint32_t>(result_ordinal - 1);
        if (result_has_topology_relation(out, result_kind, result_index)) continue;
        for (HistoryKind source_kind : source_kinds) {
            const auto& sources = input_maps.map(source_kind);
            for (int source_ordinal = 1; source_ordinal <= sources.Extent(); ++source_ordinal) {
                append_history_relation(out, result_maps, results(result_ordinal),
                    HistoryRelation::Generated, operand, source_kind,
                    static_cast<uint32_t>(source_ordinal - 1));
            }
        }
    }
}

// Record operation-level generators for builder result topology that OCCT did
// not classify. Each input shape has its own operand namespace, which is
// important for loft sections and repeated occurrences that may contain
// ordinally identical topology.
static void append_multi_operand_aggregate_generated_history(
    const std::vector<TopoDS_Shape>& inputs,
    const HistoryMaps& result_maps,
    HistoryKind result_kind,
    std::initializer_list<HistoryKind> source_kinds,
    HistoryData& out)
{
    const auto& results = result_maps.map(result_kind);
    for (int result_ordinal = 1; result_ordinal <= results.Extent(); ++result_ordinal) {
        const uint32_t result_index = static_cast<uint32_t>(result_ordinal - 1);
        if (result_has_topology_relation(out, result_kind, result_index)) continue;
        for (size_t operand = 0; operand < inputs.size(); ++operand) {
            const HistoryMaps input_maps(inputs[operand]);
            for (HistoryKind source_kind : source_kinds) {
                const auto& sources = input_maps.map(source_kind);
                for (int source_ordinal = 1;
                     source_ordinal <= sources.Extent();
                     ++source_ordinal) {
                    append_history_relation(out, result_maps,
                        results(result_ordinal), HistoryRelation::Generated,
                        static_cast<uint32_t>(operand), source_kind,
                        static_cast<uint32_t>(source_ordinal - 1));
                }
            }
        }
    }
}

// Builder APIs can report a source as deleted while omitting the generated
// result that an operation-level fallback subsequently relates to it. A
// related source is not deleted in Plex's semantic history, so remove those
// contradictory tombstones before publishing the result.
static void remove_related_deleted_topology(HistoryData& out) {
    std::vector<uint32_t> retained;
    retained.reserve(out.deleted.size());
    for (size_t deleted_offset = 0; deleted_offset + 2 < out.deleted.size(); deleted_offset += 3) {
        const uint32_t operand = out.deleted[deleted_offset];
        const uint32_t source_kind = out.deleted[deleted_offset + 1];
        const uint32_t source_index = out.deleted[deleted_offset + 2];
        bool related = false;
        for (size_t relation_offset = 0;
             relation_offset + 5 < out.relations.size();
             relation_offset += 6) {
            if (out.relations[relation_offset + 3] == operand
                && out.relations[relation_offset + 4] == source_kind
                && out.relations[relation_offset + 5] == source_index) {
                related = true;
                break;
            }
        }
        if (!related) {
            retained.push_back(operand);
            retained.push_back(source_kind);
            retained.push_back(source_index);
        }
    }
    out.deleted.clear();
    for (uint32_t value : retained) out.deleted.push_back(value);
}

static void finish_topology_history(const HistoryMaps& result_maps, HistoryData& out) {
    for (HistoryKind kind : {HistoryKind::Face, HistoryKind::Edge, HistoryKind::Vertex}) {
        const uint32_t kind_value = static_cast<uint32_t>(kind);
        const auto& results = result_maps.map(kind);
        for (int ordinal = 1; ordinal <= results.Extent(); ++ordinal) {
            const uint32_t result_index = static_cast<uint32_t>(ordinal - 1);
            bool resolved = false;
            for (size_t offset = 0; offset + 5 < out.relations.size(); offset += 6) {
                if (out.relations[offset] == kind_value
                    && out.relations[offset + 1] == result_index) {
                    resolved = true;
                    break;
                }
            }
            if (!resolved) {
                out.unresolved.push_back(kind_value);
                out.unresolved.push_back(result_index);
            }
        }
    }
    out.success = true;
}

// Evaluate any boolean expression in DNF on N solids via BOPAlgo_CellsBuilder.
// 1 回の Perform() で全交差を計算し、clause ごとに AddToResult を呼ぶ。
std::unique_ptr<TopoDS_Shape> builder_cells(
    const std::vector<TopoDS_Shape>& solids,
    rust::Slice<const int64_t> clauses,
    const CancellationToken& progress,
    rust::Vec<uint64_t>& out_history,
    HistoryData& out_topology_history)
{
    try {
        if (solids.empty() || clauses.size() == 0
            || rust_progress_cancelled(progress)) return nullptr;

        // BOPAlgo_CellsBuilder は引数 N≥2 を想定するため、単一 solid の場合は
        // deep copy のみで返す (DNF clause が `[+1, 0]` の単純ケース)。
        if (solids.size() == 1 && clauses.size() == 2 && clauses[0] == 1 && clauses[1] == 0) {
            BRepBuilderAPI_Copy copier(solids[0], true, false);
            auto shape = std::make_unique<TopoDS_Shape>(copier.Shape());
            // No builder: relay_from_pair gives {post → pre==src}; flatten it.
            std::unordered_map<uint64_t, uint64_t> relay;
            relay_from_pair(solids[0], copier.Shape(), relay);
            relay_into_history(&relay, nullptr, out_history);
            const HistoryMaps result_maps(copier.Shape());
            append_identity_topology_history(solids[0], 0, result_maps,
                out_topology_history);
            finish_topology_history(result_maps, out_topology_history);
            return shape;
        }

        BOPAlgo_CellsBuilder cb;
        NCollection_List<TopoDS_Shape> args;
        for (const auto& s : solids) args.Append(s);
        cb.SetArguments(args);
        Handle(RustProgressIndicator) indicator = new RustProgressIndicator(progress);
        cb.Perform(indicator->Start());
        if (cb.HasErrors()) return nullptr;

        const int material = 1;
        NCollection_List<TopoDS_Shape> take, avoid;
        for (size_t i = 0; i < clauses.size(); ++i) {
            if (rust_progress_cancelled(progress)) return nullptr;
            int64_t lit = clauses[i];
            if (lit == 0) {
                if (!take.IsEmpty()) {
                    cb.AddToResult(take, avoid, material);
                }
                take.Clear();
                avoid.Clear();
                continue;
            }
            int64_t idx = (lit > 0 ? lit : -lit) - 1;
            if (idx < 0 || idx >= static_cast<int64_t>(solids.size())) return nullptr;
            if (lit > 0) take.Append(solids[static_cast<size_t>(idx)]);
            else         avoid.Append(solids[static_cast<size_t>(idx)]);
        }
        cb.RemoveInternalBoundaries();
        if (rust_progress_cancelled(progress)) return nullptr;

        std::unordered_map<uint64_t, uint64_t> relay1, relay2;
        for (const auto& s : solids) {
            relay_from_builder(cb, s, relay1);
        }

        const HistoryMaps pre_copy_maps(cb.Shape());
        for (size_t operand = 0; operand < solids.size(); ++operand) {
            append_builder_topology_history(cb, solids[operand],
                static_cast<uint32_t>(operand), pre_copy_maps,
                out_topology_history);
        }

        BRepBuilderAPI_Copy copier(cb.Shape(), true, false);
        auto shape = std::make_unique<TopoDS_Shape>(copier.Shape());
        relay_from_pair(cb.Shape(), copier.Shape(), relay2);
        relay_into_history(&relay1, &relay2, out_history);
        const HistoryMaps post_copy_maps(copier.Shape());
        finish_topology_history(post_copy_maps, out_topology_history);
        return shape;
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return nullptr;
    }
}

// Unify shared faces / collinear edges. `out_history` is populated with
// flat [new_id, old_id, ...] pairs covering every old face that survived
// (either unchanged, or merged into a result face); identical layout to
// `builder_boolean`'s history.
static std::unique_ptr<TopoDS_Shape> builder_clean_impl(
    const TopoDS_Shape& shape,
    const std::vector<uint32_t>& keep_edge_indices,
    rust::Vec<uint64_t>& out_history,
    HistoryData& out_topology_history)
{
    try {
        ShapeUpgrade_UnifySameDomain unifier(shape, true, true, true);
        unifier.AllowInternalEdges(false);
        NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> edges;
        TopExp::MapShapes(shape, TopAbs_EDGE, edges);
        for (uint32_t edge_index : keep_edge_indices) {
            if (edge_index >= static_cast<uint32_t>(edges.Extent())) return nullptr;
            unifier.KeepShape(edges(static_cast<int>(edge_index + 1)));
        }
        unifier.Build();

        auto result = std::make_unique<TopoDS_Shape>(unifier.Shape());
        const HistoryMaps result_maps(*result);

        Handle(BRepTools_History) history = unifier.History();
        if (!history.IsNull()) {
            for (TopExp_Explorer ex(shape, TopAbs_FACE); ex.More(); ex.Next()) {
                const TopoDS_Shape& old_face = ex.Current();
                uint64_t old_id = reinterpret_cast<uint64_t>(old_face.TShape().get());
                if (history->IsRemoved(old_face)) continue;
                const NCollection_List<TopoDS_Shape>& mods = history->Modified(old_face);
                if (mods.IsEmpty()) {
                    // Unchanged: TShape* is the same in the result.
                    out_history.push_back(old_id);
                    out_history.push_back(old_id);
                } else {
                    // Merged: use only the first resulting face (first-found wins).
                    uint64_t new_id = reinterpret_cast<uint64_t>(mods.First().TShape().get());
                    out_history.push_back(new_id);
                    out_history.push_back(old_id);
                }
            }

            const HistoryMaps input_maps(shape);
            for (HistoryKind source_kind : {HistoryKind::Face, HistoryKind::Edge,
                                            HistoryKind::Vertex}) {
                const auto& sources = input_maps.map(source_kind);
                for (int source_ordinal = 1; source_ordinal <= sources.Extent();
                     ++source_ordinal) {
                    const TopoDS_Shape& source = sources(source_ordinal);
                    const uint32_t source_index =
                        static_cast<uint32_t>(source_ordinal - 1);
                    bool related = false;
                    const NCollection_List<TopoDS_Shape>& modified =
                        history->Modified(source);
                    for (NCollection_List<TopoDS_Shape>::Iterator it(modified);
                         it.More(); it.Next()) {
                        append_history_relation(out_topology_history, result_maps,
                            it.Value(), HistoryRelation::Modified, 0, source_kind,
                            source_index);
                        related = true;
                    }
                    const NCollection_List<TopoDS_Shape>& generated =
                        history->Generated(source);
                    if (!generated.IsEmpty()) {
                        append_generated_shapes(out_topology_history, result_maps,
                            generated, 0, source_kind, source_index);
                        related = true;
                    }
                    if (!related
                        && result_maps.map(source_kind).FindIndex(source) > 0) {
                        append_history_relation(out_topology_history, result_maps,
                            source, HistoryRelation::Unchanged, 0, source_kind,
                            source_index);
                        related = true;
                    }
                    if (!related || history->IsRemoved(source)) {
                        out_topology_history.deleted.push_back(0);
                        out_topology_history.deleted.push_back(
                            static_cast<uint32_t>(source_kind));
                        out_topology_history.deleted.push_back(source_index);
                    }
                }
            }
        } else {
            append_identity_topology_history(shape, 0, result_maps,
                out_topology_history);
        }
        finish_topology_history(result_maps, out_topology_history);
        return result;
    } catch (const Standard_Failure& failure) {
        record_standard_failure("builder_clean", "native", 7, failure);
        return nullptr;
    }
}

std::unique_ptr<TopoDS_Shape> builder_clean(
    const TopoDS_Shape& shape,
    rust::Vec<uint64_t>& out_history,
    HistoryData& out_topology_history)
{
    return builder_clean_impl(shape, {}, out_history, out_topology_history);
}

std::unique_ptr<TopoDS_Shape> builder_clean_preserving_edges(
    const TopoDS_Shape& shape,
    rust::Slice<const uint32_t> keep_edge_indices,
    rust::Vec<uint64_t>& out_history,
    HistoryData& out_topology_history)
{
    return builder_clean_impl(shape,
        std::vector<uint32_t>(keep_edge_indices.begin(), keep_edge_indices.end()),
        out_history, out_topology_history);
}

// ==================== Transforms (solid → solid, no history) ====================

std::unique_ptr<TopoDS_Shape> transform_translate(
    const TopoDS_Shape& shape,
    double tx, double ty, double tz)
{
    gp_Trsf trsf;
    trsf.SetTranslation(gp_Vec(tx, ty, tz));
    return std::make_unique<TopoDS_Shape>(shape.Moved(TopLoc_Location(trsf)));
}

std::unique_ptr<TopoDS_Shape> transform_rotate(
    const TopoDS_Shape& shape,
    double ox, double oy, double oz,
    double dx, double dy, double dz,
    double angle)
{
    try {
        gp_Trsf trsf;
        trsf.SetRotation(gp_Ax1(gp_Pnt(ox, oy, oz), gp_Dir(dx, dy, dz)), angle);
        return std::make_unique<TopoDS_Shape>(shape.Moved(TopLoc_Location(trsf)));
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return nullptr;
    }
}

std::unique_ptr<TopoDS_Shape> transform_scale(
    const TopoDS_Shape& shape,
    double cx, double cy, double cz,
    double factor,
    HistoryData& out_topology_history)
{
    try {
        gp_Trsf trsf;
        trsf.SetScale(gp_Pnt(cx, cy, cz), factor);
        BRepBuilderAPI_Transform transform(shape, trsf, true);
        auto result = std::make_unique<TopoDS_Shape>(transform.Shape());
        const HistoryMaps result_maps(*result);
        append_builder_topology_history(transform, shape, 0, result_maps,
            out_topology_history);
        finish_topology_history(result_maps, out_topology_history);
        return result;
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return nullptr;
    }
}

std::unique_ptr<TopoDS_Shape> transform_mirror(
    const TopoDS_Shape& shape,
    double ox, double oy, double oz,
    double nx, double ny, double nz,
    HistoryData& out_topology_history)
{
    try {
        gp_Trsf trsf;
        trsf.SetMirror(gp_Ax2(gp_Pnt(ox, oy, oz), gp_Dir(nx, ny, nz)));
        BRepBuilderAPI_Transform transform(shape, trsf, true);
        auto result = std::make_unique<TopoDS_Shape>(transform.Shape());
        const HistoryMaps result_maps(*result);
        append_builder_topology_history(transform, shape, 0, result_maps,
            out_topology_history);
        finish_topology_history(result_maps, out_topology_history);
        return result;
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return nullptr;
    }
}

// ==================== Shape Queries ====================

bool shape_is_null(const TopoDS_Shape& shape) {
    return shape.IsNull();
}

bool shape_is_solid(const TopoDS_Shape& shape) {
    return !shape.IsNull() && shape.ShapeType() == TopAbs_SOLID;
}

void shape_plane_section(const TopoDS_Shape& shape,
    double ox, double oy, double oz,
    double nx, double ny, double nz,
    double xx, double xy, double xz,
    double deflection,
    PlaneSectionData& out_section) {
    out_section.success = false;
    try {
        gp_Pnt origin(ox, oy, oz);
        gp_Dir normal(nx, ny, nz);
        gp_Dir x_axis(xx, xy, xz);
        gp_Dir y_axis = normal.Crossed(x_axis);
        gp_Pln plane(origin, normal);

        BRepAlgoAPI_Section section(shape, plane, false);
        section.Approximation(true);
        section.Build();
        if (!section.IsDone()) {
            return;
        }

        Handle(TopTools_HSequenceOfShape) edges = new TopTools_HSequenceOfShape();
        for (TopExp_Explorer explorer(section.Shape(), TopAbs_EDGE); explorer.More();
             explorer.Next()) {
            edges->Append(explorer.Current());
        }
        if (edges->Length() == 0) {
            out_section.success = true;  // an empty section is a valid result
            return;
        }
        Handle(TopTools_HSequenceOfShape) wires;
        ShapeAnalysis_FreeBounds::ConnectEdgesToWires(edges, 1.0e-6, false, wires);
        if (wires.IsNull()) {
            return;
        }

        for (int index = 1; index <= wires->Length(); ++index) {
            TopoDS_Wire wire = TopoDS::Wire(wires->Value(index));
            std::vector<double> local;
            gp_Pnt previous;
            bool has_previous = false;
            for (BRepTools_WireExplorer wire_explorer(wire); wire_explorer.More();
                 wire_explorer.Next()) {
                const TopoDS_Edge& edge = wire_explorer.Current();
                BRepAdaptor_Curve curve(edge);
                GCPnts_QuasiUniformDeflection sampler(curve, deflection);
                if (!sampler.IsDone() || sampler.NbPoints() < 2) {
                    continue;
                }
                bool reversed = edge.Orientation() == TopAbs_REVERSED;
                for (int sample = 1; sample <= sampler.NbPoints(); ++sample) {
                    int ordered = reversed ? sampler.NbPoints() + 1 - sample : sample;
                    gp_Pnt point = sampler.Value(ordered);
                    if (has_previous && point.Distance(previous) < 1.0e-9) {
                        continue;
                    }
                    gp_Vec relative(origin, point);
                    local.push_back(relative.Dot(gp_Vec(x_axis)));
                    local.push_back(relative.Dot(gp_Vec(y_axis)));
                    previous = point;
                    has_previous = true;
                }
            }
            std::size_t count = local.size() / 2;
            if (count < 2) {
                continue;
            }
            bool closed = false;
            double gap = std::hypot(
                local[local.size() - 2] - local[0],
                local[local.size() - 1] - local[1]);
            if (gap < 1.0e-6) {
                // Drop the duplicated closing point; closure is reported separately.
                local.pop_back();
                local.pop_back();
                count -= 1;
                closed = true;
            }
            if (count < 2) {
                continue;
            }
            for (double coordinate : local) {
                out_section.points.push_back(coordinate);
            }
            out_section.wire_sizes.push_back(static_cast<uint32_t>(count));
            out_section.wire_closed.push_back(closed ? 1 : 0);
        }
        out_section.success = true;
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        out_section.success = false;
    }
}

void shape_face_boundary_projection(const TopoDS_Shape& shape,
    uint32_t face_index,
    double ox, double oy, double oz,
    double nx, double ny, double nz,
    double xx, double xy, double xz,
    double deflection,
    PlaneSectionData& out_section) {
    out_section.success = false;
    try {
        gp_Pnt origin(ox, oy, oz);
        gp_Dir normal(nx, ny, nz);
        gp_Dir x_axis(xx, xy, xz);
        gp_Dir y_axis = normal.Crossed(x_axis);

        TopoDS_Face face;
        uint32_t index = 0;
        for (TopExp_Explorer explorer(shape, TopAbs_FACE); explorer.More();
             explorer.Next(), ++index) {
            if (index == face_index) {
                face = TopoDS::Face(explorer.Current());
                break;
            }
        }
        if (face.IsNull()) {
            return;
        }

        for (TopExp_Explorer wire_iter(face, TopAbs_WIRE); wire_iter.More();
             wire_iter.Next()) {
            TopoDS_Wire wire = TopoDS::Wire(wire_iter.Current());
            std::vector<double> local;
            gp_Pnt previous;
            bool has_previous = false;
            for (BRepTools_WireExplorer wire_explorer(wire, face); wire_explorer.More();
                 wire_explorer.Next()) {
                const TopoDS_Edge& edge = wire_explorer.Current();
                BRepAdaptor_Curve curve(edge);
                GCPnts_QuasiUniformDeflection sampler(curve, deflection);
                if (!sampler.IsDone() || sampler.NbPoints() < 2) {
                    continue;
                }
                bool reversed = edge.Orientation() == TopAbs_REVERSED;
                for (int sample = 1; sample <= sampler.NbPoints(); ++sample) {
                    int ordered = reversed ? sampler.NbPoints() + 1 - sample : sample;
                    gp_Pnt point = sampler.Value(ordered);
                    if (has_previous && point.Distance(previous) < 1.0e-9) {
                        continue;
                    }
                    // Parallel projection along the plane normal: only the in-plane
                    // coordinates are kept.
                    gp_Vec relative(origin, point);
                    local.push_back(relative.Dot(gp_Vec(x_axis)));
                    local.push_back(relative.Dot(gp_Vec(y_axis)));
                    previous = point;
                    has_previous = true;
                }
            }
            std::size_t count = local.size() / 2;
            if (count < 2) {
                continue;
            }
            bool closed = false;
            double gap = std::hypot(
                local[local.size() - 2] - local[0],
                local[local.size() - 1] - local[1]);
            if (gap < 1.0e-6) {
                local.pop_back();
                local.pop_back();
                count -= 1;
                closed = true;
            }
            if (count < 2) {
                continue;
            }
            for (double coordinate : local) {
                out_section.points.push_back(coordinate);
            }
            out_section.wire_sizes.push_back(static_cast<uint32_t>(count));
            out_section.wire_closed.push_back(closed ? 1 : 0);
        }
        out_section.success = true;
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        out_section.success = false;
    }
}

// Gauss-Kronrod resolves closed conic/spline trims but is expensive at spherical poles.
static double volume_properties(const TopoDS_Shape& shape, GProp_GProps& props,
    bool center = false, bool inertia = false)
{
    for (TopExp_Explorer explorer(shape, TopAbs_EDGE); explorer.More(); explorer.Next()) {
        const TopoDS_Edge edge = TopoDS::Edge(explorer.Current());
        if (!BRep_Tool::Degenerated(edge) && BRep_Tool::IsGeometric(edge)) {
            const auto kind = BRepAdaptor_Curve(edge).GetType();
            if (kind == GeomAbs_Ellipse || kind == GeomAbs_BSplineCurve || kind == GeomAbs_BezierCurve) {
                return BRepGProp::VolumePropertiesGK(shape, props, 1.0e-9, false, true, center, inertia);
            }
        }
    }
    return BRepGProp::VolumeProperties(shape, props, 1.0e-9);
}

double shape_volume(const TopoDS_Shape& shape) {
    GProp_GProps props;
    const double error = volume_properties(shape, props);
    return error >= 0.0 ? props.Mass() : std::numeric_limits<double>::quiet_NaN();
}

double shape_surface_area(const TopoDS_Shape& shape) {
    GProp_GProps props;
    BRepGProp::SurfaceProperties(shape, props, 1.0e-9);
    return props.Mass();
}

void shape_center_of_mass(const TopoDS_Shape& shape,
    double& x, double& y, double& z)
{
    GProp_GProps props;
    const double error = volume_properties(shape, props, true);
    if (!(error >= 0.0)) {
        x = y = z = std::numeric_limits<double>::quiet_NaN();
        return;
    }
    gp_Pnt com = props.CentreOfMass();
    x = com.X(); y = com.Y(); z = com.Z();
}

void shape_inertia_tensor(const TopoDS_Shape& shape,
    double& m00, double& m01, double& m02,
    double& m10, double& m11, double& m12,
    double& m20, double& m21, double& m22)
{
    // OCCT's MatrixOfInertia() is expressed about the center of mass, but the
    // Rust-side API returns the tensor about the world origin so collections
    // can aggregate by plain matrix sum (parallel-axis theorem is already
    // folded in). Shift here with I_world = I_com + m·(|d|² I - d⊗d),
    // where d = COM vector from world origin, m = volume (uniform density).
    GProp_GProps props;
    const double error = volume_properties(shape, props, true, true);
    if (!(error >= 0.0)) {
        m00 = m01 = m02 = m10 = m11 = m12 = m20 = m21 = m22
            = std::numeric_limits<double>::quiet_NaN();
        return;
    }
    gp_Mat ic = props.MatrixOfInertia();
    gp_Pnt com = props.CentreOfMass();
    double mass = props.Mass();
    double dx = com.X(), dy = com.Y(), dz = com.Z();
    double d2 = dx*dx + dy*dy + dz*dz;
    m00 = ic.Value(1,1) + mass * (d2 - dx*dx);
    m11 = ic.Value(2,2) + mass * (d2 - dy*dy);
    m22 = ic.Value(3,3) + mass * (d2 - dz*dz);
    m01 = ic.Value(1,2) - mass * dx * dy;
    m02 = ic.Value(1,3) - mass * dx * dz;
    m12 = ic.Value(2,3) - mass * dy * dz;
    m10 = m01; m20 = m02; m21 = m12;
}

bool shape_contains_point(const TopoDS_Shape& shape, double x, double y, double z) {
    BRepClass3d_SolidClassifier classifier(shape, gp_Pnt(x, y, z), 1e-6);
    return classifier.State() == TopAbs_IN;
}

void shape_bounding_box(const TopoDS_Shape& shape, bool precise,
    double& xmin, double& ymin, double& zmin,
    double& xmax, double& ymax, double& zmax)
{
    Bnd_Box box;
    // Validation needs surface extrema; presentation retains its conservative bounds convention.
    if (precise) BRepBndLib::AddOptimal(shape, box, false, false);
    else BRepBndLib::Add(shape, box, false);
    box.Get(xmin, ymin, zmin, xmax, ymax, zmax);
}

// ==================== Meshing ====================

struct SampledBrepEdge {
    std::vector<double> parameters;
    std::vector<gp_Pnt> points;
};

constexpr size_t maximum_brep_edge_samples = 65'536;

// Exact extraction is an untrusted allocation boundary: imported topology can
// describe arbitrarily many faces, trims, and NURBS coefficients. Bound
// topology-map growth incrementally and account serialized output before Rust
// vectors reserve it. OCCT algorithms can still require temporary working
// memory whose size is not inspectable in advance; allocation failures at that
// boundary are retained as structured resource failures.
constexpr size_t maximum_brep_faces = 65'536;
constexpr size_t maximum_brep_edges = 262'144;
constexpr size_t maximum_brep_vertices = maximum_brep_edges * 2;
constexpr size_t maximum_brep_canonical_samples = 4'194'304;
constexpr size_t maximum_brep_control_points = 2'097'152;
constexpr size_t maximum_brep_knots = 4'194'304;
constexpr size_t maximum_brep_trim_vertices = 8'388'608;
constexpr size_t maximum_brep_serialized_bytes = 256 * 1024 * 1024;

static bool checked_size_add(size_t first, size_t second, size_t& result) {
    if (second > std::numeric_limits<size_t>::max() - first) return false;
    result = first + second;
    return true;
}

static bool checked_size_multiply(size_t first, size_t second, size_t& result) {
    if (first != 0 && second > std::numeric_limits<size_t>::max() / first) {
        return false;
    }
    result = first * second;
    return true;
}

struct BrepExtractionBudget {
    size_t canonical_samples = 0;
    size_t control_points = 0;
    size_t knots = 0;
    size_t trim_vertices = 0;
    // The six offset arrays are seeded with one u32 each before extraction.
    size_t serialized_bytes = 6 * sizeof(uint32_t);

    bool claim(
        size_t& counter,
        size_t amount,
        size_t maximum,
        const char* message)
    {
        size_t next = 0;
        if (!checked_size_add(counter, amount, next) || next > maximum) {
            record_resource_failure("extract_brep_mesh_source", message);
            return false;
        }
        counter = next;
        return true;
    }

    bool claim_canonical_samples(size_t amount) {
        return claim(
            canonical_samples,
            amount,
            maximum_brep_canonical_samples,
            "canonical shared-edge sample quota exceeded");
    }

    bool claim_control_points(size_t amount) {
        return claim(
            control_points,
            amount,
            maximum_brep_control_points,
            "surface control-point quota exceeded");
    }

    bool claim_knots(size_t amount) {
        return claim(
            knots,
            amount,
            maximum_brep_knots,
            "surface knot quota exceeded");
    }

    bool claim_trim_vertices(size_t amount) {
        return claim(
            trim_vertices,
            amount,
            maximum_brep_trim_vertices,
            "trim-vertex quota exceeded");
    }

    template <typename Value>
    bool reserve_serialized_append(
        rust::Vec<Value>& output,
        size_t amount,
        const char* message)
    {
        size_t bytes = 0;
        size_t next_size = 0;
        if (!checked_size_multiply(amount, sizeof(Value), bytes)
            || !checked_size_add(output.size(), amount, next_size)) {
            record_resource_failure(
                "extract_brep_mesh_source",
                "serialized B-rep size arithmetic overflow");
            return false;
        }
        if (!claim(
                serialized_bytes,
                bytes,
                maximum_brep_serialized_bytes,
                message)) {
            return false;
        }
        if (output.capacity() < next_size) output.reserve(next_size);
        return true;
    }
};

using BrepShapeMap =
    NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher>;

static bool map_unique_brep_subshapes_bounded(
    const TopoDS_Shape& shape,
    TopAbs_ShapeEnum shape_kind,
    size_t maximum,
    const char* quota_message,
    const CancellationToken& progress,
    BrepShapeMap& shapes,
    const char* operation = "extract_brep_mesh_source")
{
    for (TopExp_Explorer explorer(shape, shape_kind);
         explorer.More(); explorer.Next()) {
        if (rust_progress_cancelled(progress)) return false;
        const TopoDS_Shape& current = explorer.Current();
        if (shapes.Contains(current)) continue;
        if (static_cast<size_t>(shapes.Extent()) >= maximum) {
            record_resource_failure(operation, quota_message);
            return false;
        }
        shapes.Add(current);
    }
    return true;
}

static Handle(Geom_Surface) unwrapped_brep_surface(
    Handle(Geom_Surface) surface,
    const char* operation = "extract_brep_mesh_source")
{
    constexpr int maximum_wrapper_depth = 16;
    for (int depth = 0; depth < maximum_wrapper_depth; ++depth) {
        Handle(Geom_RectangularTrimmedSurface) trimmed =
            Handle(Geom_RectangularTrimmedSurface)::DownCast(surface);
        if (trimmed.IsNull()) return surface;
        surface = trimmed->BasisSurface();
        if (surface.IsNull()) return surface;
    }
    if (!Handle(Geom_RectangularTrimmedSurface)::DownCast(surface).IsNull()) {
        record_resource_failure(
            operation, "surface wrapper-depth quota exceeded");
        return Handle(Geom_Surface)();
    }
    return surface;
}

static bool preflight_brep_copy_surface_storage(
    const BrepShapeMap& faces,
    size_t maximum_control_points,
    size_t maximum_knots,
    const CancellationToken& progress,
    const char* operation = "extract_brep_mesh_source")
{
    size_t control_points = 0;
    size_t knots = 0;
    // Count every unique face occurrence conservatively. OCCT is free to copy
    // a shared surface once per face during deep-copy normalization, so
    // deduplicating identical surface handles would not bound that allocation.
    for (int index = 1; index <= faces.Extent(); ++index) {
        if (rust_progress_cancelled(progress)) return false;
        Handle(Geom_Surface) surface = unwrapped_brep_surface(
            BRep_Tool::Surface(TopoDS::Face(faces(index))), operation);
        if (surface.IsNull()) return false;
        Handle(Geom_BSplineSurface) spline =
            Handle(Geom_BSplineSurface)::DownCast(surface);
        if (spline.IsNull()) continue;

        const int u_poles = spline->NbUPoles();
        const int v_poles = spline->NbVPoles();
        const int u_knots = spline->NbUKnots();
        const int v_knots = spline->NbVKnots();
        if (u_poles < 1 || v_poles < 1 || u_knots < 1 || v_knots < 1) {
            return false;
        }
        size_t surface_control_points = 0;
        size_t surface_knots = 0;
        size_t next_control_points = 0;
        size_t next_knots = 0;
        if (!checked_size_multiply(
                static_cast<size_t>(u_poles),
                static_cast<size_t>(v_poles),
                surface_control_points)
            || !checked_size_add(
                static_cast<size_t>(u_knots),
                static_cast<size_t>(v_knots),
                surface_knots)
            || !checked_size_add(
                control_points,
                surface_control_points,
                next_control_points)
            || !checked_size_add(knots, surface_knots, next_knots)) {
            record_resource_failure(
                operation, "surface-copy storage size arithmetic overflow");
            return false;
        }
        if (next_control_points > maximum_control_points) {
            record_resource_failure(
                operation, "surface-copy control-point quota exceeded");
            return false;
        }
        if (next_knots > maximum_knots) {
            record_resource_failure(
                operation, "surface-copy knot quota exceeded");
            return false;
        }
        control_points = next_control_points;
        knots = next_knots;
    }
    return true;
}

static double brep_mesh_absolute_deflection(
    const TopoDS_Shape& shape,
    double linear,
    bool relative)
{
    if (!relative) return std::max(linear, Precision::Confusion());
    // Surface area is exact, scale-covariant, and invariant under rigid
    // placement. An axis-aligned bounding-box diagonal changes under rotation
    // and made identical bodies receive different triangle densities.
    GProp_GProps properties;
    BRepGProp::SurfaceProperties(shape, properties, 1.0e-9);
    const double area = std::abs(properties.Mass());
    const double characteristic_length =
        std::isfinite(area) && area > 0.0
        ? std::max(std::sqrt(area), Precision::Confusion())
        : Precision::Confusion();
    return std::max(
        linear * characteristic_length,
        Precision::Confusion());
}

static bool sample_brep_edge(
    const TopoDS_Edge& edge,
    double linear,
    double angular,
    bool bounds_curved_surface,
    BrepExtractionBudget& budget,
    const CancellationToken& progress,
    SampledBrepEdge& sampled)
{
    double first = 0.0;
    double last = 1.0;
    BRep_Tool::Range(edge, first, last);
    if (!std::isfinite(first) || !std::isfinite(last) || last <= first) {
        return false;
    }

    const bool degenerate = BRep_Tool::Degenerated(edge);
    if (degenerate) {
        const int intervals = std::clamp(
            static_cast<int>(std::ceil(
                2.0 * std::acos(-1.0) / std::max(angular, 0.05))),
            4,
            64);
        const TopoDS_Vertex vertex = TopExp::FirstVertex(edge, true);
        if (vertex.IsNull()) return false;
        const gp_Pnt point = BRep_Tool::Pnt(vertex);
        if (!std::isfinite(point.X()) || !std::isfinite(point.Y())
            || !std::isfinite(point.Z())) {
            return false;
        }
        if (!budget.claim_canonical_samples(
                static_cast<size_t>(intervals) + 1)) {
            return false;
        }
        sampled.parameters.reserve(static_cast<size_t>(intervals) + 1);
        sampled.points.reserve(static_cast<size_t>(intervals) + 1);
        for (int index = 0; index <= intervals; ++index) {
            if (rust_progress_cancelled(progress)) return false;
            sampled.parameters.push_back(
                first + (last - first) * static_cast<double>(index)
                    / static_cast<double>(intervals));
            sampled.points.push_back(point);
        }
        return true;
    }

    BRepAdaptor_Curve curve(edge);
    const bool is_straight = curve.GetType() == GeomAbs_Line;
    if (is_straight) {
        // A straight edge needs only its endpoints for geometric accuracy.
        // Sampling it through a world-space deflection calculation makes the
        // initial partition depend on a large rigid translation even though
        // its exact curve parameterization does not change.
        sampled.parameters = {first, last};
        sampled.points = {curve.Value(first), curve.Value(last)};
        for (const gp_Pnt& point : sampled.points) {
            if (!std::isfinite(point.X()) || !std::isfinite(point.Y())
                || !std::isfinite(point.Z())) {
                return false;
            }
        }
    } else {
        GCPnts_TangentialDeflection discretization(
            curve,
            first,
            last,
            std::max(angular, 1.0e-3),
            std::max(linear, Precision::Confusion()),
            2,
            1.0e-10,
            std::max(linear * 1.0e-5, 1.0e-10));
        if (discretization.NbPoints() < 2
            || static_cast<size_t>(discretization.NbPoints())
                > maximum_brep_edge_samples) {
            if (discretization.NbPoints() >= 2) {
                record_resource_failure(
                    "extract_brep_mesh_source",
                    "single-edge sample quota exceeded");
            }
            return false;
        }
        sampled.parameters.reserve(discretization.NbPoints());
        sampled.points.reserve(discretization.NbPoints());
        for (int index = 1; index <= discretization.NbPoints(); ++index) {
            if (rust_progress_cancelled(progress)) return false;
            const double parameter = discretization.Parameter(index);
            if (!std::isfinite(parameter)) return false;
            if (!sampled.parameters.empty()
                && std::abs(parameter - sampled.parameters.back()) <= 1.0e-14) {
                continue;
            }
            sampled.parameters.push_back(parameter);
            const gp_Pnt point = discretization.Value(index);
            if (!std::isfinite(point.X()) || !std::isfinite(point.Y())
                || !std::isfinite(point.Z())) {
                return false;
            }
            sampled.points.push_back(point);
        }
    }
    if (sampled.parameters.size() < 2) return false;
    if (!budget.claim_canonical_samples(sampled.parameters.size())) {
        return false;
    }

    // Chord and angular deflection alone deliberately leave straight edges
    // with just their endpoints. That is sufficient geometrically, but it
    // forces adjacent smooth faces into long needle triangles. Add a bounded,
    // scale-independent longitudinal sampling floor so the shared edge graph
    // also supplies useful aspect-ratio anchors. Because this happens once per
    // topological edge, every incident face receives the exact same samples.
    const double curve_length = GCPnts_AbscissaPoint::Length(curve, first, last);
    if (std::isfinite(curve_length) && curve_length > Precision::Confusion()) {
        // A geometrically straight edge still bounds a two-dimensional face.
        // When that face is curved, leaving the edge at the ordinary chordal
        // sample rate forces a much denser interior row to collapse onto a
        // handful of boundary sites. The resulting transition fan is valid but
        // consists of conspicuous needle triangles. Give only edges incident to
        // curved faces a stronger, bounded longitudinal floor. Planar boxes and
        // polyhedral models retain the inexpensive ordinary edge distribution.
        const double angular_fraction = bounds_curved_surface
            ? std::clamp(angular * 0.25, 1.0 / 32.0, 0.125)
            : std::clamp(angular, 0.02, 0.5);
        const double target_length = std::max(
            linear * 4.0,
            curve_length * angular_fraction);
        if (!std::isfinite(target_length)
            || target_length <= Precision::Confusion()) {
            return false;
        }
        std::vector<double> refined_parameters;
        std::vector<gp_Pnt> refined_points;
        refined_parameters.reserve(sampled.parameters.size());
        refined_points.reserve(sampled.points.size());
        for (size_t interval = 0; interval + 1 < sampled.parameters.size(); ++interval) {
            if (rust_progress_cancelled(progress)) return false;
            const double parameter_start = sampled.parameters[interval];
            const double parameter_end = sampled.parameters[interval + 1];
            const gp_Pnt& point_start = sampled.points[interval];
            const gp_Pnt& point_end = sampled.points[interval + 1];
            if (interval == 0) {
                refined_parameters.push_back(parameter_start);
                refined_points.push_back(point_start);
            }
            // For a line, arc length is affine in its exact parameter. Derive
            // the longitudinal floor from that invariant instead of subtracting
            // two large world-space endpoints. The latter can straddle an
            // integer subdivision boundary after a far rigid placement.
            const double interval_length = is_straight
                ? curve_length * (parameter_end - parameter_start)
                    / (last - first)
                : point_start.Distance(point_end);
            const double subdivision_ratio = interval_length / target_length;
            const double ratio_dead_band = std::max(
                1.0e-10,
                std::abs(subdivision_ratio)
                    * std::numeric_limits<double>::epsilon() * 64.0);
            const double requested_subdivisions = std::ceil(
                subdivision_ratio - ratio_dead_band);
            if (!std::isfinite(requested_subdivisions)) return false;
            const int subdivisions = requested_subdivisions >= 256.0
                ? 256
                : std::max(1, static_cast<int>(requested_subdivisions));
            if (static_cast<size_t>(subdivisions)
                > maximum_brep_edge_samples - refined_parameters.size()) {
                record_resource_failure(
                    "extract_brep_mesh_source",
                    "single-edge sample quota exceeded");
                return false;
            }
            for (int subdivision = 1; subdivision <= subdivisions; ++subdivision) {
                if (rust_progress_cancelled(progress)) return false;
                const double fraction = static_cast<double>(subdivision)
                    / static_cast<double>(subdivisions);
                const double parameter = parameter_start
                    + (parameter_end - parameter_start) * fraction;
                refined_parameters.push_back(parameter);
                refined_points.push_back(
                    subdivision == subdivisions ? point_end : curve.Value(parameter));
            }
        }
        if (refined_parameters.size() > sampled.parameters.size()
            && !budget.claim_canonical_samples(
                refined_parameters.size() - sampled.parameters.size())) {
            return false;
        }
        sampled.parameters = std::move(refined_parameters);
        sampled.points = std::move(refined_points);
    }

    // Canonical topological vertices override independent curve endpoint
    // evaluation so every incident edge uses the same endpoint bits.
    const TopoDS_Vertex first_vertex = TopExp::FirstVertex(edge, false);
    const TopoDS_Vertex last_vertex = TopExp::LastVertex(edge, false);
    if (!first_vertex.IsNull()) sampled.points.front() = BRep_Tool::Pnt(first_vertex);
    if (!last_vertex.IsNull()) sampled.points.back() = BRep_Tool::Pnt(last_vertex);
    return sampled.parameters.size() == sampled.points.size()
        && std::all_of(
            sampled.points.begin(),
            sampled.points.end(),
            [](const gp_Pnt& point) {
                return std::isfinite(point.X()) && std::isfinite(point.Y())
                    && std::isfinite(point.Z());
            });
}

static bool canonical_brep_vertex_point(
    const TopoDS_Vertex& vertex,
    const NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher>& vertices,
    const std::vector<gp_Pnt>& canonical_vertex_points,
    gp_Pnt& point)
{
    if (vertex.IsNull()) return false;
    const int vertex_index = vertices.FindIndex(vertex);
    if (vertex_index < 1
        || static_cast<size_t>(vertex_index)
            > canonical_vertex_points.size()) {
        return false;
    }
    point = canonical_vertex_points[static_cast<size_t>(vertex_index - 1)];
    return std::isfinite(point.X()) && std::isfinite(point.Y())
        && std::isfinite(point.Z());
}

static bool refresh_sampled_brep_edge_points(
    const TopoDS_Edge& edge,
    const NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher>& vertices,
    const std::vector<gp_Pnt>& canonical_vertex_points,
    const CancellationToken& progress,
    SampledBrepEdge& sampled)
{
    if (sampled.parameters.size() < 2
        || !std::is_sorted(sampled.parameters.begin(), sampled.parameters.end())
        || std::adjacent_find(
               sampled.parameters.begin(),
               sampled.parameters.end(),
               [](double first, double second) {
                   return !std::isfinite(first) || !std::isfinite(second)
                       || second <= first;
               }) != sampled.parameters.end()) {
        return false;
    }

    sampled.points.clear();
    sampled.points.reserve(sampled.parameters.size());
    if (BRep_Tool::Degenerated(edge)) {
        const TopoDS_Vertex vertex = TopExp::FirstVertex(edge, true);
        gp_Pnt point;
        if (!canonical_brep_vertex_point(
                vertex, vertices, canonical_vertex_points, point)) {
            return false;
        }
        sampled.points.assign(sampled.parameters.size(), point);
    } else {
        BRepAdaptor_Curve curve(edge);
        for (double parameter : sampled.parameters) {
            if (rust_progress_cancelled(progress)) return false;
            sampled.points.push_back(curve.Value(parameter));
        }
    }

    // Look endpoints up through the copied topological vertex identity, but
    // use the single world-space coordinate captured from its authoritative
    // source vertex. Evaluating the copied edge or vertex independently can
    // differ by a final bit after a rigid placement, which in turn creates a
    // spurious trim vertex at otherwise identical wire corners.
    const TopoDS_Vertex first_vertex = TopExp::FirstVertex(edge, false);
    const TopoDS_Vertex last_vertex = TopExp::LastVertex(edge, false);
    if (!canonical_brep_vertex_point(
            first_vertex,
            vertices,
            canonical_vertex_points,
            sampled.points.front())
        || !canonical_brep_vertex_point(
            last_vertex,
            vertices,
            canonical_vertex_points,
            sampled.points.back())) {
        return false;
    }
    return sampled.parameters.size() == sampled.points.size()
        && std::all_of(
            sampled.points.begin(),
            sampled.points.end(),
            [](const gp_Pnt& point) {
                return std::isfinite(point.X()) && std::isfinite(point.Y())
                    && std::isfinite(point.Z());
            });
}

static bool pcurve_interval_needs_refinement(
    const BRepAdaptor_Curve2d& pcurve,
    const Handle(Geom_Surface)& surface,
    double first,
    double last,
    double u_scale,
    double v_scale,
    double linear,
    double normalized_tolerance)
{
    const double middle = first + (last - first) * 0.5;
    if (!std::isfinite(middle) || middle <= first || middle >= last) {
        return false;
    }
    const gp_Pnt2d first_uv = pcurve.Value(first);
    const gp_Pnt2d middle_uv = pcurve.Value(middle);
    const gp_Pnt2d last_uv = pcurve.Value(last);
    if (!std::isfinite(first_uv.X()) || !std::isfinite(first_uv.Y())
        || !std::isfinite(middle_uv.X()) || !std::isfinite(middle_uv.Y())
        || !std::isfinite(last_uv.X()) || !std::isfinite(last_uv.Y())) {
        return true;
    }

    const gp_Pnt2d chord_middle(
        (first_uv.X() + last_uv.X()) * 0.5,
        (first_uv.Y() + last_uv.Y()) * 0.5);
    const double normalized_u = (middle_uv.X() - chord_middle.X()) / u_scale;
    const double normalized_v = (middle_uv.Y() - chord_middle.Y()) / v_scale;
    const double normalized_deviation = std::hypot(normalized_u, normalized_v);
    if (!std::isfinite(normalized_deviation)
        || normalized_deviation > normalized_tolerance) {
        return true;
    }

    const gp_Pnt exact_middle = surface->Value(middle_uv.X(), middle_uv.Y());
    const gp_Pnt chord_surface_middle =
        surface->Value(chord_middle.X(), chord_middle.Y());
    const double physical_deviation =
        exact_middle.Distance(chord_surface_middle);
    return !std::isfinite(physical_deviation)
        || physical_deviation > std::max(linear * 0.5, Precision::Confusion());
}

static bool parameter_intervals_match(
    double first,
    double last,
    double other_first,
    double other_last)
{
    if (!std::isfinite(first) || !std::isfinite(last)
        || !std::isfinite(other_first) || !std::isfinite(other_last)
        || last <= first || other_last <= other_first) {
        return false;
    }
    const double scale = std::max(
        {std::abs(first),
         std::abs(last),
         std::abs(other_first),
         std::abs(other_last),
         std::abs(last - first),
         std::abs(other_last - other_first),
         1.0});
    const double tolerance = std::max(
        Precision::PConfusion() * 10.0,
        scale * 1.0e-12);
    return std::abs(first - other_first) <= tolerance
        && std::abs(last - other_last) <= tolerance;
}

// Shared edge parameters must describe not only the 3-D edge curve but every
// incident pcurve. A straight-enough 3-D chord can still cut across a strongly
// distorted UV trim if its pcurve bends between the same two parameters. Add
// the union of parameters required by each incident chart, then re-evaluate the
// one canonical 3-D point sequence after all faces have contributed.
static bool refine_sampled_brep_edge_for_pcurve(
    const TopoDS_Edge& edge_use,
    const TopoDS_Face& face,
    double linear,
    double angular,
    BrepExtractionBudget& budget,
    const CancellationToken& progress,
    SampledBrepEdge& sampled)
{
    BRepAdaptor_Curve2d pcurve(edge_use, face);
    const double pcurve_first = pcurve.FirstParameter();
    const double pcurve_last = pcurve.LastParameter();
    if (!std::isfinite(pcurve_first) || !std::isfinite(pcurve_last)
        || pcurve_last <= pcurve_first) {
        return false;
    }
    if (!parameter_intervals_match(
            sampled.parameters.front(),
            sampled.parameters.back(),
            pcurve_first,
            pcurve_last)) {
        return false;
    }

    const Handle(Geom_Surface) surface = BRep_Tool::Surface(face);
    if (surface.IsNull()) return false;
    double u_min = 0.0;
    double u_max = 0.0;
    double v_min = 0.0;
    double v_max = 0.0;
    BRepTools::UVBounds(face, u_min, u_max, v_min, v_max);
    if (!std::isfinite(u_min) || !std::isfinite(u_max)
        || !std::isfinite(v_min) || !std::isfinite(v_max)
        || u_max <= u_min || v_max <= v_min) {
        return false;
    }
    const double u_scale = std::max(u_max - u_min, Precision::PConfusion());
    const double v_scale = std::max(v_max - v_min, Precision::PConfusion());
    const double normalized_tolerance =
        std::clamp(angular * 0.02, 2.5e-4, 1.0e-2);
    constexpr int maximum_passes = 16;

    for (int pass = 0; pass < maximum_passes; ++pass) {
        if (rust_progress_cancelled(progress)) return false;
        std::vector<double> insertions;
        insertions.reserve(sampled.parameters.size());
        for (size_t index = 0; index + 1 < sampled.parameters.size(); ++index) {
            if (rust_progress_cancelled(progress)) return false;
            const double first = sampled.parameters[index];
            const double last = sampled.parameters[index + 1];
            if (pcurve_interval_needs_refinement(
                    pcurve,
                    surface,
                    first,
                    last,
                    u_scale,
                    v_scale,
                    linear,
                    normalized_tolerance)) {
                const double middle = first + (last - first) * 0.5;
                if (!std::isfinite(middle) || middle <= first || middle >= last) {
                    return false;
                }
                insertions.push_back(middle);
            }
        }
        if (insertions.empty()) return true;
        if (sampled.parameters.size() > maximum_brep_edge_samples
            || insertions.size()
                > maximum_brep_edge_samples - sampled.parameters.size()) {
            record_resource_failure(
                "extract_brep_mesh_source",
                "single-edge pcurve refinement quota exceeded");
            return false;
        }
        if (!budget.claim_canonical_samples(insertions.size())) return false;
        sampled.parameters.insert(
            sampled.parameters.end(), insertions.begin(), insertions.end());
        std::sort(sampled.parameters.begin(), sampled.parameters.end());
        sampled.parameters.erase(
            std::unique(sampled.parameters.begin(), sampled.parameters.end()),
            sampled.parameters.end());
    }

    // Never publish a shared boundary whose pcurve still violates the stated
    // refinement criteria after the bounded work budget.
    for (size_t index = 0; index + 1 < sampled.parameters.size(); ++index) {
        if (rust_progress_cancelled(progress)) return false;
        if (pcurve_interval_needs_refinement(
                pcurve,
                surface,
                sampled.parameters[index],
                sampled.parameters[index + 1],
                u_scale,
                v_scale,
                linear,
                normalized_tolerance)) {
            record_resource_failure(
                "extract_brep_mesh_source",
                "single-edge pcurve refinement pass quota exceeded");
            return false;
        }
    }
    return true;
}

static bool append_brep_point(rust::Vec<double>& output, const gp_Pnt& point) {
    if (!std::isfinite(point.X()) || !std::isfinite(point.Y())
        || !std::isfinite(point.Z())) {
        return false;
    }
    output.push_back(point.X());
    output.push_back(point.Y());
    output.push_back(point.Z());
    return true;
}

static bool append_brep_offset(
    rust::Vec<uint32_t>& output,
    size_t value,
    BrepExtractionBudget& budget,
    const char* quota_message)
{
    if (value > static_cast<size_t>(std::numeric_limits<uint32_t>::max())) {
        record_resource_failure(
            "extract_brep_mesh_source",
            "serialized B-rep offset exceeds u32 range");
        return false;
    }
    if (!budget.reserve_serialized_append(output, 1, quota_message)) {
        return false;
    }
    output.push_back(static_cast<uint32_t>(value));
    return true;
}

struct BrepSurfaceChart {
    Handle(Geom_Surface) original;
    Handle(Geom_BSplineSurface) spline;
    double original_u_min = 0.0;
    double original_u_max = 0.0;
    double original_v_min = 0.0;
    double original_v_max = 0.0;
    double spline_u_min = 0.0;
    double spline_u_max = 0.0;
    double spline_v_min = 0.0;
    double spline_v_max = 0.0;
};

struct BrepChartMapState {
    bool has_previous = false;
    gp_Pnt2d original_uv;
    gp_Pnt2d spline_uv;
};

static bool append_bounded_bspline_surface(
    const TopoDS_Face& face,
    BrepMeshSourceData& result,
    BrepSurfaceChart& chart,
    BrepExtractionBudget& budget,
    const CancellationToken& progress)
{
    double u_min, u_max, v_min, v_max;
    BRepTools::UVBounds(face, u_min, u_max, v_min, v_max);
    if (!std::isfinite(u_min) || !std::isfinite(u_max)
        || !std::isfinite(v_min) || !std::isfinite(v_max)
        || u_max <= u_min || v_max <= v_min) {
        return false;
    }
    chart.original = BRep_Tool::Surface(face);
    if (chart.original.IsNull()) return false;
    chart.original_u_min = u_min;
    chart.original_u_max = u_max;
    chart.original_v_min = v_min;
    chart.original_v_max = v_max;
    Handle(Geom_RectangularTrimmedSurface) bounded =
        new Geom_RectangularTrimmedSurface(
            chart.original, u_min, u_max, v_min, v_max, true, true);
    try {
        chart.spline = GeomConvert::SurfaceToBSplineSurface(bounded);
    } catch (const Standard_Failure&) {
        // Approximate surfaces are not an acceptable source for an exact B-rep
        // tessellator. Unsupported surface kinds fail explicitly until their
        // exact representation is added to the Rust-owned evaluator.
        return false;
    }
    if (chart.spline.IsNull()) return false;
    if (chart.spline->IsUPeriodic()) chart.spline->SetUNotPeriodic();
    if (chart.spline->IsVPeriodic()) chart.spline->SetVNotPeriodic();
    chart.spline->Bounds(
        chart.spline_u_min,
        chart.spline_u_max,
        chart.spline_v_min,
        chart.spline_v_max);
    if (!std::isfinite(chart.spline_u_min)
        || !std::isfinite(chart.spline_u_max)
        || !std::isfinite(chart.spline_v_min)
        || !std::isfinite(chart.spline_v_max)
        || chart.spline_u_max <= chart.spline_u_min
        || chart.spline_v_max <= chart.spline_v_min) {
        return false;
    }

    const int u_poles = chart.spline->NbUPoles();
    const int v_poles = chart.spline->NbVPoles();
    const int u_degree = chart.spline->UDegree();
    const int v_degree = chart.spline->VDegree();
    if (u_poles < 2 || v_poles < 2 || u_degree < 1 || v_degree < 1) {
        return false;
    }
    size_t pole_count = 0;
    size_t pole_scalar_count = 0;
    if (!checked_size_multiply(
            static_cast<size_t>(u_poles),
            static_cast<size_t>(v_poles),
            pole_count)
        || !checked_size_multiply(pole_count, 3, pole_scalar_count)) {
        record_resource_failure(
            "extract_brep_mesh_source",
            "surface control-point size arithmetic overflow");
        return false;
    }
    if (!budget.claim_control_points(pole_count)
        || !budget.reserve_serialized_append(
            result.poles,
            pole_scalar_count,
            "serialized surface-pole byte quota exceeded")
        || !budget.reserve_serialized_append(
            result.weights,
            pole_count,
            "serialized surface-weight byte quota exceeded")) {
        return false;
    }
    result.face_u_degrees.push_back(static_cast<uint32_t>(u_degree));
    result.face_v_degrees.push_back(static_cast<uint32_t>(v_degree));
    result.face_u_pole_counts.push_back(static_cast<uint32_t>(u_poles));
    result.face_v_pole_counts.push_back(static_cast<uint32_t>(v_poles));
    for (int v = 1; v <= v_poles; ++v) {
        if (rust_progress_cancelled(progress)) return false;
        for (int u = 1; u <= u_poles; ++u) {
            if (rust_progress_cancelled(progress)) return false;
            const double weight = chart.spline->Weight(u, v);
            if (!append_brep_point(result.poles, chart.spline->Pole(u, v))
                || !std::isfinite(weight) || weight <= 0.0) {
                return false;
            }
            result.weights.push_back(weight);
        }
    }
    if (!append_brep_offset(
            result.face_pole_offsets,
            result.weights.size(),
            budget,
            "serialized surface-offset byte quota exceeded")) {
        return false;
    }
    const auto& u_knots = chart.spline->UKnotSequence();
    const auto& v_knots = chart.spline->VKnotSequence();
    if (u_knots.Length() <= 0 || v_knots.Length() <= 0) return false;
    const size_t u_knot_count = static_cast<size_t>(u_knots.Length());
    const size_t v_knot_count = static_cast<size_t>(v_knots.Length());
    size_t knot_count = 0;
    if (!checked_size_add(u_knot_count, v_knot_count, knot_count)) {
        record_resource_failure(
            "extract_brep_mesh_source",
            "surface knot-count arithmetic overflow");
        return false;
    }
    if (!budget.claim_knots(knot_count)
        || !budget.reserve_serialized_append(
            result.u_knots,
            u_knot_count,
            "serialized U-knot byte quota exceeded")
        || !budget.reserve_serialized_append(
            result.v_knots,
            v_knot_count,
            "serialized V-knot byte quota exceeded")) {
        return false;
    }
    for (int index = u_knots.Lower(); index <= u_knots.Upper(); ++index) {
        if (rust_progress_cancelled(progress)) return false;
        const double knot = u_knots(index);
        if (!std::isfinite(knot)) return false;
        result.u_knots.push_back(knot);
    }
    for (int index = v_knots.Lower(); index <= v_knots.Upper(); ++index) {
        if (rust_progress_cancelled(progress)) return false;
        const double knot = v_knots(index);
        if (!std::isfinite(knot)) return false;
        result.v_knots.push_back(knot);
    }
    if (!append_brep_offset(
            result.face_u_knot_offsets,
            result.u_knots.size(),
            budget,
            "serialized surface-offset byte quota exceeded")
        || !append_brep_offset(
            result.face_v_knot_offsets,
            result.v_knots.size(),
            budget,
            "serialized surface-offset byte quota exceeded")) {
        return false;
    }
    result.face_uv_bounds.push_back(chart.spline_u_min);
    result.face_uv_bounds.push_back(chart.spline_u_max);
    result.face_uv_bounds.push_back(chart.spline_v_min);
    result.face_uv_bounds.push_back(chart.spline_v_max);
    result.face_approximation_errors.push_back(0.0);
    return true;
}

static bool map_to_bspline_chart(
    const BrepSurfaceChart& chart,
    const gp_Pnt2d& original_uv,
    const gp_Pnt& canonical_edge_point,
    BrepChartMapState& state,
    double geometric_tolerance,
    gp_Pnt2d& spline_uv)
{
    if (!std::isfinite(geometric_tolerance)
        || geometric_tolerance < Precision::Confusion()) {
        return false;
    }
    const double parameter_tolerance = Precision::PConfusion() * 10.0;
    const bool on_u_min = std::abs(original_uv.X() - chart.original_u_min)
        <= parameter_tolerance;
    const bool on_u_max = std::abs(original_uv.X() - chart.original_u_max)
        <= parameter_tolerance;
    const bool on_v_min = std::abs(original_uv.Y() - chart.original_v_min)
        <= parameter_tolerance;
    const bool on_v_max = std::abs(original_uv.Y() - chart.original_v_max)
        <= parameter_tolerance;
    if ((on_u_min || on_u_max) && (on_v_min || on_v_max)) {
        spline_uv.SetCoord(
            on_u_min ? chart.spline_u_min : chart.spline_u_max,
            on_v_min ? chart.spline_v_min : chart.spline_v_max);
        state.has_previous = true;
        state.original_uv = original_uv;
        state.spline_uv = spline_uv;
        return true;
    }

    // GeomConvert preserves the normalized parameterization for the exact
    // bounded conversions used here. Prefer that deterministic chart map when
    // it evaluates to the same surface point. Projecting every trim sample
    // independently is both slower and sensitive to the body's world-space
    // placement (for example 5 may come back as 4.999999999999999 after a
    // translation), which can create coincident-but-distinct trim vertices.
    const double original_u_fraction =
        (original_uv.X() - chart.original_u_min)
        / (chart.original_u_max - chart.original_u_min);
    const double original_v_fraction =
        (original_uv.Y() - chart.original_v_min)
        / (chart.original_v_max - chart.original_v_min);
    const gp_Pnt2d normalized_uv(
        chart.spline_u_min
            + original_u_fraction
                * (chart.spline_u_max - chart.spline_u_min),
        chart.spline_v_min
            + original_v_fraction
                * (chart.spline_v_max - chart.spline_v_min));
    const gp_Pnt original_point =
        chart.original->Value(original_uv.X(), original_uv.Y());
    const gp_Pnt normalized_point =
        chart.spline->Value(normalized_uv.X(), normalized_uv.Y());
    const double normalized_error = original_point.Distance(normalized_point);
    // The chart shortcut must fit the canonical edge's budget as well as
    // the source surface's: their existing pcurve error consumes that budget.
    if (std::isfinite(normalized_error)
        && normalized_error <= geometric_tolerance
        && normalized_point.Distance(canonical_edge_point) <= geometric_tolerance) {
        spline_uv = normalized_uv;
        state.has_previous = true;
        state.original_uv = original_uv;
        state.spline_uv = spline_uv;
        return true;
    }

    // A collapsed pole has infinitely many valid parameters along the
    // collapsed direction, so projecting the pole itself loses the authored
    // coordinate. Only move the probe into the chart when the relevant
    // boundary is geometrically collapsed. Treating every V boundary as a
    // pole projected an ordinary tiny sweep's end ellipse from its mid-span
    // instead, producing an error comparable to the part itself.
    const auto collapsed = [geometric_tolerance](
                               const gp_Pnt& first,
                               const gp_Pnt& middle,
                               const gp_Pnt& last) {
        return first.Distance(middle) <= geometric_tolerance
            && first.Distance(last) <= geometric_tolerance
            && middle.Distance(last) <= geometric_tolerance;
    };
    const double original_u_middle =
        (chart.original_u_min + chart.original_u_max) * 0.5;
    const double original_v_middle =
        (chart.original_v_min + chart.original_v_max) * 0.5;
    const bool u_boundary_collapsed = (on_u_min || on_u_max)
        && collapsed(
            chart.original->Value(
                original_uv.X(), chart.original_v_min),
            chart.original->Value(original_uv.X(), original_v_middle),
            chart.original->Value(
                original_uv.X(), chart.original_v_max));
    const bool v_boundary_collapsed = (on_v_min || on_v_max)
        && collapsed(
            chart.original->Value(
                chart.original_u_min, original_uv.Y()),
            chart.original->Value(original_u_middle, original_uv.Y()),
            chart.original->Value(
                chart.original_u_max, original_uv.Y()));
    const double probe_u = u_boundary_collapsed
        ? original_u_middle
        : original_uv.X();
    const double probe_v = v_boundary_collapsed
        ? original_v_middle
        : original_uv.Y();
    double expected_u_fraction = original_u_fraction;
    double expected_v_fraction = original_v_fraction;
    if (state.has_previous) {
        const double previous_original_u_fraction =
            (state.original_uv.X() - chart.original_u_min)
            / (chart.original_u_max - chart.original_u_min);
        const double previous_original_v_fraction =
            (state.original_uv.Y() - chart.original_v_min)
            / (chart.original_v_max - chart.original_v_min);
        const double previous_spline_u_fraction =
            (state.spline_uv.X() - chart.spline_u_min)
            / (chart.spline_u_max - chart.spline_u_min);
        const double previous_spline_v_fraction =
            (state.spline_uv.Y() - chart.spline_v_min)
            / (chart.spline_v_max - chart.spline_v_min);
        expected_u_fraction = previous_spline_u_fraction
            + original_u_fraction - previous_original_u_fraction;
        expected_v_fraction = previous_spline_v_fraction
            + original_v_fraction - previous_original_v_fraction;
    }
    const gp_Pnt2d expected_uv(
        chart.spline_u_min
            + expected_u_fraction
                * (chart.spline_u_max - chart.spline_u_min),
        chart.spline_v_min
            + expected_v_fraction
                * (chart.spline_v_max - chart.spline_v_min));
    const gp_Pnt exact_point = chart.original->Value(probe_u, probe_v);

    // GeomConvert may reparameterize an analytical surface non-linearly. A
    // global closest-point query on a closed conversion can then choose the
    // other copy of a periodic seam even though the previous trim sample
    // proves which local branch is intended. Seed OCCT's continuity-aware
    // surface analyzer with that expected branch before falling back to the
    // exhaustive projector.
    ShapeAnalysis_Surface analysis(chart.spline);
    analysis.SetDomain(
        chart.spline_u_min,
        chart.spline_u_max,
        chart.spline_v_min,
        chart.spline_v_max);
    const gp_Pnt2d local_uv = analysis.NextValueOfUV(
        expected_uv,
        exact_point,
        Precision::Confusion(),
        geometric_tolerance);
    const double local_distance = analysis.Gap();
    const bool local_in_domain = std::isfinite(local_uv.X())
        && std::isfinite(local_uv.Y())
        && local_uv.X() >= chart.spline_u_min - parameter_tolerance
        && local_uv.X() <= chart.spline_u_max + parameter_tolerance
        && local_uv.Y() >= chart.spline_v_min - parameter_tolerance
        && local_uv.Y() <= chart.spline_v_max + parameter_tolerance;
    if (local_in_domain && std::isfinite(local_distance)
        && local_distance <= geometric_tolerance) {
        spline_uv = local_uv;
    } else {
        GeomAPI_ProjectPointOnSurf projection(
            exact_point,
            chart.spline,
            chart.spline_u_min,
            chart.spline_u_max,
            chart.spline_v_min,
            chart.spline_v_max,
            Precision::Confusion());
        if (!projection.IsDone() || projection.NbPoints() < 1) {
            record_stage_failure(
                "extract_brep_mesh_source",
                "extract_face_trim_loops/map_chart/project",
                "surface projection found no B-spline chart coordinate");
            return false;
        }

        double minimum_distance = std::numeric_limits<double>::infinity();
        for (int index = 1; index <= projection.NbPoints(); ++index) {
            const double distance = projection.Distance(index);
            if (std::isfinite(distance)) {
                minimum_distance = std::min(minimum_distance, distance);
            }
        }
        if (!std::isfinite(minimum_distance)) {
            record_stage_failure(
                "extract_brep_mesh_source",
                "extract_face_trim_loops/map_chart/distance",
                "surface projection returned no finite distance");
            return false;
        }
        const double distance_window = std::max(
            Precision::Confusion(), minimum_distance * 1.0e-6);
        double best_score = std::numeric_limits<double>::infinity();
        bool found = false;
        for (int index = 1; index <= projection.NbPoints(); ++index) {
            const double distance = projection.Distance(index);
            if (!std::isfinite(distance)
                || distance > minimum_distance + distance_window) {
                continue;
            }
            double u = 0.0;
            double v = 0.0;
            projection.Parameters(index, u, v);
            if (!std::isfinite(u) || !std::isfinite(v)) continue;
            const double u_fraction = (u - chart.spline_u_min)
                / (chart.spline_u_max - chart.spline_u_min);
            const double v_fraction = (v - chart.spline_v_min)
                / (chart.spline_v_max - chart.spline_v_min);
            const double absolute_score =
                (u_fraction - original_u_fraction) * (u_fraction - original_u_fraction)
                + (v_fraction - original_v_fraction) * (v_fraction - original_v_fraction);
            const double continuity_score =
                (u_fraction - expected_u_fraction) * (u_fraction - expected_u_fraction)
                + (v_fraction - expected_v_fraction) * (v_fraction - expected_v_fraction);
            const double score = state.has_previous
                ? continuity_score * 4.0 + absolute_score
                : absolute_score;
            if (!found || score < best_score) {
                spline_uv.SetCoord(u, v);
                best_score = score;
                found = true;
            }
        }
        if (!found || minimum_distance > geometric_tolerance) {
            const std::string message =
                "surface projection distance " + std::to_string(minimum_distance)
                + " exceeds chart-mapping tolerance "
                + std::to_string(geometric_tolerance)
                + "; normalized-map error " + std::to_string(normalized_error)
                + "; original UV (" + std::to_string(original_uv.X())
                + ", " + std::to_string(original_uv.Y()) + ")"
                + "; projected UV (" + std::to_string(spline_uv.X())
                + ", " + std::to_string(spline_uv.Y()) + ")"
                + "; original bounds ["
                + std::to_string(chart.original_u_min) + ", "
                + std::to_string(chart.original_u_max) + ", "
                + std::to_string(chart.original_v_min) + ", "
                + std::to_string(chart.original_v_max) + "]"
                + "; spline bounds ["
                + std::to_string(chart.spline_u_min) + ", "
                + std::to_string(chart.spline_u_max) + ", "
                + std::to_string(chart.spline_v_min) + ", "
                + std::to_string(chart.spline_v_max) + "]";
            record_stage_failure(
                "extract_brep_mesh_source",
                "extract_face_trim_loops/map_chart/accuracy",
                message.c_str());
            return false;
        }
    }
    if (on_u_min) spline_uv.SetX(chart.spline_u_min);
    if (on_u_max) spline_uv.SetX(chart.spline_u_max);
    if (on_v_min) spline_uv.SetY(chart.spline_v_min);
    if (on_v_max) spline_uv.SetY(chart.spline_v_max);
    if (spline_uv.X() < chart.spline_u_min - parameter_tolerance
        || spline_uv.X() > chart.spline_u_max + parameter_tolerance
        || spline_uv.Y() < chart.spline_v_min - parameter_tolerance
        || spline_uv.Y() > chart.spline_v_max + parameter_tolerance) {
        record_stage_failure(
            "extract_brep_mesh_source",
            "extract_face_trim_loops/map_chart/domain",
            "surface projection lies outside the bounded B-spline chart");
        return false;
    }
    if (state.has_previous) {
        const double original_step_u =
            (original_uv.X() - state.original_uv.X())
            / (chart.original_u_max - chart.original_u_min);
        const double original_step_v =
            (original_uv.Y() - state.original_uv.Y())
            / (chart.original_v_max - chart.original_v_min);
        const double spline_step_u =
            (spline_uv.X() - state.spline_uv.X())
            / (chart.spline_u_max - chart.spline_u_min);
        const double spline_step_v =
            (spline_uv.Y() - state.spline_uv.Y())
            / (chart.spline_v_max - chart.spline_v_min);
        const double original_step = std::hypot(original_step_u, original_step_v);
        const double spline_step = std::hypot(spline_step_u, spline_step_v);
        // Adjacent samples on one pcurve must remain adjacent after chart
        // conversion. A large jump with a small authored step indicates that
        // closest-point projection switched to another periodic branch.
        if (!std::isfinite(original_step) || !std::isfinite(spline_step)
            || (original_step < 0.25
                && spline_step > std::max(0.35, original_step * 8.0 + 0.02))) {
            record_stage_failure(
                "extract_brep_mesh_source",
                "extract_face_trim_loops/map_chart/continuity",
                "surface projection switched to a discontinuous periodic branch");
            return false;
        }
    }
    state.has_previous = true;
    state.original_uv = original_uv;
    state.spline_uv = spline_uv;
    return true;
}

static bool is_internal_brep_wire(const TopoDS_Wire& wire) {
    if (wire.Orientation() == TopAbs_INTERNAL) return true;
    bool has_edge = false;
    for (TopExp_Explorer explorer(wire, TopAbs_EDGE);
         explorer.More(); explorer.Next()) {
        if (explorer.Current().Orientation() != TopAbs_INTERNAL) return false;
        has_edge = true;
    }
    return has_edge;
}

static bool append_face_trim_loops(
    const TopoDS_Face& face,
    const BrepSurfaceChart& chart,
    const NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher>& edges,
    const std::vector<uint32_t>& copied_edge_to_source_index,
    const std::vector<SampledBrepEdge>& sampled_edges,
    double linear,
    double& maximum_boundary_error,
    BrepMeshSourceData& result,
    BrepExtractionBudget& budget,
    const CancellationToken& progress)
{
    const auto fail = [](const char* stage, const char* message) {
        record_stage_failure("extract_brep_mesh_source", stage, message);
        return false;
    };
    if (copied_edge_to_source_index.size()
        != static_cast<size_t>(edges.Extent())) {
        return fail(
            "extract_face_trim_loops/map_edges",
            "detached edge map does not match the source edge map");
    }
    maximum_boundary_error = 0.0;
    const double maximum_allowed_boundary_error =
        std::max(linear * 0.10, Precision::Confusion() * 100.0);
    for (TopExp_Explorer wire_iterator(face, TopAbs_WIRE);
         wire_iterator.More(); wire_iterator.Next()) {
        if (rust_progress_cancelled(progress)) return false;
        const TopoDS_Wire wire = TopoDS::Wire(wire_iterator.Current());
        // Internal wires lie inside the face and do not trim its surface domain.
        if (is_internal_brep_wire(wire)) continue;
        if (wire.Orientation() != TopAbs_FORWARD
            && wire.Orientation() != TopAbs_REVERSED) {
            return fail(
                "extract_face_trim_loops/wire_orientation",
                "face wire has no traversable orientation");
        }
        BRepTools_WireExplorer explorer(wire, face);
        const size_t loop_start = result.loop_edge_indices.size();
        uint32_t edge_occurrence_index = 0;
        gp_Pnt2d previous;
        bool has_previous = false;
        gp_Pnt first_edge_start;
        gp_Pnt previous_edge_end;
        bool has_edge = false;
        for (; explorer.More(); explorer.Next()) {
            if (rust_progress_cancelled(progress)) return false;
            const TopoDS_Edge edge_use = explorer.Current();
            const int mapped_index = edges.FindIndex(edge_use);
            if (mapped_index < 1) {
                return fail(
                    "extract_face_trim_loops/map_edge_occurrence",
                    "face-loop edge occurrence is absent from detached topology");
            }
            const uint32_t edge_index = copied_edge_to_source_index[
                static_cast<size_t>(mapped_index - 1)];
            if (edge_index >= sampled_edges.size()) {
                return fail(
                    "extract_face_trim_loops/map_edge_occurrence",
                    "face-loop edge occurrence maps outside canonical samples");
            }
            const auto& sampled = sampled_edges[edge_index];
            if (sampled.parameters.size() < 2) {
                return fail(
                    "extract_face_trim_loops/canonical_samples",
                    "canonical edge occurrence has fewer than two samples");
            }

            if (edge_use.Orientation() != TopAbs_FORWARD
                && edge_use.Orientation() != TopAbs_REVERSED) {
                return fail(
                    "extract_face_trim_loops/edge_orientation",
                    "face-loop edge occurrence has no traversal direction");
            }

            BRepAdaptor_Curve2d pcurve(edge_use, face);
            const double pcurve_first = pcurve.FirstParameter();
            const double pcurve_last = pcurve.LastParameter();
            if (!std::isfinite(pcurve_first) || !std::isfinite(pcurve_last)
                || pcurve_last <= pcurve_first) {
                return fail(
                    "extract_face_trim_loops/pcurve_domain",
                    "edge pcurve has an invalid parameter interval");
            }
            if (!parameter_intervals_match(
                    sampled.parameters.front(),
                    sampled.parameters.back(),
                    pcurve_first,
                    pcurve_last)) {
                return fail(
                    "extract_face_trim_loops/pcurve_domain",
                    "edge and pcurve parameter intervals disagree");
            }
            const bool reversed = edge_use.Orientation() == TopAbs_REVERSED;
            const uint8_t occurrence_direction = reversed ? 1 : 0;
            const size_t oriented_start_index = reversed
                ? sampled.points.size() - 1
                : 0;
            const size_t oriented_end_index = reversed
                ? 0
                : sampled.points.size() - 1;
            const gp_Pnt& edge_start = sampled.points[oriented_start_index];
            const gp_Pnt& edge_end = sampled.points[oriented_end_index];
            if (!has_edge) {
                first_edge_start = edge_start;
                has_edge = true;
            } else {
                const double connection_error =
                    previous_edge_end.Distance(edge_start);
                if (!std::isfinite(connection_error)
                    || connection_error > maximum_allowed_boundary_error) {
                    return fail(
                        "extract_face_trim_loops/connect_edges",
                        "consecutive canonical edge occurrences do not meet within tolerance");
                }
                maximum_boundary_error =
                    std::max(maximum_boundary_error, connection_error);
            }
            BrepChartMapState map_state;
            for (size_t ordinal = 0; ordinal < sampled.parameters.size(); ++ordinal) {
                if (rust_progress_cancelled(progress)) return false;
                const size_t sample_index = reversed
                    ? sampled.parameters.size() - 1 - ordinal
                    : ordinal;
                // The detached extraction copy is normalized with
                // BRepLib::SameParameter before sampling. The canonical 3D
                // edge and every incident pcurve therefore share this exact
                // parameter; a range remap would be geometrically invalid for
                // a non-linear curve representation.
                const double parameter = sampled.parameters[sample_index];
                const gp_Pnt2d original_uv = pcurve.Value(parameter);
                gp_Pnt2d uv;
                if (!std::isfinite(original_uv.X())
                    || !std::isfinite(original_uv.Y())
                    || !map_to_bspline_chart(
                        chart,
                        original_uv,
                        sampled.points[sample_index],
                        map_state,
                        maximum_allowed_boundary_error,
                        uv)) {
                    return fail(
                        "extract_face_trim_loops/map_chart",
                        "edge pcurve could not be mapped into the bounded B-spline chart");
                }
                if (!std::isfinite(uv.X()) || !std::isfinite(uv.Y())) {
                    return fail(
                        "extract_face_trim_loops/map_chart",
                        "mapped edge pcurve contains a non-finite coordinate");
                }
                const gp_Pnt original_surface_point =
                    chart.original->Value(original_uv.X(), original_uv.Y());
                const gp_Pnt spline_surface_point =
                    chart.spline->Value(uv.X(), uv.Y());
                const gp_Pnt& canonical_edge_point = sampled.points[sample_index];
                const double original_error =
                    original_surface_point.Distance(canonical_edge_point);
                const double spline_error =
                    spline_surface_point.Distance(canonical_edge_point);
                const double conversion_error =
                    spline_surface_point.Distance(original_surface_point);
                const double boundary_error =
                    std::max({original_error, spline_error, conversion_error});
                if (!std::isfinite(boundary_error)
                    || boundary_error > maximum_allowed_boundary_error) {
                    return fail(
                        "extract_face_trim_loops/validate_boundary",
                        ("surface chart and canonical edge disagree beyond tolerance: edge "
                            + std::to_string(edge_index) + ", sample "
                            + std::to_string(sample_index) + ", original "
                            + std::to_string(original_error) + ", spline "
                            + std::to_string(spline_error) + ", conversion "
                            + std::to_string(conversion_error) + ", allowed "
                            + std::to_string(maximum_allowed_boundary_error)).c_str());
                }
                maximum_boundary_error =
                    std::max(maximum_boundary_error, boundary_error);
                // The next oriented edge contributes this terminal vertex to
                // the trim loop, but validate it against this edge and chart
                // before omitting the duplicate from the serialized loop.
                if (ordinal + 1 == sampled.parameters.size()) continue;
                if (has_previous && uv.Distance(previous) <= Precision::PConfusion()) {
                    continue;
                }
                if (!budget.claim_trim_vertices(1)
                    || !budget.reserve_serialized_append(
                        result.loop_uvs,
                        2,
                        "serialized trim-coordinate byte quota exceeded")
                    || !budget.reserve_serialized_append(
                        result.loop_edge_indices,
                        1,
                        "serialized trim-edge byte quota exceeded")
                    || !budget.reserve_serialized_append(
                        result.loop_edge_sample_indices,
                        1,
                        "serialized trim-sample byte quota exceeded")
                    || !budget.reserve_serialized_append(
                        result.loop_edge_occurrence_indices,
                        1,
                        "serialized trim-occurrence byte quota exceeded")
                    || !budget.reserve_serialized_append(
                        result.loop_edge_occurrence_directions,
                        1,
                        "serialized trim-direction byte quota exceeded")) {
                    return false;
                }
                result.loop_uvs.push_back(uv.X());
                result.loop_uvs.push_back(uv.Y());
                result.loop_edge_indices.push_back(edge_index);
                if (sample_index
                    > static_cast<size_t>(std::numeric_limits<uint32_t>::max())) {
                    return fail(
                        "extract_face_trim_loops/serialize_sample",
                        "canonical edge sample ordinal exceeds u32 range");
                }
                result.loop_edge_sample_indices.push_back(static_cast<uint32_t>(sample_index));
                result.loop_edge_occurrence_indices.push_back(
                    edge_occurrence_index);
                result.loop_edge_occurrence_directions.push_back(
                    occurrence_direction);
                previous = uv;
                has_previous = true;
            }
            if (edge_occurrence_index
                == std::numeric_limits<uint32_t>::max()) {
                return fail(
                    "extract_face_trim_loops/serialize_occurrence",
                    "face-loop edge occurrence ordinal exceeds u32 range");
            }
            ++edge_occurrence_index;
            previous_edge_end = edge_end;
        }
        const double closure_error = has_edge
            ? previous_edge_end.Distance(first_edge_start)
            : std::numeric_limits<double>::infinity();
        if (!std::isfinite(closure_error)
            || closure_error > maximum_allowed_boundary_error
            || result.loop_edge_indices.size() - loop_start < 3) {
            return fail(
                "extract_face_trim_loops/close_loop",
                "face-loop boundary does not close with at least three distinct vertices");
        }
        maximum_boundary_error =
            std::max(maximum_boundary_error, closure_error);
        if (!append_brep_offset(
                result.loop_vertex_offsets,
                result.loop_edge_indices.size(),
                budget,
                "serialized trim-loop offset byte quota exceeded")) {
            return false;
        }
    }
    return append_brep_offset(
        result.face_loop_offsets,
        result.loop_vertex_offsets.size() - 1,
        budget,
        "serialized face-loop offset byte quota exceeded");
}

BrepMeshSourceData extract_brep_mesh_source(
    const TopoDS_Shape& shape,
    rust::Slice<const uint32_t> face_indices,
    double linear,
    double angular,
    bool relative,
    const CancellationToken& progress)
{
    BrepMeshSourceData result;
    result.success = false;
    const char* failure_stage = "validate_input";
    ScopedFailureDiagnostic failure_diagnostic(
        __func__, failure_stage, result.success, progress);
    result.face_pole_offsets.push_back(0);
    result.face_u_knot_offsets.push_back(0);
    result.face_v_knot_offsets.push_back(0);
    result.face_loop_offsets.push_back(0);
    result.loop_vertex_offsets.push_back(0);
    result.edge_point_offsets.push_back(0);
    try {
        if (rust_progress_cancelled(progress)) return result;
        if (shape.IsNull() || !std::isfinite(linear) || linear <= 0.0
            || !std::isfinite(angular) || angular <= 0.0) {
            record_input_failure(
                __func__,
                "shape and tessellation tolerances must be valid");
            return result;
        }
        if (face_indices.size() > maximum_brep_faces) {
            record_resource_failure(
                __func__, "requested face-selection quota exceeded");
            return result;
        }
        BrepExtractionBudget budget;
        failure_stage = "map_source_topology";
        BrepShapeMap source_faces;
        BrepShapeMap source_edges;
        BrepShapeMap source_vertices;
        if (!map_unique_brep_subshapes_bounded(
                shape,
                TopAbs_FACE,
                maximum_brep_faces,
                "shape face quota exceeded",
                progress,
                source_faces)
            || !map_unique_brep_subshapes_bounded(
                shape,
                TopAbs_EDGE,
                maximum_brep_edges,
                "shape edge quota exceeded",
                progress,
                source_edges)
            || !map_unique_brep_subshapes_bounded(
                shape,
                TopAbs_VERTEX,
                maximum_brep_vertices,
                "shape vertex quota exceeded",
                progress,
                source_vertices)) {
            return result;
        }

        failure_stage = "preflight_copy_surface_storage";
        if (!preflight_brep_copy_surface_storage(
                source_faces,
                maximum_brep_control_points,
                maximum_brep_knots,
                progress)) {
            return result;
        }

        // Curve construction and SameParameter may repair imported B-reps.
        // They therefore run only on a detached copy, never Plex's authoritative
        // shape, and use a tolerance bounded by the requested tessellation
        // deflection and the kernel's numeric floor. Boundary validation below
        // rejects any repaired representation that exceeds that same policy.
        // Existing OCCT triangulation is deliberately not copied.
        failure_stage = "copy_shape";
        BRepBuilderAPI_Copy copier(shape, true, false);
        TopoDS_Shape extraction_shape = copier.Shape();
        if (extraction_shape.IsNull()) return result;
        std::vector<TopoDS_Face> extraction_faces_by_source_index;
        extraction_faces_by_source_index.reserve(
            static_cast<size_t>(source_faces.Extent()));
        for (int index = 1; index <= source_faces.Extent(); ++index) {
            if (rust_progress_cancelled(progress)) return result;
            const TopoDS_Shape copied_face = copier.ModifiedShape(
                source_faces(index));
            if (copied_face.IsNull()
                || copied_face.ShapeType() != TopAbs_FACE) {
                return result;
            }
            extraction_faces_by_source_index.push_back(
                TopoDS::Face(copied_face));
        }
        const double absolute_linear = brep_mesh_absolute_deflection(
            extraction_shape, linear, relative);
        if (!std::isfinite(absolute_linear) || absolute_linear <= 0.0) {
            return result;
        }
        result.linear_deflection = absolute_linear;
        const double edge_normalization_tolerance =
            std::max(absolute_linear * 0.05, Precision::Confusion());
        // Build missing 3D curves only for ordinary edges; collapsed pcurve-only
        // edges become invalid if the aggregate BuildCurves3d fills them in.
        failure_stage = "normalize_edges";
        BrepShapeMap edges;
        if (!map_unique_brep_subshapes_bounded(
                extraction_shape,
                TopAbs_EDGE,
                maximum_brep_edges,
                "detached shape edge quota exceeded",
                progress,
                edges)) {
            return result;
        }
        for (int index = 1; index <= edges.Extent(); ++index) {
            if (rust_progress_cancelled(progress)) return result;
            const TopoDS_Edge edge = TopoDS::Edge(edges(index));
            if (!BRep_Tool::Degenerated(edge)
                && !BRepLib::BuildCurve3d(
                    edge,
                    edge_normalization_tolerance,
                    GeomAbs_C1,
                    14,
                    0)) {
                return result;
            }
        }
        BRepLib::SameParameter(
            extraction_shape,
            edge_normalization_tolerance,
            true);
        for (int index = 1; index <= edges.Extent(); ++index) {
            if (rust_progress_cancelled(progress)) return result;
            const TopoDS_Edge edge = TopoDS::Edge(edges(index));
            if (!BRep_Tool::Degenerated(edge)
                && (!BRep_Tool::SameRange(edge)
                    || !BRep_Tool::SameParameter(edge))) {
                return result;
            }
        }

        failure_stage = "map_detached_topology";
        BrepShapeMap faces;
        BrepShapeMap vertices;
        if (!map_unique_brep_subshapes_bounded(
                extraction_shape,
                TopAbs_FACE,
                maximum_brep_faces,
                "detached shape face quota exceeded",
                progress,
                faces)
            || !map_unique_brep_subshapes_bounded(
                extraction_shape,
                TopAbs_VERTEX,
                maximum_brep_vertices,
                "detached shape vertex quota exceeded",
                progress,
                vertices)) {
            return result;
        }
        if (faces.Extent() != source_faces.Extent()) return result;
        if (edges.Extent() != source_edges.Extent()) return result;
        if (vertices.Extent() != source_vertices.Extent()) return result;
        failure_stage = "map_copy_identity";
        std::vector<gp_Pnt> canonical_vertex_points(
            static_cast<size_t>(vertices.Extent()));
        std::vector<bool> copied_vertex_seen(
            static_cast<size_t>(vertices.Extent()), false);
        for (int index = 1; index <= source_vertices.Extent(); ++index) {
            if (rust_progress_cancelled(progress)) return result;
            const TopoDS_Shape copied_vertex = copier.ModifiedShape(
                source_vertices(index));
            if (copied_vertex.IsNull()
                || copied_vertex.ShapeType() != TopAbs_VERTEX) {
                return result;
            }
            const int copied_ordinal = vertices.FindIndex(copied_vertex);
            if (copied_ordinal < 1
                || copied_vertex_seen[static_cast<size_t>(copied_ordinal - 1)]) {
                return result;
            }
            const gp_Pnt point = BRep_Tool::Pnt(
                TopoDS::Vertex(source_vertices(index)));
            if (!std::isfinite(point.X()) || !std::isfinite(point.Y())
                || !std::isfinite(point.Z())) {
                return result;
            }
            canonical_vertex_points[static_cast<size_t>(copied_ordinal - 1)] =
                point;
            copied_vertex_seen[static_cast<size_t>(copied_ordinal - 1)] = true;
        }
        if (std::find(
                copied_vertex_seen.begin(), copied_vertex_seen.end(), false)
            != copied_vertex_seen.end()) {
            return result;
        }
        std::vector<int> copied_edge_ordinals_by_source_index(
            static_cast<size_t>(source_edges.Extent()), 0);
        std::vector<uint32_t> copied_edge_to_source_index(
            static_cast<size_t>(edges.Extent()),
            std::numeric_limits<uint32_t>::max());
        for (int index = 1; index <= source_edges.Extent(); ++index) {
            if (rust_progress_cancelled(progress)) return result;
            const TopoDS_Shape copied_edge = copier.ModifiedShape(
                source_edges(index));
            if (copied_edge.IsNull()
                || copied_edge.ShapeType() != TopAbs_EDGE) {
                return result;
            }
            const int copied_ordinal = edges.FindIndex(copied_edge);
            if (copied_ordinal < 1
                || copied_edge_to_source_index[
                       static_cast<size_t>(copied_ordinal - 1)]
                    != std::numeric_limits<uint32_t>::max()) {
                return result;
            }
            copied_edge_ordinals_by_source_index[
                static_cast<size_t>(index - 1)] = copied_ordinal;
            copied_edge_to_source_index[
                static_cast<size_t>(copied_ordinal - 1)] =
                static_cast<uint32_t>(index - 1);
        }
        if (std::find(
                copied_edge_to_source_index.begin(),
                copied_edge_to_source_index.end(),
                std::numeric_limits<uint32_t>::max())
            != copied_edge_to_source_index.end()) {
            return result;
        }
        std::unordered_set<int> copied_face_ordinals;
        copied_face_ordinals.reserve(extraction_faces_by_source_index.size());
        for (TopoDS_Face& face : extraction_faces_by_source_index) {
            if (rust_progress_cancelled(progress)) return result;
            const int copied_ordinal = faces.FindIndex(face);
            if (copied_ordinal < 1
                || !copied_face_ordinals.insert(copied_ordinal).second) {
                return result;
            }
            // ModifiedShape identifies the copied TShape, but a detached
            // subshape does not necessarily carry the orientation of its
            // occurrence in the copied shell. Preserve source-index ordering
            // while serializing the mapped occurrence from the extraction
            // shape so face winding remains globally consistent.
            face = TopoDS::Face(faces(copied_ordinal));
        }

        std::vector<SampledBrepEdge> sampled_edges(
            static_cast<size_t>(edges.Extent()));
        std::vector<bool> edge_bounds_curved_surface(
            static_cast<size_t>(source_edges.Extent()), false);
        for (int face_index = 1; face_index <= faces.Extent(); ++face_index) {
            if (rust_progress_cancelled(progress)) return result;
            const TopoDS_Face face = TopoDS::Face(faces(face_index));
            BRepAdaptor_Surface surface(face, true);
            if (surface.GetType() == GeomAbs_Plane) continue;
            for (TopExp_Explorer explorer(face, TopAbs_EDGE);
                 explorer.More(); explorer.Next()) {
                const int copied_ordinal = edges.FindIndex(explorer.Current());
                if (copied_ordinal < 1) return result;
                const uint32_t source_index = copied_edge_to_source_index[
                    static_cast<size_t>(copied_ordinal - 1)];
                if (source_index >= edge_bounds_curved_surface.size()) {
                    return result;
                }
                edge_bounds_curved_surface[source_index] = true;
            }
        }
        // A surface triangle spans both the edge direction and an interior
        // direction, so consuming the entire requested deviation on its
        // boundary can make the combined diagonal exceed that request. Give
        // canonical edges half of each face-level error budget; all incident
        // faces still reuse this one deterministic sample sequence.
        const double edge_linear =
            std::max(absolute_linear * 0.5, Precision::Confusion());
        const double edge_angular = std::max(angular * 0.5, 1.0e-3);
        failure_stage = "sample_canonical_edges";
        for (int index = 1; index <= source_edges.Extent(); ++index) {
            if (rust_progress_cancelled(progress)) return result;
            const int copied_ordinal = copied_edge_ordinals_by_source_index[
                static_cast<size_t>(index - 1)];
            // Canonical samples follow curve parameters, independently of face-use orientation.
            const TopoDS_Edge edge = TopoDS::Edge(
                edges(copied_ordinal).Oriented(TopAbs_FORWARD));
            if (!sample_brep_edge(
                    edge,
                    edge_linear,
                    edge_angular,
                    edge_bounds_curved_surface[static_cast<size_t>(index - 1)],
                    budget,
                    progress,
                    sampled_edges[static_cast<size_t>(index - 1)])) {
                return result;
            }
        }
        // Refine every canonical edge with the union of parameters demanded by
        // all incident face pcurves. This intentionally covers the whole
        // extraction shape even when only selected face chunks were requested,
        // so independently requested chunks cannot disagree at a shared seam.
        // Repeat to a fixed point: samples inserted for a later incident chart
        // split intervals that an earlier chart must be allowed to re-check.
        constexpr int maximum_incident_chart_passes = 8;
        bool incident_charts_stable = false;
        failure_stage = "refine_incident_pcurves";
        for (int pass = 0; pass < maximum_incident_chart_passes; ++pass) {
            size_t sample_count_before = 0;
            for (const auto& sampled : sampled_edges) {
                if (sampled.parameters.size()
                    > std::numeric_limits<size_t>::max() - sample_count_before) {
                    return result;
                }
                sample_count_before += sampled.parameters.size();
            }
            for (int face_index = 1; face_index <= faces.Extent(); ++face_index) {
                if (rust_progress_cancelled(progress)) return result;
                const TopoDS_Face face = TopoDS::Face(faces(face_index));
                if (face.Orientation() != TopAbs_FORWARD
                    && face.Orientation() != TopAbs_REVERSED) {
                    return result;
                }
                for (TopExp_Explorer wire_iterator(face, TopAbs_WIRE);
                     wire_iterator.More(); wire_iterator.Next()) {
                    const TopoDS_Wire wire = TopoDS::Wire(wire_iterator.Current());
                    if (is_internal_brep_wire(wire)) continue;
                    if (wire.Orientation() != TopAbs_FORWARD
                        && wire.Orientation() != TopAbs_REVERSED) {
                        return result;
                    }
                    for (BRepTools_WireExplorer explorer(wire, face);
                         explorer.More(); explorer.Next()) {
                        const TopoDS_Edge edge_use = explorer.Current();
                        if (edge_use.Orientation() != TopAbs_FORWARD
                            && edge_use.Orientation() != TopAbs_REVERSED) {
                            return result;
                        }
                        const int mapped_index = edges.FindIndex(edge_use);
                        if (mapped_index < 1
                            || !refine_sampled_brep_edge_for_pcurve(
                                edge_use,
                                face,
                                absolute_linear,
                                angular,
                                budget,
                                progress,
                                sampled_edges[copied_edge_to_source_index[
                                    static_cast<size_t>(mapped_index - 1)]])) {
                            return result;
                        }
                    }
                }
            }
            size_t sample_count_after = 0;
            for (const auto& sampled : sampled_edges) {
                if (sampled.parameters.size()
                    > std::numeric_limits<size_t>::max() - sample_count_after) {
                    return result;
                }
                sample_count_after += sampled.parameters.size();
            }
            if (sample_count_after > maximum_brep_canonical_samples) {
                record_resource_failure(
                    __func__, "canonical shared-edge sample quota exceeded");
                return result;
            }
            if (sample_count_after == sample_count_before) {
                incident_charts_stable = true;
                break;
            }
        }
        if (!incident_charts_stable) {
            record_resource_failure(
                __func__,
                "shared-edge incident-chart refinement pass quota exceeded");
            return result;
        }
        size_t edge_point_scalar_count = 0;
        if (!checked_size_multiply(
                budget.canonical_samples, 3, edge_point_scalar_count)) {
            record_resource_failure(
                __func__, "shared-edge point size arithmetic overflow");
            return result;
        }
        if (!budget.reserve_serialized_append(
                result.edge_points,
                edge_point_scalar_count,
                "serialized shared-edge point byte quota exceeded")) {
            return result;
        }
        failure_stage = "serialize_edge_samples";
        for (int index = 1; index <= source_edges.Extent(); ++index) {
            if (rust_progress_cancelled(progress)) return result;
            auto& sampled = sampled_edges[static_cast<size_t>(index - 1)];
            const int copied_ordinal = copied_edge_ordinals_by_source_index[
                static_cast<size_t>(index - 1)];
            if (!refresh_sampled_brep_edge_points(
                    TopoDS::Edge(edges(copied_ordinal)),
                    vertices,
                    canonical_vertex_points,
                    progress,
                    sampled)) {
                return result;
            }
            for (const gp_Pnt& point : sampled.points) {
                if (rust_progress_cancelled(progress)) return result;
                if (!append_brep_point(result.edge_points, point)) return result;
            }
            if (!append_brep_offset(
                    result.edge_point_offsets,
                    result.edge_points.size() / 3,
                    budget,
                    "serialized shared-edge offset byte quota exceeded")) {
                return result;
            }
        }
        failure_stage = "select_faces";
        std::vector<uint32_t> selected_faces;
        if (face_indices.size() == 0) {
            selected_faces.reserve(static_cast<size_t>(faces.Extent()));
            for (int index = 0; index < faces.Extent(); ++index) {
                if (rust_progress_cancelled(progress)) return result;
                selected_faces.push_back(static_cast<uint32_t>(index));
            }
        } else {
            selected_faces.assign(face_indices.begin(), face_indices.end());
        }
        const size_t selected_face_count = selected_faces.size();
        size_t face_bounds_scalar_count = 0;
        if (!checked_size_multiply(
                selected_face_count, 4, face_bounds_scalar_count)) {
            record_resource_failure(
                __func__, "face-bound size arithmetic overflow");
            return result;
        }
        if (!budget.reserve_serialized_append(
                result.face_indices,
                selected_face_count,
                "serialized face-index byte quota exceeded")
            || !budget.reserve_serialized_append(
                result.face_tshape_ids,
                selected_face_count,
                "serialized face-identity byte quota exceeded")
            || !budget.reserve_serialized_append(
                result.face_reversed,
                selected_face_count,
                "serialized face-orientation byte quota exceeded")
            || !budget.reserve_serialized_append(
                result.face_u_degrees,
                selected_face_count,
                "serialized surface-degree byte quota exceeded")
            || !budget.reserve_serialized_append(
                result.face_v_degrees,
                selected_face_count,
                "serialized surface-degree byte quota exceeded")
            || !budget.reserve_serialized_append(
                result.face_u_pole_counts,
                selected_face_count,
                "serialized surface-dimension byte quota exceeded")
            || !budget.reserve_serialized_append(
                result.face_v_pole_counts,
                selected_face_count,
                "serialized surface-dimension byte quota exceeded")
            || !budget.reserve_serialized_append(
                result.face_uv_bounds,
                face_bounds_scalar_count,
                "serialized surface-bound byte quota exceeded")
            || !budget.reserve_serialized_append(
                result.face_approximation_errors,
                selected_face_count,
                "serialized approximation-error byte quota exceeded")) {
            return result;
        }
        std::unordered_set<uint32_t> unique_face_indices;
        unique_face_indices.reserve(selected_faces.size());
        failure_stage = "extract_face_surfaces_and_trims";
        for (uint32_t face_index : selected_faces) {
            if (rust_progress_cancelled(progress)
                || face_index >= static_cast<uint32_t>(faces.Extent())
                || !unique_face_indices.insert(face_index).second) {
                return result;
            }
            const TopoDS_Face face =
                extraction_faces_by_source_index[face_index];
            if (face.Orientation() != TopAbs_FORWARD
                && face.Orientation() != TopAbs_REVERSED) {
                return result;
            }
            result.face_indices.push_back(face_index);
            result.face_tshape_ids.push_back(
                reinterpret_cast<uint64_t>(
                    source_faces(static_cast<int>(face_index + 1)).TShape().get()));
            result.face_reversed.push_back(
                face.Orientation() == TopAbs_REVERSED ? 1 : 0);
            BrepSurfaceChart chart;
            double maximum_boundary_error = 0.0;
            failure_stage = "extract_face_surface";
            if (!append_bounded_bspline_surface(
                    face, result, chart, budget, progress)) {
                return result;
            }
            failure_stage = "extract_face_trim_loops";
            if (!append_face_trim_loops(
                    face,
                    chart,
                    edges,
                    copied_edge_to_source_index,
                    sampled_edges,
                    absolute_linear,
                    maximum_boundary_error,
                    result,
                    budget,
                    progress)) {
                return result;
            }
            if (result.face_approximation_errors.empty()) return result;
            result.face_approximation_errors.back() = maximum_boundary_error;
        }
        failure_stage = "complete";
        result.success = true;
        rust_progress_set(progress, 0.25);
    } catch (const Standard_OutOfMemory& failure) {
        record_standard_failure(__func__, "resource_limit", 5, failure);
        return result;
    } catch (const std::bad_alloc&) {
        record_resource_failure(__func__, "native extraction allocation failed");
        return result;
    } catch (const std::length_error&) {
        record_resource_failure(
            __func__, "native extraction container limit exceeded");
        return result;
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "extract", 7, failure);
        return result;
    } catch (...) {
        record_input_failure(__func__, "B-rep source extraction failed");
        return result;
    }
    return result;
}

constexpr size_t maximum_raw_occt_vertices = 4'194'304;
constexpr size_t maximum_raw_occt_triangles = 8'388'608;
constexpr size_t maximum_raw_occt_edge_points = 4'194'304;

struct RawOcctMeshBudget {
    size_t vertices = 0;
    size_t triangles = 0;
    size_t edge_points = 0;
    // The face-vertex, face-index, and edge-point offset arrays start at zero.
    size_t serialized_bytes = 3 * sizeof(uint32_t);

    bool claim(
        size_t& counter,
        size_t amount,
        size_t maximum,
        const char* message)
    {
        size_t next = 0;
        if (!checked_size_add(counter, amount, next) || next > maximum) {
            record_resource_failure("mesh_shape_raw_occt", message);
            return false;
        }
        counter = next;
        return true;
    }

    template <typename Value>
    bool reserve_append(
        rust::Vec<Value>& output,
        size_t amount,
        const char* message)
    {
        size_t bytes = 0;
        size_t next_size = 0;
        size_t next_bytes = 0;
        if (!checked_size_multiply(amount, sizeof(Value), bytes)
            || !checked_size_add(output.size(), amount, next_size)
            || !checked_size_add(serialized_bytes, bytes, next_bytes)) {
            record_resource_failure(
                "mesh_shape_raw_occt",
                "raw OCCT mesh output size arithmetic overflow");
            return false;
        }
        if (next_bytes > maximum_brep_serialized_bytes) {
            record_resource_failure("mesh_shape_raw_occt", message);
            return false;
        }
        serialized_bytes = next_bytes;
        if (output.capacity() < next_size) output.reserve(next_size);
        return true;
    }

    bool reserve_face(
        MeshData& result,
        size_t node_count,
        size_t triangle_count)
    {
        size_t coordinate_count = 0;
        size_t index_count = 0;
        if (!checked_size_multiply(node_count, 3, coordinate_count)
            || !checked_size_multiply(triangle_count, 3, index_count)) {
            record_resource_failure(
                "mesh_shape_raw_occt",
                "raw OCCT face output size arithmetic overflow");
            return false;
        }
        return claim(
                   vertices,
                   node_count,
                   maximum_raw_occt_vertices,
                   "raw OCCT vertex quota exceeded")
            && claim(
                triangles,
                triangle_count,
                maximum_raw_occt_triangles,
                "raw OCCT triangle quota exceeded")
            && reserve_append(
                result.vertices,
                coordinate_count,
                "raw OCCT serialized vertex byte quota exceeded")
            && reserve_append(
                result.normals,
                coordinate_count,
                "raw OCCT serialized normal byte quota exceeded")
            && reserve_append(
                result.indices,
                index_count,
                "raw OCCT serialized index byte quota exceeded")
            && reserve_append(
                result.face_tshape_ids,
                triangle_count,
                "raw OCCT serialized face-identity byte quota exceeded")
            && reserve_append(
                result.chunk_face_tshape_ids,
                1,
                "raw OCCT serialized face-chunk byte quota exceeded")
            && reserve_append(
                result.chunk_face_indices,
                1,
                "raw OCCT serialized face-index byte quota exceeded")
            && reserve_append(
                result.face_vertex_offsets,
                1,
                "raw OCCT serialized face-offset byte quota exceeded")
            && reserve_append(
                result.face_index_offsets,
                1,
                "raw OCCT serialized face-offset byte quota exceeded");
    }

    bool reserve_edge(MeshData& result, size_t point_count) {
        size_t coordinate_count = 0;
        if (!checked_size_multiply(point_count, 3, coordinate_count)) {
            record_resource_failure(
                "mesh_shape_raw_occt",
                "raw OCCT edge output size arithmetic overflow");
            return false;
        }
        return claim(
                   edge_points,
                   point_count,
                   maximum_raw_occt_edge_points,
                   "raw OCCT edge-point quota exceeded")
            && reserve_append(
                result.edge_points,
                coordinate_count,
                "raw OCCT serialized edge-point byte quota exceeded")
            && reserve_append(
                result.chunk_edge_indices,
                1,
                "raw OCCT serialized edge-index byte quota exceeded")
            && reserve_append(
                result.edge_point_offsets,
                1,
                "raw OCCT serialized edge-offset byte quota exceeded");
    }
};

static bool raw_occt_copy_ordinals(
    BRepBuilderAPI_Copy& copier,
    const BrepShapeMap& source_shapes,
    const BrepShapeMap& copied_shapes,
    const CancellationToken& progress,
    std::vector<int>& copied_ordinals)
{
    copied_ordinals.assign(static_cast<size_t>(source_shapes.Extent()), 0);
    std::vector<bool> copied_seen(
        static_cast<size_t>(copied_shapes.Extent()), false);
    for (int index = 1; index <= source_shapes.Extent(); ++index) {
        if (rust_progress_cancelled(progress)) return false;
        const TopoDS_Shape copied = copier.ModifiedShape(source_shapes(index));
        if (copied.IsNull()) return false;
        const int copied_ordinal = copied_shapes.FindIndex(copied);
        if (copied_ordinal < 1
            || copied_seen[static_cast<size_t>(copied_ordinal - 1)]) {
            return false;
        }
        copied_ordinals[static_cast<size_t>(index - 1)] = copied_ordinal;
        copied_seen[static_cast<size_t>(copied_ordinal - 1)] = true;
    }
    return std::find(copied_seen.begin(), copied_seen.end(), false)
        == copied_seen.end();
}

static bool append_raw_occt_face_mesh(
    const TopoDS_Face& face,
    uint32_t source_face_index,
    uint64_t source_face_tshape_id,
    const CancellationToken& progress,
    RawOcctMeshBudget& budget,
    MeshData& result)
{
    if (rust_progress_cancelled(progress)) return false;
    TopLoc_Location location;
    Handle(Poly_Triangulation) triangulation =
        BRep_Tool::Triangulation(face, location);
    if (triangulation.IsNull()) return false;
    // Ask OCCT to evaluate its native surface normals on its own mesh nodes.
    // No Plex normal averaging, filtering, or repair is applied afterward.
    BRepLib_ToolTriangulatedShape::ComputeNormals(face, triangulation);
    if (rust_progress_cancelled(progress) || !triangulation->HasNormals()) {
        return false;
    }

    const int node_count = triangulation->NbNodes();
    const int triangle_count = triangulation->NbTriangles();
    if (node_count < 3 || triangle_count < 1
        || !budget.reserve_face(
            result,
            static_cast<size_t>(node_count),
            static_cast<size_t>(triangle_count))) {
        return false;
    }
    const uint32_t vertex_offset =
        static_cast<uint32_t>(result.vertices.size() / 3);
    result.chunk_face_tshape_ids.push_back(source_face_tshape_id);
    result.chunk_face_indices.push_back(source_face_index);
    const gp_Trsf location_transform = location.Transformation();
    const bool reversed = face.Orientation() == TopAbs_REVERSED;
    if (!reversed && face.Orientation() != TopAbs_FORWARD) return false;
    for (int index = 1; index <= node_count; ++index) {
        if (rust_progress_cancelled(progress)) return false;
        gp_Pnt point = triangulation->Node(index);
        point.Transform(location_transform);
        gp_Dir normal = triangulation->Normal(index);
        normal.Transform(location_transform);
        if (reversed) normal.Reverse();
        result.vertices.push_back(point.X());
        result.vertices.push_back(point.Y());
        result.vertices.push_back(point.Z());
        result.normals.push_back(normal.X());
        result.normals.push_back(normal.Y());
        result.normals.push_back(normal.Z());
    }
    for (int index = 1; index <= triangle_count; ++index) {
        if (rust_progress_cancelled(progress)) return false;
        int first = 0;
        int second = 0;
        int third = 0;
        triangulation->Triangle(index).Get(first, second, third);
        if (first < 1 || second < 1 || third < 1
            || first > node_count || second > node_count
            || third > node_count) {
            return false;
        }
        result.indices.push_back(vertex_offset + static_cast<uint32_t>(first - 1));
        result.indices.push_back(
            vertex_offset
            + static_cast<uint32_t>((reversed ? third : second) - 1));
        result.indices.push_back(
            vertex_offset
            + static_cast<uint32_t>((reversed ? second : third) - 1));
        result.face_tshape_ids.push_back(source_face_tshape_id);
    }
    result.face_vertex_offsets.push_back(
        static_cast<uint32_t>(result.vertices.size() / 3));
    result.face_index_offsets.push_back(
        static_cast<uint32_t>(result.indices.size()));
    return true;
}

static bool append_raw_occt_edge_polygon(
    const TopoDS_Edge& edge,
    uint32_t source_edge_index,
    const CancellationToken& progress,
    RawOcctMeshBudget& budget,
    MeshData& result)
{
    if (rust_progress_cancelled(progress)) return false;
    Handle(Poly_PolygonOnTriangulation) polygon;
    Handle(Poly_Triangulation) triangulation;
    TopLoc_Location location;
    BRep_Tool::PolygonOnTriangulation(
        edge, polygon, triangulation, location);
    // OCCT does not create a polygon for a zero-length pole edge. Omitting it
    // preserves the source ordinal of every emitted edge without inventing a
    // presentation segment.
    if (polygon.IsNull() || triangulation.IsNull()
        || polygon->NbNodes() < 2) {
        return true;
    }
    const auto& nodes = polygon->Nodes();
    const size_t point_count = static_cast<size_t>(nodes.Length());
    if (!budget.reserve_edge(result, point_count)) return false;
    const bool reversed = edge.Orientation() == TopAbs_REVERSED;
    if (!reversed && edge.Orientation() != TopAbs_FORWARD) return false;
    result.chunk_edge_indices.push_back(source_edge_index);
    const gp_Trsf location_transform = location.Transformation();
    for (int ordinal = nodes.Lower(); ordinal <= nodes.Upper(); ++ordinal) {
        if (rust_progress_cancelled(progress)) return false;
        const int node_index = reversed
            ? nodes(nodes.Upper() - (ordinal - nodes.Lower()))
            : nodes(ordinal);
        if (node_index < 1 || node_index > triangulation->NbNodes()) {
            return false;
        }
        gp_Pnt point = triangulation->Node(node_index);
        point.Transform(location_transform);
        result.edge_points.push_back(point.X());
        result.edge_points.push_back(point.Y());
        result.edge_points.push_back(point.Z());
    }
    result.edge_point_offsets.push_back(
        static_cast<uint32_t>(result.edge_points.size() / 3));
    return true;
}

MeshData mesh_shape_raw_occt(
    const TopoDS_Shape& shape,
    double linear,
    double angular,
    bool relative,
    bool parallel,
    bool include_edges,
    const CancellationToken& progress)
{
    MeshData result;
    result.success = false;
    result.face_vertex_offsets.push_back(0);
    result.face_index_offsets.push_back(0);
    result.edge_point_offsets.push_back(0);
    const char* failure_stage = "validate_input";
    ScopedFailureDiagnostic failure_diagnostic(
        __func__, failure_stage, result.success, progress);
    try {
        if (shape.IsNull()
            || !std::isfinite(linear) || linear <= 0.0
            || !std::isfinite(angular) || angular <= 0.0) {
            record_input_failure(
                __func__,
                "shape and tessellation tolerances must be valid");
            return result;
        }
        if (rust_progress_cancelled(progress)) return result;

        failure_stage = "map_source_topology";
        BrepShapeMap source_faces;
        BrepShapeMap source_edges;
        BrepShapeMap source_vertices;
        if (!map_unique_brep_subshapes_bounded(
                shape,
                TopAbs_FACE,
                maximum_brep_faces,
                "shape face quota exceeded",
                progress,
                source_faces,
                __func__)
            || !map_unique_brep_subshapes_bounded(
                shape,
                TopAbs_EDGE,
                maximum_brep_edges,
                "shape edge quota exceeded",
                progress,
                source_edges,
                __func__)
            || !map_unique_brep_subshapes_bounded(
                shape,
                TopAbs_VERTEX,
                maximum_brep_vertices,
                "shape vertex quota exceeded",
                progress,
                source_vertices,
                __func__)) {
            return result;
        }
        failure_stage = "preflight_copy_surface_storage";
        if (!preflight_brep_copy_surface_storage(
                source_faces,
                maximum_brep_control_points,
                maximum_brep_knots,
                progress,
                __func__)) {
            return result;
        }

        failure_stage = "copy_detached_shape";
        if (rust_progress_cancelled(progress)) return result;
        BRepBuilderAPI_Copy copier(shape, true, false);
        if (rust_progress_cancelled(progress)
            || !copier.IsDone() || copier.Shape().IsNull()) {
            return result;
        }
        const TopoDS_Shape detached_shape = copier.Shape();

        failure_stage = "map_detached_topology";
        BrepShapeMap detached_faces;
        BrepShapeMap detached_edges;
        BrepShapeMap detached_vertices;
        if (!map_unique_brep_subshapes_bounded(
                detached_shape,
                TopAbs_FACE,
                maximum_brep_faces,
                "detached shape face quota exceeded",
                progress,
                detached_faces,
                __func__)
            || !map_unique_brep_subshapes_bounded(
                detached_shape,
                TopAbs_EDGE,
                maximum_brep_edges,
                "detached shape edge quota exceeded",
                progress,
                detached_edges,
                __func__)
            || !map_unique_brep_subshapes_bounded(
                detached_shape,
                TopAbs_VERTEX,
                maximum_brep_vertices,
                "detached shape vertex quota exceeded",
                progress,
                detached_vertices,
                __func__)) {
            return result;
        }
        if (detached_faces.Extent() != source_faces.Extent()
            || detached_edges.Extent() != source_edges.Extent()
            || detached_vertices.Extent() != source_vertices.Extent()) {
            return result;
        }
        std::vector<int> copied_face_ordinals;
        std::vector<int> copied_edge_ordinals;
        if (!raw_occt_copy_ordinals(
                copier,
                source_faces,
                detached_faces,
                progress,
                copied_face_ordinals)
            || !raw_occt_copy_ordinals(
                copier,
                source_edges,
                detached_edges,
                progress,
                copied_edge_ordinals)) {
            return result;
        }

        failure_stage = "mesh_detached_shape";
        IMeshTools_Parameters parameters;
        parameters.Deflection = linear;
        parameters.Angle = angular;
        parameters.Relative = relative;
        parameters.InParallel = parallel;
        Handle(RustProgressIndicator) indicator =
            new RustProgressIndicator(progress);
        BRepMesh_IncrementalMesh mesher(
            detached_shape, parameters, indicator->Start());
        if (rust_progress_cancelled(progress) || !mesher.IsDone()) {
            return result;
        }

        RawOcctMeshBudget budget;
        failure_stage = "copy_face_triangulations";
        for (int source_index = 1;
             source_index <= source_faces.Extent(); ++source_index) {
            if (rust_progress_cancelled(progress)) return result;
            const TopoDS_Face face = TopoDS::Face(
                detached_faces(copied_face_ordinals[
                    static_cast<size_t>(source_index - 1)]));
            if (!append_raw_occt_face_mesh(
                    face,
                    static_cast<uint32_t>(source_index - 1),
                    reinterpret_cast<uint64_t>(
                        source_faces(source_index).TShape().get()),
                    progress,
                    budget,
                    result)) {
                return result;
            }
        }
        if (include_edges) {
            failure_stage = "copy_edge_polygons";
            for (int source_index = 1;
                 source_index <= source_edges.Extent(); ++source_index) {
                if (rust_progress_cancelled(progress)) return result;
                const TopoDS_Edge edge = TopoDS::Edge(
                    detached_edges(copied_edge_ordinals[
                        static_cast<size_t>(source_index - 1)]));
                if (!append_raw_occt_edge_polygon(
                        edge,
                        static_cast<uint32_t>(source_index - 1),
                        progress,
                        budget,
                        result)) {
                    return result;
                }
            }
        }
        if (rust_progress_cancelled(progress)) return result;
        failure_stage = "complete";
        result.success = true;
    } catch (const Standard_OutOfMemory& failure) {
        record_standard_failure(__func__, "resource_limit", 5, failure);
    } catch (const std::bad_alloc&) {
        record_resource_failure(
            __func__, "raw OCCT mesh allocation failed");
    } catch (const std::length_error&) {
        record_resource_failure(
            __func__, "raw OCCT mesh container limit exceeded");
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, failure_stage, 7, failure);
    }
    return result;
}

rust::Vec<double> test_brep_chart_mapping_errors()
{
    BrepSurfaceChart chart;
    chart.original = new Geom_CylindricalSurface(
        gp_Ax3(gp_Pnt(0.0, 0.0, 0.0), gp_Dir(0.0, 0.0, 1.0)), 5.0);
    chart.original_u_max = 2.0 * std::acos(-1.0);
    chart.original_v_max = 4.0;
    const Handle(Geom_RectangularTrimmedSurface) bounded =
        new Geom_RectangularTrimmedSurface(
            chart.original, 0.0, chart.original_u_max, 0.0, 4.0);
    chart.spline = GeomConvert::SurfaceToBSplineSurface(bounded);
    chart.spline->Bounds(
        chart.spline_u_min, chart.spline_u_max,
        chart.spline_v_min, chart.spline_v_max);
    const gp_Pnt2d original_uv(0.37, 0.0);
    const gp_Pnt source = chart.original->Value(original_uv.X(), original_uv.Y());
    const gp_Pnt normalized = chart.spline->Value(
        chart.spline_u_min + original_uv.X() / chart.original_u_max
            * (chart.spline_u_max - chart.spline_u_min), chart.spline_v_min);
    const double conversion_error = source.Distance(normalized);
    const double allowed = conversion_error * 1.001;
    const gp_Pnt canonical = source.Translated(gp_Vec(normalized, source) * 0.002);
    BrepChartMapState state;
    gp_Pnt2d mapped_uv;
    if (!map_to_bspline_chart(chart, original_uv, canonical, state, allowed, mapped_uv)) {
        return {};
    }
    const gp_Pnt mapped = chart.spline->Value(mapped_uv.X(), mapped_uv.Y());
    return {allowed, source.Distance(canonical), conversion_error,
        normalized.Distance(canonical), mapped.Distance(source), mapped.Distance(canonical)};
}

bool test_seed_occt_triangulation_cache(
    const TopoDS_Shape& shape,
    double linear,
    double angular,
    bool relative)
{
    try {
        if (shape.IsNull()
            || !std::isfinite(linear) || linear <= 0.0
            || !std::isfinite(angular) || angular <= 0.0) {
            return false;
        }
        IMeshTools_Parameters parameters;
        parameters.Deflection = linear;
        parameters.Angle = angular;
        parameters.Relative = relative;
        parameters.InParallel = false;
        BRepMesh_IncrementalMesh mesher(shape, parameters);
        return mesher.IsDone();
    } catch (const Standard_Failure&) {
        return false;
    }
}

TriangulationCacheData test_occt_triangulation_cache(
    const TopoDS_Shape& shape)
{
    TriangulationCacheData result;
    result.success = false;
    try {
        BrepShapeMap faces;
        TopExp::MapShapes(shape, TopAbs_FACE, faces);
        if (faces.Extent() < 0
            || static_cast<size_t>(faces.Extent()) > maximum_brep_faces) {
            return result;
        }
        result.face_count = static_cast<uint32_t>(faces.Extent());
        for (int index = 1; index <= faces.Extent(); ++index) {
            TopLoc_Location location;
            const Handle(Poly_Triangulation) triangulation =
                BRep_Tool::Triangulation(
                    TopoDS::Face(faces(index)), location);
            if (triangulation.IsNull()) continue;
            const int node_count = triangulation->NbNodes();
            const int triangle_count = triangulation->NbTriangles();
            if (node_count < 0 || triangle_count < 0
                || result.node_count
                    > std::numeric_limits<uint64_t>::max()
                        - static_cast<uint64_t>(node_count)
                || result.triangle_count
                    > std::numeric_limits<uint64_t>::max()
                        - static_cast<uint64_t>(triangle_count)) {
                return result;
            }
            ++result.triangulated_face_count;
            result.node_count += static_cast<uint64_t>(node_count);
            result.triangle_count += static_cast<uint64_t>(triangle_count);
        }
        result.success = true;
    } catch (const Standard_Failure&) {
        return result;
    }
    return result;
}

bool test_brep_extraction_preflight_limits(
    const TopoDS_Shape& shape,
    uint32_t maximum_faces,
    uint32_t maximum_edges,
    uint32_t maximum_vertices,
    uint32_t maximum_control_points,
    uint32_t maximum_knots,
    const CancellationToken& progress)
{
    bool success = false;
    const char* failure_stage = "map_source_topology";
    ScopedFailureDiagnostic failure_diagnostic(
        "extract_brep_mesh_source", failure_stage, success, progress);
    try {
        if (shape.IsNull()) {
            record_input_failure(
                "extract_brep_mesh_source", "shape must not be null");
            return false;
        }
        BrepShapeMap faces;
        BrepShapeMap edges;
        BrepShapeMap vertices;
        if (!map_unique_brep_subshapes_bounded(
                shape,
                TopAbs_FACE,
                maximum_faces,
                "shape face quota exceeded",
                progress,
                faces)
            || !map_unique_brep_subshapes_bounded(
                shape,
                TopAbs_EDGE,
                maximum_edges,
                "shape edge quota exceeded",
                progress,
                edges)
            || !map_unique_brep_subshapes_bounded(
                shape,
                TopAbs_VERTEX,
                maximum_vertices,
                "shape vertex quota exceeded",
                progress,
                vertices)) {
            return false;
        }
        failure_stage = "preflight_copy_surface_storage";
        if (!preflight_brep_copy_surface_storage(
                faces,
                maximum_control_points,
                maximum_knots,
                progress)) {
            return false;
        }
        success = true;
        return true;
    } catch (const Standard_OutOfMemory& failure) {
        record_standard_failure(
            "extract_brep_mesh_source", "resource_limit", 5, failure);
    } catch (const std::bad_alloc&) {
        record_resource_failure(
            "extract_brep_mesh_source",
            "native extraction preflight allocation failed");
    } catch (const std::length_error&) {
        record_resource_failure(
            "extract_brep_mesh_source",
            "native extraction preflight container limit exceeded");
    } catch (const Standard_Failure& failure) {
        record_standard_failure(
            "extract_brep_mesh_source", "preflight", 7, failure);
    }
    return false;
}


// ==================== Topology enumeration ====================

std::unique_ptr<std::vector<TopoDS_Edge>> shape_edges(const TopoDS_Shape& shape) {
    // TopExp_Explorer visits shared edges once per adjacent face.
    // NCollection_IndexedMap collapses those into unique edges.
    NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> edgeMap;
    TopExp::MapShapes(shape, TopAbs_EDGE, edgeMap);
    auto out = std::make_unique<std::vector<TopoDS_Edge>>();
    out->reserve(edgeMap.Extent());
    for (int i = 1; i <= edgeMap.Extent(); i++) {
        out->push_back(TopoDS::Edge(edgeMap(i)));
    }
    return out;
}

std::unique_ptr<std::vector<TopoDS_Face>> shape_faces(const TopoDS_Shape& shape) {
	NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> faceMap;
	TopExp::MapShapes(shape, TopAbs_FACE, faceMap);
	auto out = std::make_unique<std::vector<TopoDS_Face>>();
	out->reserve(faceMap.Extent());
	for (int i = 1; i <= faceMap.Extent(); i++) {
		out->push_back(TopoDS::Face(faceMap(i)));
	}
	return out;
}

uint32_t shape_vertex_count(const TopoDS_Shape& shape) {
    try {
        NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> vertices;
        TopExp::MapShapes(shape, TopAbs_VERTEX, vertices);
        return static_cast<uint32_t>(vertices.Extent());
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return UINT32_MAX;
    }
}

TopologyData shape_topology(const TopoDS_Shape& shape, uint32_t query_flags) {
    TopologyData result;
    result.success = false;
    try {
        constexpr uint32_t QUERY_FRAMES = 1U << 0;
        constexpr uint32_t QUERY_MEASUREMENTS = 1U << 1;
        constexpr uint32_t QUERY_GEOMETRY = 1U << 2;
        constexpr uint32_t QUERY_EDGE_DIRECTIONS = 1U << 3;
        constexpr uint32_t FACT_FRAME = 1U << 0;
        constexpr uint32_t FACT_MEASUREMENT = 1U << 1;
        constexpr uint32_t FACT_CLOSED = 1U << 2;
        constexpr uint32_t FACT_DEGENERATE = 1U << 3;
        constexpr uint32_t FACT_SEAM = 1U << 4;
        constexpr uint32_t FACT_MANIFOLD = 1U << 5;
        constexpr uint32_t FACT_DIRECTION = 1U << 6;
        const bool query_frames = (query_flags & QUERY_FRAMES) != 0;
        const bool query_measurements = (query_flags & QUERY_MEASUREMENTS) != 0;
        const bool query_geometry = (query_flags & QUERY_GEOMETRY) != 0;
        const bool query_edge_directions = (query_flags & QUERY_EDGE_DIRECTIONS) != 0;

        NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> faces;
        NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> edges;
        NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> vertices;
        TopExp::MapShapes(shape, TopAbs_FACE, faces);
        TopExp::MapShapes(shape, TopAbs_EDGE, edges);
        TopExp::MapShapes(shape, TopAbs_VERTEX, vertices);

        result.face_edge_offsets.reserve(static_cast<size_t>(faces.Extent()) + 1);
        result.edge_face_offsets.reserve(static_cast<size_t>(edges.Extent()) + 1);
        result.edge_vertex_offsets.reserve(static_cast<size_t>(edges.Extent()) + 1);
        result.face_edge_offsets.push_back(0);
        result.edge_vertex_offsets.push_back(0);
        std::vector<std::vector<uint32_t>> edge_faces(static_cast<size_t>(edges.Extent()));

        for (int face_index = 1; face_index <= faces.Extent(); ++face_index) {
            const TopoDS_Shape& face = faces(face_index);
            result.face_tshape_ids.push_back(reinterpret_cast<uint64_t>(face.TShape().get()));
            result.face_location_hashes.push_back(static_cast<uint64_t>(face.Location().HashCode()));
            result.face_orientations.push_back(static_cast<uint32_t>(face.Orientation()));
            NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> face_edges;
            TopExp::MapShapes(face, TopAbs_EDGE, face_edges);
            for (int local_index = 1; local_index <= face_edges.Extent(); ++local_index) {
                const int edge_index = edges.FindIndex(face_edges(local_index));
                if (edge_index < 1) return result;
                const auto zero_based_edge = static_cast<uint32_t>(edge_index - 1);
                result.face_edge_indices.push_back(zero_based_edge);
                edge_faces[static_cast<size_t>(zero_based_edge)].push_back(
                    static_cast<uint32_t>(face_index - 1));
            }
            result.face_edge_offsets.push_back(
                static_cast<uint32_t>(result.face_edge_indices.size()));

            if (query_frames || query_measurements || query_geometry) {
                uint32_t fact_flags = face.Closed() ? FACT_CLOSED : 0;
                uint32_t geometry_kind = 0;
                gp_Pnt point(0.0, 0.0, 0.0);
                gp_Dir normal(0.0, 0.0, 1.0);
                double area = 0.0;
                const TopoDS_Face typed_face = TopoDS::Face(face);
                BRepAdaptor_Surface surface(typed_face, true);
                const GeomAbs_SurfaceType surface_type = surface.GetType();
                if (query_geometry) {
                    geometry_kind = static_cast<uint32_t>(surface_type) + 1;
                }
                GProp_GProps surface_properties;
                const bool has_surface_properties = query_measurements
                    || (query_frames && query_geometry && surface_type == GeomAbs_Plane);
                if (has_surface_properties) {
                    BRepGProp::SurfaceProperties(typed_face, surface_properties, 1.0e-9);
                    area = surface_properties.Mass();
                }
                if (query_frames) {
                    const double u_first = surface.FirstUParameter();
                    const double u_last = surface.LastUParameter();
                    const double v_first = surface.FirstVParameter();
                    const double v_last = surface.LastVParameter();
                    if (std::isfinite(u_first) && std::isfinite(u_last)
                        && std::isfinite(v_first) && std::isfinite(v_last)) {
                        BRepLProp_SLProps properties(
                            surface,
                            (u_first + u_last) * 0.5,
                            (v_first + v_last) * 0.5,
                            1,
                            Precision::Confusion());
                        if (properties.IsNormalDefined()) {
                            point = properties.Value();
                            normal = properties.Normal();
                            if (typed_face.Orientation() == TopAbs_REVERSED) normal.Reverse();
                            if (surface_type == GeomAbs_Plane && has_surface_properties) {
                                const gp_Pnt center = surface_properties.CentreOfMass();
                                if (std::isfinite(center.X()) && std::isfinite(center.Y())
                                    && std::isfinite(center.Z())) {
                                    point = center;
                                }
                            }
                            fact_flags |= FACT_FRAME;
                        }
                    }
                }
                if (query_measurements) {
                    if (std::isfinite(area)) fact_flags |= FACT_MEASUREMENT;
                }
                result.face_geometry_kinds.push_back(geometry_kind);
                result.face_fact_flags.push_back(fact_flags);
                result.face_points.push_back(point.X());
                result.face_points.push_back(point.Y());
                result.face_points.push_back(point.Z());
                result.face_normals.push_back(normal.X());
                result.face_normals.push_back(normal.Y());
                result.face_normals.push_back(normal.Z());
                result.face_areas.push_back(area);
            }
        }

        result.edge_face_offsets.push_back(0);
        for (int edge_index = 1; edge_index <= edges.Extent(); ++edge_index) {
            const TopoDS_Shape& edge = edges(edge_index);
            result.edge_tshape_ids.push_back(reinterpret_cast<uint64_t>(edge.TShape().get()));
            result.edge_location_hashes.push_back(static_cast<uint64_t>(edge.Location().HashCode()));
            result.edge_orientations.push_back(static_cast<uint32_t>(edge.Orientation()));
            const auto& adjacent = edge_faces[static_cast<size_t>(edge_index - 1)];
            for (uint32_t face_index : adjacent) result.edge_face_indices.push_back(face_index);
            result.edge_face_offsets.push_back(
                static_cast<uint32_t>(result.edge_face_indices.size()));

            const TopoDS_Edge typed_edge = TopoDS::Edge(edge);
            NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> edge_vertices;
            TopExp::MapShapes(edge, TopAbs_VERTEX, edge_vertices);
            for (int local_index = 1; local_index <= edge_vertices.Extent(); ++local_index) {
                const int vertex_index = vertices.FindIndex(edge_vertices(local_index));
                if (vertex_index < 1) return result;
                result.edge_vertex_indices.push_back(static_cast<uint32_t>(vertex_index - 1));
            }
            result.edge_vertex_offsets.push_back(
                static_cast<uint32_t>(result.edge_vertex_indices.size()));

            if (query_frames || query_measurements || query_geometry) {
                uint32_t fact_flags = 0;
                if (BRep_Tool::IsClosed(edge)) fact_flags |= FACT_CLOSED;
                if (BRep_Tool::Degenerated(typed_edge)) fact_flags |= FACT_DEGENERATE;
                if (adjacent.size() == 2) fact_flags |= FACT_MANIFOLD;
                for (uint32_t face_index : adjacent) {
                    if (BRep_Tool::IsClosed(typed_edge, TopoDS::Face(faces(static_cast<int>(face_index + 1))))) {
                        fact_flags |= FACT_SEAM;
                        break;
                    }
                }
                uint32_t geometry_kind = 0;
                gp_Pnt point(0.0, 0.0, 0.0);
                gp_Vec tangent(0.0, 0.0, 0.0);
                gp_Vec outward(0.0, 0.0, 0.0);
                double length = 0.0;
                // Collapsed sewing edges can have only a pcurve. Querying their 3D
                // adaptor or length can dereference a null BSpline inside OCCT.
                if ((fact_flags & FACT_DEGENERATE) == 0) {
                    BRepAdaptor_Curve curve(typed_edge);
                    if (query_geometry) {
                        geometry_kind = static_cast<uint32_t>(curve.GetType()) + 1;
                    }
                    if (query_frames) {
                        const double first = curve.FirstParameter();
                        const double last = curve.LastParameter();
                        if (std::isfinite(first) && std::isfinite(last)) {
                            curve.D1((first + last) * 0.5, point, tangent);
                            if (tangent.SquareMagnitude() > Precision::SquareConfusion()) {
                                tangent.Normalize();
                                fact_flags |= FACT_FRAME;
                            }
                        }
                        if (query_edge_directions && (fact_flags & FACT_FRAME) != 0) {
                            for (uint32_t face_index : adjacent) {
                                try {
                                    const TopoDS_Face face = TopoDS::Face(
                                        faces(static_cast<int>(face_index + 1)));
                                    const TopoDS_Vertex probe =
                                        BRepBuilderAPI_MakeVertex(point);
                                    BRepExtrema_ExtPF extrema(probe, face);
                                    if (!extrema.IsDone() || extrema.NbExt() < 1) continue;
                                    int nearest = 1;
                                    double nearest_distance = extrema.SquareDistance(1);
                                    for (int candidate = 2; candidate <= extrema.NbExt(); ++candidate) {
                                        const double distance = extrema.SquareDistance(candidate);
                                        if (distance < nearest_distance) {
                                            nearest = candidate;
                                            nearest_distance = distance;
                                        }
                                    }
                                    double u = 0.0;
                                    double v = 0.0;
                                    extrema.Parameter(nearest, u, v);
                                    BRepAdaptor_Surface surface(face);
                                    BRepLProp_SLProps properties(
                                        surface, u, v, 1, Precision::Confusion());
                                    if (!properties.IsNormalDefined()) continue;
                                    gp_Dir normal = properties.Normal();
                                    if (face.Orientation() == TopAbs_REVERSED) normal.Reverse();
                                    outward += gp_Vec(normal);
                                } catch (const Standard_Failure&) {
                                    // A single singular adjacent surface does not invalidate the
                                    // topology snapshot; other adjacent normals may still define a
                                    // stable drag direction.
                                }
                            }
                            if (outward.SquareMagnitude() > Precision::SquareConfusion()) {
                                outward.Normalize();
                                fact_flags |= FACT_DIRECTION;
                            }
                        }
                    }
                    if (query_measurements) {
                        GProp_GProps properties;
                        BRepGProp::LinearProperties(typed_edge, properties);
                        length = properties.Mass();
                        if (std::isfinite(length)) fact_flags |= FACT_MEASUREMENT;
                    }
                } else if (query_measurements) {
                    fact_flags |= FACT_MEASUREMENT;
                }
                result.edge_geometry_kinds.push_back(geometry_kind);
                result.edge_fact_flags.push_back(fact_flags);
                result.edge_points.push_back(point.X());
                result.edge_points.push_back(point.Y());
                result.edge_points.push_back(point.Z());
                result.edge_tangents.push_back(tangent.X());
                result.edge_tangents.push_back(tangent.Y());
                result.edge_tangents.push_back(tangent.Z());
                result.edge_directions.push_back(outward.X());
                result.edge_directions.push_back(outward.Y());
                result.edge_directions.push_back(outward.Z());
                result.edge_lengths.push_back(length);
            }
        }
        for (int vertex_index = 1; vertex_index <= vertices.Extent(); ++vertex_index) {
            const TopoDS_Shape& vertex = vertices(vertex_index);
            result.vertex_tshape_ids.push_back(
                reinterpret_cast<uint64_t>(vertex.TShape().get()));
            result.vertex_location_hashes.push_back(static_cast<uint64_t>(vertex.Location().HashCode()));
            result.vertex_orientations.push_back(static_cast<uint32_t>(vertex.Orientation()));
            if (query_frames) {
                const gp_Pnt point = BRep_Tool::Pnt(TopoDS::Vertex(vertex));
                result.vertex_fact_flags.push_back(FACT_FRAME);
                result.vertex_points.push_back(point.X());
                result.vertex_points.push_back(point.Y());
                result.vertex_points.push_back(point.Z());
            }
        }
        result.success = true;
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return result;
    }
    return result;
}

ValidationData shape_validation(const TopoDS_Shape& shape) {
    ValidationData result;
    result.invalid_faces = 0;
    result.invalid_edges = 0;
    result.invalid_vertices = 0;
    result.valid = false;
    result.success = false;
    try {
        BRepCheck_Analyzer analyzer(shape, true);
        result.valid = analyzer.IsValid();
        for (TopExp_Explorer ex(shape, TopAbs_FACE); ex.More(); ex.Next()) {
            if (!analyzer.IsValid(ex.Current())) ++result.invalid_faces;
        }
        for (TopExp_Explorer ex(shape, TopAbs_EDGE); ex.More(); ex.Next()) {
            if (!analyzer.IsValid(ex.Current())) ++result.invalid_edges;
        }
        for (TopExp_Explorer ex(shape, TopAbs_VERTEX); ex.More(); ex.Next()) {
            if (!analyzer.IsValid(ex.Current())) ++result.invalid_vertices;
        }
        result.success = true;
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "validate_result", 4, failure);
    }
    return result;
}

rust::Vec<uint32_t> shared_face_indices(
    const TopoDS_Shape& first,
    const TopoDS_Shape& second)
{
    rust::Vec<uint32_t> result;
    try {
        NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> first_faces;
        NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> second_faces;
        TopExp::MapShapes(first, TopAbs_FACE, first_faces);
        TopExp::MapShapes(second, TopAbs_FACE, second_faces);
        for (int second_index = 1; second_index <= second_faces.Extent(); ++second_index) {
            const int first_index = first_faces.FindIndex(second_faces(second_index));
            if (first_index < 1) continue;
            result.push_back(static_cast<uint32_t>(first_index - 1));
            result.push_back(static_cast<uint32_t>(second_index - 1));
        }
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        result.clear();
    }
    return result;
}

std::unique_ptr<std::vector<TopoDS_Edge>> face_edges(const TopoDS_Face& face) {
    // A face's outer wire and (optional) inner wires can share edges; collapse
    // them into unique edges with the same IndexedMap trick used in shape_edges.
    NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> edgeMap;
    TopExp::MapShapes(face, TopAbs_EDGE, edgeMap);
    auto out = std::make_unique<std::vector<TopoDS_Edge>>();
    out->reserve(edgeMap.Extent());
    for (int i = 1; i <= edgeMap.Extent(); i++) {
        out->push_back(TopoDS::Edge(edgeMap(i)));
    }
    return out;
}

std::unique_ptr<std::vector<TopoDS_Edge>> face_boundary_wires(
    const TopoDS_Face& face)
{
    auto out = std::make_unique<std::vector<TopoDS_Edge>>();
    try {
        const TopoDS_Wire outer = BRepTools::OuterWire(face);
        auto append_wire = [&](const TopoDS_Wire& wire) {
            if (!out->empty()) out->push_back(TopoDS_Edge());
            for (BRepTools_WireExplorer explorer(wire, face); explorer.More();
                 explorer.Next()) {
                out->push_back(explorer.Current());
            }
        };
        if (!outer.IsNull()) append_wire(outer);
        for (TopExp_Explorer explorer(face, TopAbs_WIRE); explorer.More();
             explorer.Next()) {
            const TopoDS_Wire wire = TopoDS::Wire(explorer.Current());
            if (outer.IsNull() || !wire.IsSame(outer)) append_wire(wire);
        }
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        out->clear();
    }
    return out;
}

std::unique_ptr<TopoDS_Shape> clone_shape_handle(const TopoDS_Shape& shape) {
    return std::make_unique<TopoDS_Shape>(shape);
}

std::unique_ptr<TopoDS_Edge> clone_edge_handle(const TopoDS_Edge& edge) {
    return std::make_unique<TopoDS_Edge>(edge);
}

std::unique_ptr<TopoDS_Face> clone_face_handle(const TopoDS_Face& face) {
    return std::make_unique<TopoDS_Face>(face);
}

// ==================== Face Methods ====================

uint64_t face_tshape_id(const TopoDS_Face& face) {
    return reinterpret_cast<uint64_t>(face.TShape().get());
}

uint64_t shape_tshape_id(const TopoDS_Shape& shape) {
    return reinterpret_cast<uint64_t>(shape.TShape().get());
}

uint64_t edge_tshape_id(const TopoDS_Edge& edge) {
    return reinterpret_cast<uint64_t>(edge.TShape().get());
}

bool face_planar_frame(const TopoDS_Face& face,
    double& px, double& py, double& pz,
    double& nx, double& ny, double& nz)
{
    try {
        const Handle(Geom_Surface) surface = BRep_Tool::Surface(face);
        if (surface.IsNull()) return false;
        const double tolerance = std::max(BRep_Tool::Tolerance(face), Precision::Confusion());
        const GeomLib_IsPlanarSurface planar(surface, tolerance);
        if (!planar.IsPlanar()) return false;

        GProp_GProps properties;
        BRepGProp::SurfaceProperties(face, properties, 1.0e-9);
        const gp_Pnt center = properties.CentreOfMass();
        gp_Dir normal = planar.Plan().Axis().Direction();
        if (face.Orientation() == TopAbs_REVERSED) normal.Reverse();
        if (!std::isfinite(center.X()) || !std::isfinite(center.Y())
            || !std::isfinite(center.Z())) return false;
        px = center.X(); py = center.Y(); pz = center.Z();
        nx = normal.X(); ny = normal.Y(); nz = normal.Z();
        return true;
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return false;
    }
}

bool face_project_point(const TopoDS_Face& face,
    double px, double py, double pz,
    double& cpx, double& cpy, double& cpz,
    double& nx, double& ny, double& nz)
{
    // Default normal = zero. Returned when BRepLProp can't define a normal
    // at the closest hit (e.g. degenerate surface point or zero first
    // derivative). Caller can detect via `normal.length() == 0`.
    nx = 0.0; ny = 0.0; nz = 0.0;

    try {
        // BRepExtrema_ExtPF respects face trim, unlike Extrema_ExtPS which
        // works on the underlying infinite surface. The vertex wrapping
        // overhead (Handle alloc) is bounded — single Handle per call.
        TopoDS_Vertex vert = BRepBuilderAPI_MakeVertex(gp_Pnt(px, py, pz));
        BRepExtrema_ExtPF ext(vert, face);
        if (!ext.IsDone() || ext.NbExt() < 1) return false;

        // Pick the smallest-distance extremum.
        int best = 1;
        double best_d2 = ext.SquareDistance(1);
        for (int i = 2; i <= ext.NbExt(); ++i) {
            double d2 = ext.SquareDistance(i);
            if (d2 < best_d2) {
                best_d2 = d2;
                best = i;
            }
        }

        gp_Pnt cp = ext.Point(best);
        cpx = cp.X();
        cpy = cp.Y();
        cpz = cp.Z();

        double u, v;
        ext.Parameter(best, u, v);

        BRepAdaptor_Surface surf(face);
        BRepLProp_SLProps props(surf, u, v, /*derivOrder=*/1, Precision::Confusion());
        if (!props.IsNormalDefined()) return true;  // cp valid, normal stays 0.

        gp_Dir n = props.Normal();
        // BRepLProp returns the surface-orientation normal; flip when the
        // face is REVERSED in its enclosing shell so the caller always
        // sees an outward-pointing direction.
        if (face.Orientation() == TopAbs_REVERSED) n.Reverse();
        nx = n.X();
        ny = n.Y();
        nz = n.Z();
        return true;
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return false;
    }
}

// ==================== Edge Methods ====================

rust::Vec<double> edge_approximation_segments(
    const TopoDS_Edge& edge, double linear, double angular, bool relative)
{
    rust::Vec<double> out;
    try {
        // A standalone edge has no surface-area scale. Use its bounding-box
        // extent for relative curve approximation; solid tessellation instead
        // derives a rigid-transform-invariant scale from exact surface area.
        double eff_chord = linear;
        if (relative) {
            Bnd_Box box;
            // A stored polygon or face triangulation must not change the
            // scale used to approximate this exact curve.
            BRepBndLib::Add(edge, box, false);
            if (!box.IsVoid()) {
                double xmin, ymin, zmin, xmax, ymax, zmax;
                box.Get(xmin, ymin, zmin, xmax, ymax, zmax);
                eff_chord = linear * std::max(xmax - xmin, std::max(ymax - ymin, zmax - zmin));
            }
        }
        BRepAdaptor_Curve curve(edge);
        GCPnts_TangentialDeflection approx(curve, angular, eff_chord);

        int nb_points = approx.NbPoints();
        for (int i = 1; i <= nb_points; i++) {
            gp_Pnt p = approx.Value(i);
            out.push_back(p.X());
            out.push_back(p.Y());
            out.push_back(p.Z());
        }
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        out.clear();
    }
    return out;
}

std::unique_ptr<TopoDS_Edge> make_helix_edge(
    double ax, double ay, double az,
    double xrx, double xry, double xrz,
    double radius, double pitch, double height)
{
    try {
        if (radius < Precision::Confusion()) return nullptr;
        if (pitch < Precision::Confusion()) return nullptr;
        if (height < Precision::Confusion()) return nullptr;

        // Build a deterministic local frame: the cylinder's local +X is the
        // user-supplied x_ref (orthogonalized against axis by gp_Ax2). The
        // helix then starts at (radius, 0, 0) in this frame, which is
        // origin + radius * normalize(x_ref ⊥ axis) in world coordinates.
        gp_Dir axis_dir(ax, ay, az);
        gp_Dir x_ref(xrx, xry, xrz);
        if (axis_dir.IsParallel(x_ref, Precision::Angular())) return nullptr;
        gp_Ax2 ax2(gp_Pnt(0.0, 0.0, 0.0), axis_dir, x_ref);
        Handle(Geom_CylindricalSurface) cylinder =
            new Geom_CylindricalSurface(ax2, radius);

        double turns = height / pitch;
        double total_angle = turns * 2.0 * M_PI;
        gp_Pnt2d line_origin(0.0, 0.0);
        gp_Dir2d line_dir(total_angle, height);
        Handle(Geom2d_Line) line2d = new Geom2d_Line(line_origin, line_dir);

        double param_end = std::sqrt(total_angle * total_angle + height * height);

        BRepBuilderAPI_MakeEdge edgeMaker(line2d, cylinder, 0.0, param_end);
        if (!edgeMaker.IsDone()) return nullptr;
        TopoDS_Edge edge = edgeMaker.Edge();
        BRepLib::BuildCurve3d(edge);
        return std::make_unique<TopoDS_Edge>(edge);
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return nullptr;
    }
}

std::unique_ptr<std::vector<TopoDS_Edge>> make_polygon_edges(rust::Slice<const double> coords) {
    auto out = std::make_unique<std::vector<TopoDS_Edge>>();
    if (coords.size() < 9 || coords.size() % 3 != 0) return out;
    try {
        BRepBuilderAPI_MakePolygon poly;
        for (size_t i = 0; i + 2 < coords.size(); i += 3) {
            poly.Add(gp_Pnt(coords[i], coords[i + 1], coords[i + 2]));
        }
        poly.Close();
        if (!poly.IsDone()) return out;
        TopoDS_Wire wire = poly.Wire();
        // Walk the wire's edges in order using TopExp_Explorer.
        for (TopExp_Explorer ex(wire, TopAbs_EDGE); ex.More(); ex.Next()) {
            out->push_back(TopoDS::Edge(ex.Current()));
        }
        return out;
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        out->clear();
        return out;
    }
}

std::unique_ptr<TopoDS_Edge> make_circle_edge(
    double ax, double ay, double az, double radius)
{
    try {
        if (radius < Precision::Confusion()) return nullptr;
        gp_Dir axis_dir(ax, ay, az);
        // Single-arg gp_Ax2(origin, N): OCCT picks an arbitrary X direction
        // orthogonal to the normal. The circle's parametric start is then at
        // (radius, 0, 0) in that implicit local frame. Callers that need a
        // specific start direction should rotate the result into place.
        gp_Ax2 ax2(gp_Pnt(0.0, 0.0, 0.0), axis_dir);
        gp_Circ circ(ax2, radius);
        BRepBuilderAPI_MakeEdge edgeMaker(circ);
        if (!edgeMaker.IsDone()) return nullptr;
        return std::make_unique<TopoDS_Edge>(edgeMaker.Edge());
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return nullptr;
    }
}

std::unique_ptr<TopoDS_Edge> make_line_edge(
    double ax, double ay, double az,
    double bx, double by, double bz)
{
    try {
        gp_Pnt a(ax, ay, az);
        gp_Pnt b(bx, by, bz);
        if (a.Distance(b) < Precision::Confusion()) return nullptr;
        BRepBuilderAPI_MakeEdge edgeMaker(a, b);
        if (!edgeMaker.IsDone()) return nullptr;
        return std::make_unique<TopoDS_Edge>(edgeMaker.Edge());
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return nullptr;
    }
}

std::unique_ptr<TopoDS_Edge> make_arc_edge(
    double sx, double sy, double sz,
    double mx, double my, double mz,
    double ex, double ey, double ez)
{
    try {
        gp_Pnt p_start(sx, sy, sz);
        gp_Pnt p_mid(mx, my, mz);
        gp_Pnt p_end(ex, ey, ez);
        // false: do not wrap around; the arc goes from start through
        // mid to end on the unique circle defined by those three points.
        GC_MakeArcOfCircle maker(p_start, p_mid, p_end);
        if (!maker.IsDone()) return nullptr;
        BRepBuilderAPI_MakeEdge edgeMaker(maker.Value());
        if (!edgeMaker.IsDone()) return nullptr;
        return std::make_unique<TopoDS_Edge>(edgeMaker.Edge());
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return nullptr;
    }
}

// Cubic B-spline edge interpolating the given data points.
//
// `coords` is a flat array of xyz triples (length must be a multiple of 3
// and ≥ 6). Each (x, y, z) triple is one interpolation target — the
// resulting curve passes through every input point.
//
// `end_kind` selects the end-condition variant of `BSplineEnd`:
//   0 = Periodic — wraps around with C² continuity. Periodic is encoded
//       in the basis; the caller must NOT duplicate the first point at
//       the end. Needs ≥ 3 points (Rust side validates).
//   1 = NotAKnot — open curve, OCCT default end condition (the boundary
//       cubic is fit to 3 data points instead of being constrained by
//       an artificial derivative). Needs ≥ 2 points.
//   2 = Clamped — open curve with explicit start/end tangent vectors
//       passed in (sx, sy, sz) and (ex, ey, ez). Needs ≥ 2 points.
//
// For end_kind 0 and 1, the tangent arguments are ignored.
//
// Returns null on any failure (out-of-range end_kind, OCCT internal
// failure, degenerate point distribution).
std::unique_ptr<TopoDS_Edge> make_bspline_edge(
    rust::Slice<const double> coords,
    uint32_t end_kind,
    double sx, double sy, double sz,
    double ex, double ey, double ez)
{
    if (coords.size() < 6 || coords.size() % 3 != 0) return nullptr;
    try {
        // Local alias: `Handle(NCollection_HArray1<gp_Pnt>)` は Handle マクロが
        // template 内のカンマで引数を分割してしまうので、using alias を噛ませて
        // 単一トークン化する(コミット a72e330 で deprecated 型に戻した時の回避策)。
        using HPntArray = NCollection_HArray1<gp_Pnt>;
        const int n = static_cast<int>(coords.size() / 3);
        Handle(HPntArray) pts = new HPntArray(1, n);
        for (int i = 0; i < n; ++i) {
            pts->SetValue(i + 1, gp_Pnt(coords[i * 3], coords[i * 3 + 1], coords[i * 3 + 2]));
        }

        const bool periodic = (end_kind == 0) ? true : false;
        GeomAPI_Interpolate interp(pts, periodic, Precision::Confusion());

        if (end_kind == 2) {
            // Clamped: load explicit start and end tangent vectors.
            // Scale = true lets OCCT scale the tangent magnitude
            // by the chord length, which usually gives more intuitive
            // pull strength than the raw vector magnitude.
            gp_Vec start_tan(sx, sy, sz);
            gp_Vec end_tan(ex, ey, ez);
            interp.Load(start_tan, end_tan, true);
        } else if (end_kind != 0 && end_kind != 1) {
            // Unknown end_kind; fail rather than silently picking a default.
            return nullptr;
        }

        interp.Perform();
        if (!interp.IsDone()) return nullptr;

        Handle(Geom_BSplineCurve) curve = interp.Curve();
        if (curve.IsNull()) return nullptr;

        BRepBuilderAPI_MakeEdge edgeMaker(curve);
        if (!edgeMaker.IsDone()) return nullptr;
        return std::make_unique<TopoDS_Edge>(edgeMaker.Edge());
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return nullptr;
    }
}

std::unique_ptr<TopoDS_Edge> make_bspline_poles_edge(
    rust::Slice<const double> coords, uint32_t degree, rust::Slice<const double> knots)
{
    if (degree < 1 || degree > 25 || coords.size() % 3 != 0
        || coords.size() / 3 <= degree || coords.size() / 3 > std::numeric_limits<int>::max()
        || knots.size() > std::numeric_limits<int>::max()
        || knots.size() != coords.size() / 3 + degree + 1) return nullptr;
    try {
        NCollection_Array1<gp_Pnt> poles(1, static_cast<int>(coords.size() / 3));
        for (int i = 1; i <= poles.Length(); ++i) {
            const size_t offset = static_cast<size_t>(i-1) * 3;
            poles.SetValue(i, gp_Pnt(coords[offset], coords[offset+1], coords[offset+2]));
        }
        std::vector<double> values;
        std::vector<int> multiplicities;
        for (double knot : knots) {
            if (!std::isfinite(knot) || (!values.empty() && knot < values.back())) return nullptr;
            if (values.empty() || knot != values.back()) {
                values.push_back(knot);
                multiplicities.push_back(1);
            } else {
                ++multiplicities.back();
            }
        }
        NCollection_Array1<double> native_knots(1, static_cast<int>(values.size()));
        NCollection_Array1<int> native_mults(1, static_cast<int>(values.size()));
        for (int i = 1; i <= native_knots.Length(); ++i) {
            native_knots.SetValue(i, values[i-1]);
            native_mults.SetValue(i, multiplicities[i-1]);
        }
        Handle(Geom_BSplineCurve) curve = new Geom_BSplineCurve(poles, native_knots, native_mults, degree, false);
        BRepBuilderAPI_MakeEdge builder(curve);
        if (!builder.IsDone()) return nullptr;
        return std::make_unique<TopoDS_Edge>(builder.Edge());
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return nullptr;
    }
}

void edge_endpoints(const TopoDS_Edge& edge,
    double& sx, double& sy, double& sz,
    double& ex, double& ey, double& ez)
{
    sx = 0.0; sy = 0.0; sz = 0.0;
    ex = 0.0; ey = 0.0; ez = 0.0;
    try {
        BRepAdaptor_Curve curve(edge);
        gp_Pnt start = curve.Value(curve.FirstParameter());
        gp_Pnt end = curve.Value(curve.LastParameter());
        sx = start.X(); sy = start.Y(); sz = start.Z();
        ex = end.X();   ey = end.Y();   ez = end.Z();
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
    }
}

void edge_tangents(const TopoDS_Edge& edge,
    double& sx, double& sy, double& sz,
    double& ex, double& ey, double& ez)
{
    sx = 0.0; sy = 0.0; sz = 0.0;
    ex = 0.0; ey = 0.0; ez = 0.0;
    try {
        BRepAdaptor_Curve curve(edge);
        gp_Pnt p;
        gp_Vec vs, ve;
        curve.D1(curve.FirstParameter(), p, vs);
        curve.D1(curve.LastParameter(), p, ve);
        if (vs.Magnitude() > Precision::Confusion()) {
            vs.Normalize();
            sx = vs.X(); sy = vs.Y(); sz = vs.Z();
        }
        if (ve.Magnitude() > Precision::Confusion()) {
            ve.Normalize();
            ex = ve.X(); ey = ve.Y(); ez = ve.Z();
        }
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
    }
}

bool edge_is_closed(const TopoDS_Edge& edge) {
    try {
        BRepAdaptor_Curve curve(edge);
        gp_Pnt p_start = curve.Value(curve.FirstParameter());
        gp_Pnt p_end   = curve.Value(curve.LastParameter());
        return p_start.Distance(p_end) < Precision::Confusion();
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return false;
    }
}

bool edge_project_point(const TopoDS_Edge& edge,
    double px, double py, double pz,
    double& cpx, double& cpy, double& cpz,
    double& tx, double& ty, double& tz)
{
    cpx = 0.0; cpy = 0.0; cpz = 0.0;
    tx = 0.0; ty = 0.0; tz = 0.0;
    try {
        double first = 0.0, last = 0.0;
        Handle(Geom_Curve) gcurve = BRep_Tool::Curve(edge, first, last);
        if (gcurve.IsNull()) return false;
        gp_Pnt target(px, py, pz);
        GeomAPI_ProjectPointOnCurve projector(target, gcurve, first, last);
        double u;
        if (projector.NbPoints() > 0) {
            u = projector.LowerDistanceParameter();
        } else {
            // No interior extremum within [first, last] — distance is monotonic
            // along the curve segment (e.g. line segment with target beyond an
            // endpoint). Clamp to whichever endpoint is closer.
            double d_first = target.Distance(gcurve->Value(first));
            double d_last  = target.Distance(gcurve->Value(last));
            u = (d_first <= d_last) ? first : last;
        }
        gp_Pnt p;
        gp_Vec v;
        gcurve->D1(u, p, v);
        cpx = p.X(); cpy = p.Y(); cpz = p.Z();
        if (v.Magnitude() > Precision::Confusion()) {
            v.Normalize();
            tx = v.X(); ty = v.Y(); tz = v.Z();
        }
        return true;
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return false;
    }
}

bool edge_midpoint(const TopoDS_Edge& edge,
    double& px, double& py, double& pz,
    double& tx, double& ty, double& tz)
{
    px = 0.0; py = 0.0; pz = 0.0;
    tx = 0.0; ty = 0.0; tz = 0.0;
    try {
        BRepAdaptor_Curve curve(edge);
        const double first = curve.FirstParameter();
        const double last = curve.LastParameter();
        const double length = GCPnts_AbscissaPoint::Length(curve, first, last);
        if (!std::isfinite(length) || length <= Precision::Confusion()) return false;
        GCPnts_AbscissaPoint halfway(curve, length * 0.5, first);
        if (!halfway.IsDone()) return false;
        gp_Pnt point;
        gp_Vec tangent;
        curve.D1(halfway.Parameter(), point, tangent);
        if (tangent.Magnitude() <= Precision::Confusion()) return false;
        tangent.Normalize();
        px = point.X(); py = point.Y(); pz = point.Z();
        tx = tangent.X(); ty = tangent.Y(); tz = tangent.Z();
        return true;
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return false;
    }
}

std::unique_ptr<TopoDS_Edge> deep_copy_edge(const TopoDS_Edge& edge) {
    try {
        BRepBuilderAPI_Copy copier(edge);
        return std::make_unique<TopoDS_Edge>(TopoDS::Edge(copier.Shape()));
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return nullptr;
    }
}

// Helper: apply a gp_Trsf to an edge via BRepBuilderAPI_Transform.
// Used for all four edge transforms below.
static std::unique_ptr<TopoDS_Edge> transform_edge_impl(
    const TopoDS_Edge& edge, const gp_Trsf& trsf)
{
    try {
        BRepBuilderAPI_Transform transform(edge, trsf, true);
        return std::make_unique<TopoDS_Edge>(TopoDS::Edge(transform.Shape()));
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return nullptr;
    }
}

std::unique_ptr<TopoDS_Edge> translate_edge(
    const TopoDS_Edge& edge, double tx, double ty, double tz)
{
    gp_Trsf trsf;
    trsf.SetTranslation(gp_Vec(tx, ty, tz));
    return transform_edge_impl(edge, trsf);
}

std::unique_ptr<TopoDS_Edge> rotate_edge(
    const TopoDS_Edge& edge,
    double ox, double oy, double oz,
    double dx, double dy, double dz,
    double angle)
{
    try {
        gp_Trsf trsf;
        trsf.SetRotation(gp_Ax1(gp_Pnt(ox, oy, oz), gp_Dir(dx, dy, dz)), angle);
        return transform_edge_impl(edge, trsf);
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return nullptr;
    }
}

std::unique_ptr<TopoDS_Edge> scale_edge(
    const TopoDS_Edge& edge,
    double cx, double cy, double cz,
    double factor)
{
    gp_Trsf trsf;
    trsf.SetScale(gp_Pnt(cx, cy, cz), factor);
    return transform_edge_impl(edge, trsf);
}

std::unique_ptr<TopoDS_Edge> mirror_edge(
    const TopoDS_Edge& edge,
    double ox, double oy, double oz,
    double nx, double ny, double nz)
{
    try {
        gp_Trsf trsf;
        trsf.SetMirror(gp_Ax2(gp_Pnt(ox, oy, oz), gp_Dir(nx, ny, nz)));
        return transform_edge_impl(edge, trsf);
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return nullptr;
    }
}

std::unique_ptr<std::vector<TopoDS_Edge>> edge_vec_new() {
    return std::make_unique<std::vector<TopoDS_Edge>>();
}

void edge_vec_push(std::vector<TopoDS_Edge>& v, const TopoDS_Edge& e) {
    v.push_back(e);
}

void edge_vec_push_null(std::vector<TopoDS_Edge>& v) {
    v.push_back(TopoDS_Edge());
}

bool edge_is_null(const TopoDS_Edge& edge) {
    return edge.IsNull();
}

std::unique_ptr<std::vector<TopoDS_Face>> face_vec_new() {
    return std::make_unique<std::vector<TopoDS_Face>>();
}

void face_vec_push(std::vector<TopoDS_Face>& v, const TopoDS_Face& f) {
    v.push_back(f);
}

std::unique_ptr<std::vector<TopoDS_Shape>> shape_vec_new() {
    return std::make_unique<std::vector<TopoDS_Shape>>();
}

void shape_vec_push(std::vector<TopoDS_Shape>& v, const TopoDS_Shape& s) {
    v.push_back(s);
}

std::unique_ptr<TopoDS_Shape> make_thickened_face_region(
    const std::vector<TopoDS_Face>& faces, double distance,
    const CancellationToken& progress)
{
    try {
        if (faces.empty() || !std::isfinite(distance) || distance <= Precision::Confusion()) {
            record_input_failure(__func__, "a face region needs positive extrusion distance");
            return nullptr;
        }
        if (rust_progress_cancelled(progress)) return nullptr;
        BRep_Builder shell_builder;
        TopoDS_Shell shell;
        shell_builder.MakeShell(shell);
        for (const auto& face : faces) shell_builder.Add(shell, face);
        // Offset builders can change tolerances; keep the source artifact immutable.
        BRepBuilderAPI_Copy copy(shell, true, false);
        BRepOffset_MakeOffset builder;
        builder.Initialize(copy.Shape(), distance, 1.0e-7, BRepOffset_Skin,
            false, false, GeomAbs_Intersection, true);
        Handle(RustProgressIndicator) indicator = new RustProgressIndicator(progress);
        builder.MakeOffsetShape(indicator->Start());
        if (rust_progress_cancelled(progress)) return nullptr;
        if (!builder.IsDone() || builder.Shape().IsNull()
            || !BRepCheck_Analyzer(builder.Shape()).IsValid()) {
            record_input_failure(__func__, "these faces cannot be extruded together at this distance; reduce the distance or turn off Unify adjacent faces");
            return nullptr;
        }
        return std::make_unique<TopoDS_Shape>(builder.Shape());
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return nullptr;
    }
}

std::unique_ptr<TopoDS_Shape> builder_thick_solid(
    const TopoDS_Shape& solid,
    const std::vector<TopoDS_Face>& open_faces,
    double thickness,
    const CancellationToken& progress,
    rust::Vec<uint64_t>& out_history,
    HistoryData& out_topology_history)
{
    try {
        if (!std::isfinite(thickness) || std::abs(thickness) < Precision::Confusion()) {
            record_input_failure(__func__, "shell thickness must be finite and nonzero");
            return nullptr;
        }
        if (rust_progress_cancelled(progress)) return nullptr;
        // Empty open_faces: MakeThickSolidByJoin degenerates to a plain offset
        // shape (no cavity) because it needs at least one removed face to
        // build the first wall W1. Instead, build the solid explicitly as
        // outer_shell + reversed inner_shell so the result is a sealed solid
        // with an internal void (OCCT permits multi-shell solids).
        if (open_faces.empty()) {
            BRepOffsetAPI_MakeOffsetShape offset;
            offset.PerformByJoin(
                solid, thickness,
                /*tolerance=*/ 1.0e-6,
                /*mode=*/ BRepOffset_Skin,
                /*intersection=*/ false,
                /*selfInter=*/ false,
                /*join=*/ GeomAbs_Arc);
            if (rust_progress_cancelled(progress)) return nullptr;
            if (!offset.IsDone()) return nullptr;
            TopoDS_Shape offset_shape = offset.Shape();

            auto extract_shell = [](const TopoDS_Shape& s) -> TopoDS_Shell {
                if (s.ShapeType() == TopAbs_SHELL) return TopoDS::Shell(s);
                TopExp_Explorer ex(s, TopAbs_SHELL);
                if (!ex.More()) return TopoDS_Shell();
                return TopoDS::Shell(ex.Current());
            };

            TopoDS_Shell original_shell = extract_shell(solid);
            TopoDS_Shell offset_shell = extract_shell(offset_shape);
            if (original_shell.IsNull() || offset_shell.IsNull()) return nullptr;

            // thickness sign determines which shell is outer:
            //   negative → offset shrinks inward: original = outer, offset = inner cavity
            //   positive → offset expands outward: offset = outer, original = inner cavity
            TopoDS_Shell outer = thickness < 0.0 ? original_shell : offset_shell;
            TopoDS_Shell inner = thickness < 0.0 ? offset_shell : original_shell;

            BRepBuilderAPI_MakeSolid solid_maker(outer);
            solid_maker.Add(TopoDS::Shell(inner.Reversed()));
            if (rust_progress_cancelled(progress)) return nullptr;
            if (!solid_maker.IsDone()) return nullptr;
            // Sealed case: original faces are retained as identity; offset walls
            // are Generated (src is an edge) and intentionally absent.
            std::unordered_map<uint64_t, uint64_t> relay;
            relay_from_pair(solid, solid, relay);
            relay_into_history(&relay, nullptr, out_history);
            auto result = std::make_unique<TopoDS_Shape>(solid_maker.Solid());
            const HistoryMaps result_maps(*result);
            append_builder_topology_history(offset, solid, 0, result_maps,
                out_topology_history);
            finish_topology_history(result_maps, out_topology_history);
            rust_progress_set(progress, 1.0);
            return result;
        }

        NCollection_List<TopoDS_Shape> faces_to_remove;
        for (const auto& f : open_faces) faces_to_remove.Append(f);

        BRepOffsetAPI_MakeThickSolid builder;
        builder.MakeThickSolidByJoin(
            solid, faces_to_remove, thickness,
            /*tolerance=*/ 1.0e-6,
            /*mode=*/ BRepOffset_Skin,
            /*intersection=*/ false,
            /*selfInter=*/ false,
            /*join=*/ GeomAbs_Arc);
        if (rust_progress_cancelled(progress)) return nullptr;
        builder.Build();
        if (rust_progress_cancelled(progress)) return nullptr;
        if (!builder.IsDone()) return nullptr;
        // No copy, so relay keys are final faces.
        std::unordered_map<uint64_t, uint64_t> relay;
        relay_from_builder(builder, solid, relay);
        // MakeThickSolid does not flag removed open faces as IsDeleted; drop
        // their (identity) pairs since those faces are absent from the result.
        for (const auto& f : open_faces) {
            uint64_t removed_id = reinterpret_cast<uint64_t>(f.TShape().get());
            for (auto it = relay.begin(); it != relay.end(); ) {
                if (it->second == removed_id) it = relay.erase(it);
                else ++it;
            }
        }
        relay_into_history(&relay, nullptr, out_history);
        auto result = std::make_unique<TopoDS_Shape>(builder.Shape());
        const HistoryMaps result_maps(*result);
        append_builder_topology_history(builder, solid, 0, result_maps,
            out_topology_history);
        append_shared_topology_history(solid, 0, result_maps,
            out_topology_history);
        append_aggregate_generated_history(solid, 0, result_maps,
            HistoryKind::Face, {HistoryKind::Face}, out_topology_history);
        append_aggregate_generated_history(solid, 0, result_maps,
            HistoryKind::Edge, {HistoryKind::Edge}, out_topology_history);
        append_aggregate_generated_history(solid, 0, result_maps,
            HistoryKind::Vertex, {HistoryKind::Vertex}, out_topology_history);
        remove_related_deleted_topology(out_topology_history);
        finish_topology_history(result_maps, out_topology_history);
        rust_progress_set(progress, 1.0);
        return result;
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return nullptr;
    }
}

bool blend_tolerances_fit(const TopoDS_Shape& source, const TopoDS_Shape& result, double size)
{
    const auto maximum_boundary_tolerance = [](const TopoDS_Shape& shape) {
        double tolerance = Precision::Confusion();
        for (TopExp_Explorer ex(shape, TopAbs_EDGE); ex.More(); ex.Next()) {
            tolerance = std::max(tolerance, BRep_Tool::Tolerance(TopoDS::Edge(ex.Current())));
        }
        for (TopExp_Explorer ex(shape, TopAbs_VERTEX); ex.More(); ex.Next()) {
            tolerance = std::max(tolerance, BRep_Tool::Tolerance(TopoDS::Vertex(ex.Current())));
        }
        return tolerance;
    };
    // OCCT can report a valid solid by inflating boundary tolerances past the blend
    // size, leaving gaps between its 3D edges and supporting surface curves.
    const double allowed = std::max(maximum_boundary_tolerance(source) * 2.0, size * 0.01);
    if (maximum_boundary_tolerance(result) <= allowed) return true;
    try {
        // Large stored tolerances can be conservative. Reject measured gaps,
        // including endpoints snapped to shared vertices by the display mesher.
        for (TopExp_Explorer faces(result, TopAbs_FACE); faces.More(); faces.Next()) {
            const TopoDS_Face face = TopoDS::Face(faces.Current());
            BRepAdaptor_Surface surface(face);
            for (TopExp_Explorer edges(face, TopAbs_EDGE); edges.More(); edges.Next()) {
                const TopoDS_Edge edge = TopoDS::Edge(edges.Current().Oriented(TopAbs_FORWARD));
                const TopoDS_Vertex first_vertex = TopExp::FirstVertex(edge);
                const TopoDS_Vertex last_vertex = TopExp::LastVertex(edge);
                if (first_vertex.IsNull() || last_vertex.IsNull()) return false;
                BRepAdaptor_Curve2d pcurve(edge, face);
                for (int sample = 0; sample <= 4; ++sample) {
                    const double parameter = pcurve.FirstParameter()
                        + (pcurve.LastParameter() - pcurve.FirstParameter()) * sample / 4.0;
                    const gp_Pnt2d uv = pcurve.Value(parameter);
                    gp_Pnt point;
                    if (sample == 0 || BRep_Tool::Degenerated(edge)) {
                        point = BRep_Tool::Pnt(first_vertex);
                    } else if (sample == 4) {
                        point = BRep_Tool::Pnt(last_vertex);
                    } else {
                        point = BRepAdaptor_Curve(edge).Value(parameter);
                    }
                    const double error = surface.Value(uv.X(), uv.Y()).Distance(point);
                    if (!std::isfinite(error) || error > allowed) {
                        if (std::getenv("PLEX_TESSELLATION_DIAGNOSTICS")) {
                            std::cerr << "blend boundary rejected: size=" << size << " sample=" << sample
                                << " error=" << error << " allowed=" << allowed
                                << " source tolerance=" << maximum_boundary_tolerance(source)
                                << " result tolerance=" << maximum_boundary_tolerance(result) << std::endl;
                        }
                        return false;
                    }
                }
            }
        }
    } catch (const Standard_Failure&) {
        return false;
    }
    return true;
}

#ifndef __wasm__
// OCCT 8.0 blend solvers share mutable scratch state across otherwise independent shapes.
// Keep this gate specific to their builders; tessellation and other operations stay parallel.
static std::timed_mutex edge_blend_mutex;
#endif

std::unique_ptr<TopoDS_Shape> builder_fillet(
    const TopoDS_Shape& solid,
    const std::vector<TopoDS_Edge>& edges,
    double radius,
    const CancellationToken& progress,
    rust::Vec<uint64_t>& out_history,
    HistoryData& out_topology_history)
{
    try {
        if (rust_progress_cancelled(progress)) return nullptr;
#ifndef __wasm__
        std::unique_lock<std::timed_mutex> blend_lock(edge_blend_mutex, std::defer_lock);
        while (!blend_lock.try_lock_for(std::chrono::milliseconds(10))) {
            if (rust_progress_cancelled(progress)) return nullptr;
        }
#endif
        if (edges.empty()) {
            // No-op: shallow copy; every face is identity.
            std::unordered_map<uint64_t, uint64_t> relay;
            relay_from_pair(solid, solid, relay);
            relay_into_history(&relay, nullptr, out_history);
            auto result = std::make_unique<TopoDS_Shape>(solid);
            const HistoryMaps result_maps(*result);
            append_identity_topology_history(solid, 0, result_maps,
                out_topology_history);
            finish_topology_history(result_maps, out_topology_history);
            return result;
        }
        BRepFilletAPI_MakeFillet mk(solid);
        for (const TopoDS_Edge& e : edges) {
            if (e.IsNull()) continue;
            mk.Add(radius, e);
        }
        Handle(RustProgressIndicator) indicator = new RustProgressIndicator(progress);
        mk.Build(indicator->Start());
        if (rust_progress_cancelled(progress)) return nullptr;
        if (!mk.IsDone()) return nullptr;
        TopoDS_Shape result = mk.Shape();
        if (result.IsNull()) return nullptr;
        // MakeFillet can wrap the solid in a compound even if it contains only one solid.
        // Solid::new requires a TopAbs_SOLID, so extract the first one if we got a container.
        if (result.ShapeType() != TopAbs_SOLID) {
            TopExp_Explorer ex(result, TopAbs_SOLID);
            if (!ex.More()) return nullptr;
            result = ex.Current();
        }
        // No copy, so relay keys are final faces (identity for untouched).
        std::unordered_map<uint64_t, uint64_t> relay;
        relay_from_builder(mk, solid, relay);
        relay_into_history(&relay, nullptr, out_history);
        auto output = std::make_unique<TopoDS_Shape>(result);
        const HistoryMaps result_maps(*output);
        append_builder_topology_history(mk, solid, 0, result_maps,
            out_topology_history);
        append_shared_topology_history(solid, 0, result_maps,
            out_topology_history);
        append_aggregate_generated_history(solid, 0, result_maps,
            HistoryKind::Face, {HistoryKind::Face, HistoryKind::Edge},
            out_topology_history);
        append_aggregate_generated_history(solid, 0, result_maps,
            HistoryKind::Edge, {HistoryKind::Edge, HistoryKind::Vertex},
            out_topology_history);
        append_aggregate_generated_history(solid, 0, result_maps,
            HistoryKind::Vertex, {HistoryKind::Vertex},
            out_topology_history);
        remove_related_deleted_topology(out_topology_history);
        finish_topology_history(result_maps, out_topology_history);
        return output;
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return nullptr;
    }
}

std::unique_ptr<TopoDS_Shape> builder_chamfer(
    const TopoDS_Shape& solid,
    const std::vector<TopoDS_Edge>& edges,
    double distance,
    const CancellationToken& progress,
    rust::Vec<uint64_t>& out_history,
    HistoryData& out_topology_history)
{
    try {
        if (rust_progress_cancelled(progress)) return nullptr;
#ifndef __wasm__
        std::unique_lock<std::timed_mutex> blend_lock(edge_blend_mutex, std::defer_lock);
        while (!blend_lock.try_lock_for(std::chrono::milliseconds(10))) {
            if (rust_progress_cancelled(progress)) return nullptr;
        }
#endif
        if (edges.empty()) {
            // No-op: shallow copy; every face is identity.
            std::unordered_map<uint64_t, uint64_t> relay;
            relay_from_pair(solid, solid, relay);
            relay_into_history(&relay, nullptr, out_history);
            auto result = std::make_unique<TopoDS_Shape>(solid);
            const HistoryMaps result_maps(*result);
            append_identity_topology_history(solid, 0, result_maps,
                out_topology_history);
            finish_topology_history(result_maps, out_topology_history);
            return result;
        }
        BRepFilletAPI_MakeChamfer mk(solid);
        for (const TopoDS_Edge& e : edges) {
            if (e.IsNull()) continue;
            mk.Add(distance, e);
        }
        Handle(RustProgressIndicator) indicator = new RustProgressIndicator(progress);
        mk.Build(indicator->Start());
        if (rust_progress_cancelled(progress)) return nullptr;
        if (!mk.IsDone()) return nullptr;
        TopoDS_Shape result = mk.Shape();
        if (result.IsNull()) return nullptr;
        // Like MakeFillet, MakeChamfer may wrap the result in a compound even if it contains only one solid.
        // Extract the first solid so Solid::new's TopAbs_SOLID invariant holds.
        if (result.ShapeType() != TopAbs_SOLID) {
            TopExp_Explorer ex(result, TopAbs_SOLID);
            if (!ex.More()) return nullptr;
            result = ex.Current();
        }
        // No copy, so relay keys are final faces (identity for untouched).
        std::unordered_map<uint64_t, uint64_t> relay;
        relay_from_builder(mk, solid, relay);
        relay_into_history(&relay, nullptr, out_history);
        auto output = std::make_unique<TopoDS_Shape>(result);
        const HistoryMaps result_maps(*output);
        append_builder_topology_history(mk, solid, 0, result_maps,
            out_topology_history);
        append_shared_topology_history(solid, 0, result_maps,
            out_topology_history);
        append_aggregate_generated_history(solid, 0, result_maps,
            HistoryKind::Face, {HistoryKind::Face, HistoryKind::Edge},
            out_topology_history);
        append_aggregate_generated_history(solid, 0, result_maps,
            HistoryKind::Edge, {HistoryKind::Edge, HistoryKind::Vertex},
            out_topology_history);
        append_aggregate_generated_history(solid, 0, result_maps,
            HistoryKind::Vertex, {HistoryKind::Vertex},
            out_topology_history);
        remove_related_deleted_topology(out_topology_history);
        finish_topology_history(result_maps, out_topology_history);
        return output;
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return nullptr;
    }
}

ProfileArrangementData arrange_planar_edges(const std::vector<TopoDS_Edge>& input,
    double tolerance, uint32_t max_fragments, const CancellationToken& progress) {
    ProfileArrangementData output;
    output.success = false;
    output.error_code = 0;
    const char* stage = "validate_input";
    ScopedFailureDiagnostic diagnostic(__func__, stage, output.success, progress);
    try {
        if (rust_progress_cancelled(progress)) return output;
        if (input.empty()) { output.success = true; return output; }
        if (input.size() > max_fragments) { output.error_code = 4; return output; }
        std::vector<TopoDS_Edge> sources;
        NCollection_List<TopoDS_Shape> arguments;
        for (size_t i = 0; i < input.size(); ++i) {
            if (rust_progress_cancelled(progress)) return output;
            Bnd_Box bounds;
            BRepBndLib::AddOptimal(input[i], bounds, false, false);
            double x0,y0,z0,x1,y1,z1;
            if (bounds.IsVoid() || bounds.IsWhole()) return output;
            bounds.Get(x0,y0,z0,x1,y1,z1);
            if (!std::isfinite(z0) || !std::isfinite(z1) || std::abs(z0)>tolerance || std::abs(z1)>tolerance) {
                output.error_code = 1; output.error_sources.push_back(i); return output;
            }
            sources.push_back(TopoDS::Edge(BRepBuilderAPI_Copy(input[i], false, false).Shape()));
            arguments.Append(sources.back());
        }
        stage = "split_intersections";
        Handle(RustProgressIndicator) indicator = new RustProgressIndicator(progress);
        Message_ProgressScope scope(indicator->Start(), "Planar profile arrangement", 2);
        BOPAlgo_Builder splitter;
        TopoDS_Shape split_shape = sources.front();
        if (sources.size() > 1) {
            splitter.SetArguments(arguments);
            splitter.SetNonDestructive(true);
            splitter.SetRunParallel(false);
            splitter.SetFuzzyValue(tolerance);
            splitter.Perform(scope.Next());
            if (splitter.HasErrors() || rust_progress_cancelled(progress)) return output;
            split_shape = splitter.Shape();
        } else { scope.Next(); }
        NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> fragments;
        TopExp::MapShapes(split_shape, TopAbs_EDGE, fragments);
        if (static_cast<uint64_t>(fragments.Extent()) > max_fragments) { output.error_code = 4; return output; }
        std::vector<std::vector<uint32_t>> origins(fragments.Extent()+1);
        for (size_t source = 0; source < sources.size(); ++source) {
            auto assign = [&](const TopoDS_Shape& shape) {
                int index = fragments.FindIndex(shape);
                if (index) origins[index].push_back(source);
            };
            if (sources.size() == 1 || splitter.Modified(sources[source]).IsEmpty()) assign(sources[source]);
            else for (const auto& image : splitter.Modified(sources[source])) assign(image);
        }
        stage = "verify_source_correspondence";
        NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> junctions;
        std::vector<std::vector<std::pair<uint32_t,gp_Pnt>>> junction_points;
        for (int i = 1; i <= fragments.Extent(); ++i) {
            if (origins[i].empty()) return output;
            if (origins[i].size() != 1) {
                output.error_code = 2;
                for (auto source : origins[i]) output.error_sources.push_back(source);
                return output;
            }
            const auto source = origins[i].front();
            double first,last,source_first,source_last;
            auto curve = BRep_Tool::Curve(sources[source], source_first, source_last);
            auto edge = TopoDS::Edge(fragments(i).Oriented(TopAbs_FORWARD));
            BRep_Tool::Range(edge, first, last);
            if (curve.IsNull() || !std::isfinite(first) || !std::isfinite(last)) return output;
            TopoDS_Vertex start,end;
            TopExp::Vertices(edge,start,end);
            for (const auto& endpoint : {std::make_pair(start,first),std::make_pair(end,last)}) {
                if (endpoint.first.IsNull()) return output;
                int index = junctions.Add(endpoint.first);
                if (junction_points.size() < static_cast<size_t>(index)) junction_points.resize(index);
                auto point = curve->Value(endpoint.second);
                for (const auto& previous : junction_points[index-1]) {
                    if (previous.second.Distance(point) > tolerance) {
                        output.error_code = 3;
                        output.error_sources.push_back(previous.first);
                        if (previous.first != source) output.error_sources.push_back(source);
                        return output;
                    }
                }
                junction_points[index-1].emplace_back(source,point);
            }
        }
        stage = "build_bounded_cells";
        const auto plane = BRepBuilderAPI_MakeFace(gp_Pln(gp::XOY())).Face();
        NCollection_List<TopoDS_Shape> directed;
        for (int i = 1; i <= fragments.Extent(); ++i) {
            auto edge = TopoDS::Edge(fragments(i).Oriented(TopAbs_FORWARD));
            BRepLib::BuildPCurveForEdgeOnPlane(edge,plane);
            directed.Append(edge); directed.Append(edge.Reversed());
        }
        BOPAlgo_BuilderFace builder;
        builder.SetFace(plane);
        builder.SetShapes(directed);
        builder.SetAvoidInternalShapes(true);
        builder.Perform(scope.Next());
        if (builder.HasErrors() || rust_progress_cancelled(progress)) return output;
        std::vector<bool> used(sources.size(), false);
        uint64_t exported_spans = 0;
        stage = "export_boundaries";
        for (const auto& shape : builder.Areas()) {
            if (rust_progress_cancelled(progress)) return output;
            auto face = TopoDS::Face(shape);
            BRepTopAdaptor_FClass2d classifier(face,tolerance);
            if (classifier.PerformInfinitePoint() != TopAbs_OUT) continue;
            if (!BRepCheck_Analyzer(face).IsValid()) return output;
            GProp_GProps properties;
            BRepGProp::SurfaceProperties(face,properties,1.0e-9);
            ProfileRegionData region;
            region.area = properties.Mass();
            NCollection_IndexedMap<TopoDS_Shape, TopTools_ShapeMapHasher> boundary_vertices;
            std::vector<uint32_t> boundary_vertex_sources;
            const auto outer = BRepTools::OuterWire(face);
            if (outer.IsNull()) return output;
            std::vector<TopoDS_Wire> wires{outer};
            for (TopExp_Explorer wire(face,TopAbs_WIRE); wire.More(); wire.Next()) {
                if (!wire.Current().IsSame(outer)) wires.push_back(TopoDS::Wire(wire.Current()));
            }
            region.wire_offsets.push_back(0);
            for (const auto& wire : wires) {
                for (BRepTools_WireExplorer edge(wire,face); edge.More(); edge.Next()) {
                    if (++exported_spans > uint64_t(max_fragments)*2) { output.error_code = 4; return output; }
                    const int index = fragments.FindIndex(edge.Current());
                    if (!index || origins[index].size()!=1) return output;
                    const auto source = origins[index].front();
                    const auto& vertex = edge.CurrentVertex();
                    const int previous = boundary_vertices.FindIndex(vertex);
                    if (previous) {
                        output.error_code = 3;
                        output.error_sources.push_back(boundary_vertex_sources[previous-1]);
                        if (boundary_vertex_sources[previous-1] != source) output.error_sources.push_back(source);
                        return output;
                    }
                    boundary_vertices.Add(vertex);
                    boundary_vertex_sources.push_back(source);
                    double first,last,a,b;
                    BRep_Tool::Range(edge.Current(),first,last);
                    BRep_Tool::Range(sources[source],a,b);
                    if (!(b>a)) return output;
                    double t0=(first-a)/(b-a),t1=(last-a)/(b-a);
                    const double roundoff = 64*std::numeric_limits<double>::epsilon();
                    if (t0 < -roundoff || t1 > 1+roundoff || !(t1>t0)) return output;
                    auto source_curve = BRep_Tool::Curve(sources[source],a,b);
                    if (source_curve.IsNull()
                        || source_curve->Value(first).Distance(source_curve->Value(a+(b-a)*std::clamp(t0,0.0,1.0))) > tolerance
                        || source_curve->Value(last).Distance(source_curve->Value(a+(b-a)*std::clamp(t1,0.0,1.0))) > tolerance) return output;
                    region.spans.push_back(ProfileSpanData{source,std::clamp(t0,0.0,1.0),std::clamp(t1,0.0,1.0),edge.Current().Orientation()==TopAbs_REVERSED});
                    used[source] = true;
                }
                region.wire_offsets.push_back(region.spans.size());
            }
            output.regions.push_back(std::move(region));
        }
        for (size_t i = 0; i < sources.size(); ++i) if (!used[i]) output.unused_sources.push_back(i);
        output.success = true;
        return output;
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, stage, 7, failure);
        return output;
    }
}

std::unique_ptr<TopoDS_Edge> trim_profile_edge(const TopoDS_Edge& source,
    double first, double last, bool reversed, double tolerance) {
    try {
        double start,end;
        auto curve = BRep_Tool::Curve(source,start,end);
        if (curve.IsNull()) return nullptr;
        BRepBuilderAPI_MakeEdge maker(curve,start+(end-start)*first,start+(end-start)*last);
        if (!maker.IsDone()) return nullptr;
        auto edge = maker.Edge();
        BRep_Builder builder;
        for (TopExp_Explorer vertex(edge,TopAbs_VERTEX); vertex.More(); vertex.Next()) {
            builder.UpdateVertex(TopoDS::Vertex(vertex.Current()),tolerance);
        }
        if (reversed) edge.Reverse();
        return std::make_unique<TopoDS_Edge>(edge);
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "trim_source", 7, failure);
        return nullptr;
    }
}

bool classify_planar_profile(const std::vector<TopoDS_Edge>& edges,
    rust::Slice<const double> points, double tolerance, const CancellationToken& progress,
    rust::Vec<uint8_t>& locations) {
    try {
        if (edges.empty() || points.size()%2 || rust_progress_cancelled(progress)) return false;
        BRepBuilderAPI_MakeFace face_maker{gp_Pln(gp::XOY())};
        BRepBuilderAPI_MakeWire wire_maker;
        bool has_edges = false;
        auto flush_wire = [&]() -> bool {
            if (!has_edges || !wire_maker.IsDone() || !wire_maker.Wire().Closed()) return false;
            face_maker.Add(wire_maker.Wire());
            wire_maker = BRepBuilderAPI_MakeWire();
            has_edges = false;
            return true;
        };
        for (const auto& edge : edges) {
            if (rust_progress_cancelled(progress)) return false;
            if (edge.IsNull()) {
                if (!flush_wire()) return false;
            } else {
                wire_maker.Add(TopoDS::Edge(BRepBuilderAPI_Copy(edge,false,false).Shape()));
                has_edges = true;
            }
        }
        if (!flush_wire() || !face_maker.IsDone()) return false;
        const auto face = face_maker.Face();
        if (!BRepCheck_Analyzer(face).IsValid()) return false;
        BRepTopAdaptor_FClass2d bounded(face,tolerance);
        if (bounded.PerformInfinitePoint() != TopAbs_OUT) return false;
        TopoDS_Compound boundary;
        BRep_Builder boundary_builder;
        boundary_builder.MakeCompound(boundary);
        for (TopExp_Explorer wire(face,TopAbs_WIRE); wire.More(); wire.Next()) {
            boundary_builder.Add(boundary,wire.Current());
        }
        Handle(RustProgressIndicator) indicator = new RustProgressIndicator(progress);
        Message_ProgressScope scope(indicator->Start(), "Classify profile points", points.size()/2);
        BRepClass_FaceClassifier classifier;
        for (size_t i=0; i<points.size(); i+=2) {
            if (rust_progress_cancelled(progress)) return false;
            // Ray classification can miss ON at periodic seams and small radii.
            // Resolve the tolerance band with trimmed-curve distance first.
            BRepExtrema_DistShapeShape distance;
            distance.LoadS1(BRepBuilderAPI_MakeVertex(gp_Pnt(points[i],points[i+1],0)).Vertex());
            distance.LoadS2(boundary);
            distance.Perform(scope.Next());
            if (!distance.IsDone() || !std::isfinite(distance.Value())) return false;
            if (distance.Value() <= tolerance) { locations.push_back(2); continue; }
            classifier.Perform(face,gp_Pnt2d(points[i],points[i+1]),tolerance,false,tolerance);
            switch (classifier.State()) {
                case TopAbs_OUT: locations.push_back(0); break;
                case TopAbs_IN: locations.push_back(1); break;
                default: return false;
            }
        }
        return !rust_progress_cancelled(progress);
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "classify", 7, failure);
        return false;
    }
}

// Extrude closed profile wires into a solid via BRepPrimAPI_MakePrism.
std::unique_ptr<TopoDS_Shape> make_extrude(
    const std::vector<TopoDS_Edge>& profile_edges,
    double dx, double dy, double dz,
    const CancellationToken& progress,
    HistoryData& out_topology_history)
{
    try {
        if (profile_edges.empty() || rust_progress_cancelled(progress)) return nullptr;
        std::vector<TopoDS_Wire> profile_wires;
        BRepBuilderAPI_MakeWire wire_maker;
        bool has_edges = false;
        auto flush_wire = [&]() -> bool {
            if (!has_edges || !wire_maker.IsDone()) return false;
            profile_wires.push_back(wire_maker.Wire());
            wire_maker = BRepBuilderAPI_MakeWire();
            has_edges = false;
            return true;
        };
        for (const auto& edge : profile_edges) {
            if (edge.IsNull()) {
                if (!flush_wire()) return nullptr;
            } else {
                wire_maker.Add(edge);
                has_edges = true;
            }
        }
        if (!flush_wire() || profile_wires.empty()) return nullptr;

        BRepBuilderAPI_MakeFace face_maker(profile_wires.front());
        for (size_t index = 1; index < profile_wires.size(); ++index) {
            face_maker.Add(profile_wires[index]);
        }
        if (!face_maker.IsDone()) return nullptr;
        gp_Vec dir(dx, dy, dz);
        BRepPrimAPI_MakePrism prism(face_maker.Face(), dir);
        Handle(RustProgressIndicator) indicator = new RustProgressIndicator(progress);
        prism.Build(indicator->Start());
        if (rust_progress_cancelled(progress)) return nullptr;
        if (!prism.IsDone()) return nullptr;
        auto result = std::make_unique<TopoDS_Shape>(prism.Shape());
        const HistoryMaps result_maps(*result);
        TopoDS_Shape profile_shape = profile_wires.front();
        if (profile_wires.size() > 1) {
            BRep_Builder builder;
            TopoDS_Compound profile_compound;
            builder.MakeCompound(profile_compound);
            for (const auto& wire : profile_wires) {
                builder.Add(profile_compound, wire);
            }
            profile_shape = profile_compound;
        }
        append_builder_topology_history(prism, profile_shape, 0,
            result_maps, out_topology_history);
        const HistoryMaps profile_maps(profile_shape);
        const TopoDS_Shape first = prism.FirstShape();
        const TopoDS_Shape last = prism.LastShape();
        for (int index = 1; index <= profile_maps.edges.Extent(); ++index) {
            append_history_relation(out_topology_history, result_maps, first,
                HistoryRelation::Generated, 0, HistoryKind::Edge,
                static_cast<uint32_t>(index - 1));
            append_history_relation(out_topology_history, result_maps, last,
                HistoryRelation::Generated, 0, HistoryKind::Edge,
                static_cast<uint32_t>(index - 1));
        }
        for (HistoryKind kind : {HistoryKind::Edge, HistoryKind::Vertex}) {
            const auto& sources = profile_maps.map(kind);
            for (int index = 1; index <= sources.Extent(); ++index) {
                append_history_relation(out_topology_history, result_maps,
                    prism.FirstShape(sources(index)), HistoryRelation::Generated,
                    0, kind, static_cast<uint32_t>(index - 1));
                append_history_relation(out_topology_history, result_maps,
                    prism.LastShape(sources(index)), HistoryRelation::Generated,
                    0, kind, static_cast<uint32_t>(index - 1));
            }
        }
        finish_topology_history(result_maps, out_topology_history);
        return result;
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return nullptr;
    }
}

std::unique_ptr<TopoDS_Shape> make_revolve(
    const std::vector<TopoDS_Edge>& profile_edges,
    double ox, double oy, double oz, double dx, double dy, double dz, double angle,
    const CancellationToken& progress, HistoryData& out_topology_history)
{
    try {
        if (profile_edges.empty() || rust_progress_cancelled(progress)) return nullptr;
        std::vector<TopoDS_Wire> wires;
        BRepBuilderAPI_MakeWire wire_maker;
        bool has_edges = false;
        auto flush_wire = [&]() -> bool {
            if (!has_edges || !wire_maker.IsDone() || !wire_maker.Wire().Closed()) return false;
            wires.push_back(wire_maker.Wire());
            wire_maker = BRepBuilderAPI_MakeWire();
            has_edges = false;
            return true;
        };
        for (const auto& edge : profile_edges) {
            if (rust_progress_cancelled(progress)) return nullptr;
            if (edge.IsNull()) {
                if (!flush_wire()) return nullptr;
            } else {
                wire_maker.Add(TopoDS::Edge(BRepBuilderAPI_Copy(edge,false,false).Shape()));
                has_edges = true;
            }
        }
        if (!flush_wire()) return nullptr;
        BRepBuilderAPI_MakeFace face_maker(wires.front(),true);
        for (size_t i=1; i<wires.size(); ++i) face_maker.Add(wires[i]);
        if (!face_maker.IsDone() || !BRepCheck_Analyzer(face_maker.Face()).IsValid()) return nullptr;
        BRepTopAdaptor_FClass2d bounded(face_maker.Face(),Precision::Confusion());
        if (bounded.PerformInfinitePoint() != TopAbs_OUT) return nullptr;
        // OCCT takes a positive sweep; reversing the axis preserves signed travel.
        const double sign = angle < 0 ? -1 : 1;
        gp_Ax1 axis(gp_Pnt(ox,oy,oz),gp_Dir(dx*sign,dy*sign,dz*sign));
        BRepPrimAPI_MakeRevol revol(face_maker.Face(),axis,std::abs(angle),true);
        Handle(RustProgressIndicator) indicator = new RustProgressIndicator(progress);
        revol.Build(indicator->Start());
        if (rust_progress_cancelled(progress) || !revol.IsDone()) return nullptr;
        auto result = std::make_unique<TopoDS_Shape>(revol.Shape());
        if (!BRepCheck_Analyzer(*result).IsValid()) return nullptr;
        const HistoryMaps result_maps(*result);
        const auto profile = face_maker.Face();
        append_builder_topology_history(revol,profile,0,result_maps,out_topology_history);
        const HistoryMaps profile_maps(profile);
        for (int i=1; i<=profile_maps.edges.Extent(); ++i) {
            append_history_relation(out_topology_history,result_maps,revol.FirstShape(),
                HistoryRelation::Generated,0,HistoryKind::Edge,i-1);
            append_history_relation(out_topology_history,result_maps,revol.LastShape(),
                HistoryRelation::Generated,0,HistoryKind::Edge,i-1);
        }
        for (HistoryKind kind : {HistoryKind::Edge, HistoryKind::Vertex}) {
            const auto& sources = profile_maps.map(kind);
            for (int i=1; i<=sources.Extent(); ++i) {
                append_history_relation(out_topology_history,result_maps,
                    revol.FirstShape(sources(i)),HistoryRelation::Generated,0,kind,i-1);
                append_history_relation(out_topology_history,result_maps,
                    revol.LastShape(sources(i)),HistoryRelation::Generated,0,kind,i-1);
            }
        }
        finish_topology_history(result_maps,out_topology_history);
        return result;
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__,"revolve",7,failure);
        return nullptr;
    }
}

// Unified MakePipeShell wrapper.  Handles both single-profile sweep and
// multi-profile morphing sweep.  Profile sections in `all_edges` are
// separated by null-edge sentinels (TopoDS_Edge().IsNull() == true).
// `aux_spine_edges` is used only when orient == 3 (Auxiliary); pass an
// empty vector for other modes.
std::unique_ptr<TopoDS_Shape> make_pipe_shell(
    const std::vector<TopoDS_Edge>& all_edges,
    const std::vector<TopoDS_Edge>& spine_edges,
    uint32_t orient,
    double ux, double uy, double uz,
    const std::vector<TopoDS_Edge>& aux_spine_edges,
    const CancellationToken& progress,
    HistoryData& out_topology_history)
{
    try {
        if (all_edges.empty() || spine_edges.empty()
            || rust_progress_cancelled(progress)) return nullptr;

        TopoDS_Compound profile_compound;
        BRep_Builder profile_builder;
        profile_builder.MakeCompound(profile_compound);
        for (const auto& edge : all_edges) {
            if (!edge.IsNull()) profile_builder.Add(profile_compound, edge);
        }

        // Build the spine wire.
        BRepBuilderAPI_MakeWire spineMaker;
        for (const auto& e : spine_edges) spineMaker.Add(e);
        if (!spineMaker.IsDone()) return nullptr;
        TopoDS_Wire spine = spineMaker.Wire();

        BRepOffsetAPI_MakePipeShell shell(spine);
        shell.SetIsBuildHistory(true);

        // Configure trihedron law.
        switch (orient) {
            case 0: {
                // Fixed: lock the trihedron to the spine-start frame.
                BRepAdaptor_Curve curve(spine_edges.front());
                gp_Pnt start_pnt;
                gp_Vec start_tan;
                curve.D1(curve.FirstParameter(), start_pnt, start_tan);
                if (start_tan.Magnitude() < Precision::Confusion()) return nullptr;
                gp_Dir tdir(start_tan);
                gp_Dir xref = (std::abs(tdir.X()) < 0.9) ? gp_Dir(1, 0, 0) : gp_Dir(0, 1, 0);
                gp_Ax2 fixed_ax2(start_pnt, tdir, xref);
                shell.SetMode(fixed_ax2);
                break;
            }
            case 1: {
                // Torsion: raw Frenet.
                shell.SetMode(true);
                break;
            }
            case 2: {
                // Up(v): fix the binormal direction.
                gp_Vec up_vec(ux, uy, uz);
                if (up_vec.Magnitude() < Precision::Confusion()) return nullptr;
                shell.SetMode(gp_Dir(up_vec));
                break;
            }
            case 3: {
                // Auxiliary: build aux spine wire and use it for twist control.
                if (aux_spine_edges.empty()) return nullptr;
                BRepBuilderAPI_MakeWire auxMaker;
                for (const auto& e : aux_spine_edges) auxMaker.Add(e);
                if (!auxMaker.IsDone()) return nullptr;
                shell.SetMode(auxMaker.Wire(), false);
                break;
            }
            default: {
                shell.SetMode(true);
                break;
            }
        }

        // Split all_edges by null sentinels into profile wires and Add each.
        BRepBuilderAPI_MakeWire wire_maker;
        bool has_edges = false;
        for (const auto& e : all_edges) {
            if (e.IsNull()) {
                if (!wire_maker.IsDone()) return nullptr;
                shell.Add(wire_maker.Wire(), false, false);
                wire_maker = BRepBuilderAPI_MakeWire();
                has_edges = false;
            } else {
                wire_maker.Add(e);
                has_edges = true;
            }
        }
        // Last section (after final sentinel or single section with no sentinel).
        if (has_edges) {
            if (!wire_maker.IsDone()) return nullptr;
            shell.Add(wire_maker.Wire(), false, false);
        }

        Handle(RustProgressIndicator) indicator = new RustProgressIndicator(progress);
        shell.Build(indicator->Start());
        if (rust_progress_cancelled(progress)) return nullptr;
        if (!shell.IsDone()) return nullptr;
        if (!shell.MakeSolid()) return nullptr;
        auto result = std::make_unique<TopoDS_Shape>(shell.Shape());
        const HistoryMaps result_maps(*result);
        append_builder_topology_history(shell, profile_compound, 0,
            result_maps, out_topology_history);
        append_shared_topology_history(profile_compound, 0, result_maps,
            out_topology_history);
        append_aggregate_generated_history(profile_compound, 0, result_maps,
            HistoryKind::Face, {HistoryKind::Edge}, out_topology_history);
        append_aggregate_generated_history(profile_compound, 0, result_maps,
            HistoryKind::Edge, {HistoryKind::Edge, HistoryKind::Vertex},
            out_topology_history);
        append_aggregate_generated_history(profile_compound, 0, result_maps,
            HistoryKind::Vertex, {HistoryKind::Vertex}, out_topology_history);
        remove_related_deleted_topology(out_topology_history);
        finish_topology_history(result_maps, out_topology_history);
        return result;
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return nullptr;
    }
}

// Loft (skin) a solid through a sequence of cross-section wires.
//
// `all_edges` is a flat edge list with sections delimited by null-edge
// sentinels (TopoDS_Edge().IsNull()); ≥2 sections required. Built via
// BRepOffsetAPI_ThruSections with isSolid=true (cap open ends with planar
// faces). `ruled=false` gives B-spline / C² smoothed interpolation through
// all sections; `ruled=true` gives per-panel ruled (straight-line) surfaces
// between adjacent sections. Both pass through every section wire exactly.
std::unique_ptr<TopoDS_Shape> make_loft(
    const std::vector<TopoDS_Edge>& all_edges,
    bool ruled,
    bool closed,
    const CancellationToken& progress,
    HistoryData& out_topology_history)
{
    try {
        if (all_edges.empty() || rust_progress_cancelled(progress)) return nullptr;
        BRepOffsetAPI_ThruSections loft(
            /*isSolid=*/true,
            /*isRuled=*/ruled,
            Precision::Confusion());

        // Split all_edges by null sentinels into section wires.
        size_t wire_count = 0;
        std::vector<TopoDS_Shape> section_wires;
        BRepBuilderAPI_MakeWire wire_maker;
        bool has_edges = false;

        auto flush_wire = [&]() -> bool {
            if (!has_edges) return true;
            if (!wire_maker.IsDone()) return false;
            const TopoDS_Wire wire = wire_maker.Wire();
            loft.AddWire(wire);
            section_wires.push_back(wire);
            wire_count++;
            wire_maker = BRepBuilderAPI_MakeWire();
            has_edges = false;
            return true;
        };

        for (const auto& e : all_edges) {
            if (e.IsNull()) {
                if (!flush_wire()) return nullptr;
            } else {
                wire_maker.Add(e);
                has_edges = true;
            }
        }
        if (!flush_wire()) return nullptr;

        if (wire_count < 2) return nullptr;

        if (closed) {
            // The same TopoDS_Wire object (not a copy) makes ThruSections'
            // IsSame() check select its official v-periodic closed path.
            loft.AddWire(TopoDS::Wire(section_wires.front()));
        }

        Handle(RustProgressIndicator) indicator = new RustProgressIndicator(progress);
        loft.Build(indicator->Start());
        if (rust_progress_cancelled(progress)) return nullptr;
        if (!loft.IsDone()) return nullptr;
        auto result = std::make_unique<TopoDS_Shape>(loft.Shape());
        const HistoryMaps result_maps(*result);
        for (size_t operand = 0; operand < section_wires.size(); ++operand) {
            append_builder_topology_history(loft, section_wires[operand],
                static_cast<uint32_t>(operand), result_maps,
                out_topology_history);
            append_shared_topology_history(section_wires[operand],
                static_cast<uint32_t>(operand), result_maps,
                out_topology_history);
        }
        append_multi_operand_aggregate_generated_history(section_wires,
            result_maps, HistoryKind::Face, {HistoryKind::Edge},
            out_topology_history);
        append_multi_operand_aggregate_generated_history(section_wires,
            result_maps, HistoryKind::Edge,
            {HistoryKind::Edge, HistoryKind::Vertex}, out_topology_history);
        append_multi_operand_aggregate_generated_history(section_wires,
            result_maps, HistoryKind::Vertex, {HistoryKind::Vertex},
            out_topology_history);
        remove_related_deleted_topology(out_topology_history);
        finish_topology_history(result_maps, out_topology_history);
        return result;
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return nullptr;
    }
}

std::unique_ptr<TopoDS_Shape> builder_sew_without_faces(
    const TopoDS_Shape& shape,
    rust::Slice<const uint32_t> removed_face_indices,
    double tolerance,
    rust::Vec<uint64_t>& out_history,
    HistoryData& out_topology_history)
{
    try {
        const HistoryMaps input_maps(shape);
        BRepBuilderAPI_Sewing sewing(tolerance);
        for (int ordinal = 1; ordinal <= input_maps.faces.Extent(); ++ordinal) {
            if (std::find(removed_face_indices.begin(), removed_face_indices.end(),
                          static_cast<uint32_t>(ordinal - 1)) == removed_face_indices.end()) {
                sewing.Add(input_maps.faces(ordinal));
            }
        }
        sewing.Perform();
        const TopoDS_Shape& sewn = sewing.SewedShape();
        if (sewn.IsNull() || sewn.ShapeType() != TopAbs_SHELL
            || !BRep_Tool::IsClosed(sewn)) return nullptr;
        BRepBuilderAPI_MakeSolid maker(TopoDS::Shell(sewn));
        if (!maker.IsDone()) return nullptr;
        TopoDS_Solid solid = maker.Solid();
        if (!BRepLib::OrientClosedSolid(solid) || !BRepCheck_Analyzer(solid).IsValid()) {
            return nullptr;
        }
        const HistoryMaps result_maps(solid);
        for (HistoryKind kind : {HistoryKind::Face, HistoryKind::Edge, HistoryKind::Vertex}) {
            const auto& sources = input_maps.map(kind);
            for (int ordinal = 1; ordinal <= sources.Extent(); ++ordinal) {
                const TopoDS_Shape& source = sources(ordinal);
                TopoDS_Shape replacement = source;
                if (sewing.IsModified(source)) replacement = sewing.Modified(source);
                else if (sewing.IsModifiedSubShape(source)) replacement = sewing.ModifiedSubShape(source);
                const size_t previous_size = out_topology_history.relations.size();
                if (!replacement.IsNull()) {
                    const HistoryRelation relation = source.IsSame(replacement)
                        ? HistoryRelation::Unchanged : HistoryRelation::Modified;
                    append_history_relation(out_topology_history, result_maps, replacement,
                        relation, 0, kind, static_cast<uint32_t>(ordinal - 1));
                    if (kind == HistoryKind::Face && result_maps.faces.Contains(replacement)) {
                        out_history.push_back(reinterpret_cast<uint64_t>(replacement.TShape().get()));
                        out_history.push_back(reinterpret_cast<uint64_t>(source.TShape().get()));
                    }
                }
                if (out_topology_history.relations.size() == previous_size) {
                    out_topology_history.deleted.push_back(0);
                    out_topology_history.deleted.push_back(static_cast<uint32_t>(kind));
                    out_topology_history.deleted.push_back(static_cast<uint32_t>(ordinal - 1));
                }
            }
        }
        finish_topology_history(result_maps, out_topology_history);
        return std::make_unique<TopoDS_Shape>(solid);
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return nullptr;
    }
}

// Sew (stitch) free faces into a single closed shell and upgrade it to a
// solid. BRepBuilderAPI_Sewing merges boundary edges that coincide within
// `tolerance`; the sewn result must contain exactly one closed shell —
// gaps (open shell), leftover free faces, or multiple disconnected shells
// all return nullptr. The solid is oriented with BRepLib::OrientClosedSolid
// so the enclosed volume is positive regardless of input face orientation.
std::unique_ptr<TopoDS_Shape> make_sewn_solid(
    const std::vector<TopoDS_Face>& faces,
    double tolerance)
{
    try {
        if (faces.empty()) return nullptr;
        BRepBuilderAPI_Sewing sewing(tolerance);
        for (const auto& f : faces) sewing.Add(f);
        sewing.Perform();
        const TopoDS_Shape& sewn = sewing.SewedShape();
        if (sewn.IsNull()) return nullptr;

        // A fully sewn input comes back as a single TopAbs_SHELL; partial
        // sewing yields a compound mixing shells and free faces, in which
        // case requiring exactly one shell rejects the stray-face cases.
        std::vector<TopoDS_Shell> shells;
        if (sewn.ShapeType() == TopAbs_SHELL) {
            shells.push_back(TopoDS::Shell(sewn));
        } else {
            for (TopExp_Explorer ex(sewn, TopAbs_SHELL); ex.More(); ex.Next()) {
                shells.push_back(TopoDS::Shell(ex.Current()));
            }
        }
        if (shells.size() != 1) return nullptr;
        if (!BRep_Tool::IsClosed(shells.front())) return nullptr;

        BRepBuilderAPI_MakeSolid solid_maker(shells.front());
        if (!solid_maker.IsDone()) return nullptr;
        TopoDS_Solid solid = solid_maker.Solid();
        BRepLib::OrientClosedSolid(solid);
        return std::make_unique<TopoDS_Shape>(solid);
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return nullptr;
    }
}

// Offset every face of `shape` by signed `offset` (positive = outward)
// using BRepOffsetAPI_MakeOffsetShape. Same PerformByJoin configuration as
// builder_thick_solid's sealed-shell fallback (Skin mode, Arc join). The
// result of offsetting a solid is normally a SOLID, but OCCT occasionally
// returns the bare offset SHELL or a one-element compound — both are
// upgraded here so the Rust side always receives a TopAbs_SOLID. Returns
// nullptr when OCCT rejects the offset (self-intersecting offset surfaces:
// |offset| ≥ half the local wall thickness, or a concave slot narrower
// than 2*offset pinching shut).
std::unique_ptr<TopoDS_Shape> make_offset_shape(
    const TopoDS_Shape& shape,
    double offset,
    double tolerance)
{
    try {
        BRepOffsetAPI_MakeOffsetShape offsetter;
        offsetter.PerformByJoin(
            shape, offset, tolerance,
            /*mode=*/ BRepOffset_Skin,
            /*intersection=*/ false,
            /*selfInter=*/ false,
            /*join=*/ GeomAbs_Arc);
        if (!offsetter.IsDone()) return nullptr;
        TopoDS_Shape result = offsetter.Shape();
        if (result.IsNull()) return nullptr;

        if (result.ShapeType() == TopAbs_COMPOUND) {
            // Unwrap a one-solid (or one-shell) compound.
            TopExp_Explorer solid_ex(result, TopAbs_SOLID);
            if (solid_ex.More()) {
                result = solid_ex.Current();
                solid_ex.Next();
                if (solid_ex.More()) return nullptr;
            } else {
                TopExp_Explorer shell_ex(result, TopAbs_SHELL);
                if (!shell_ex.More()) return nullptr;
                result = shell_ex.Current();
                shell_ex.Next();
                if (shell_ex.More()) return nullptr;
            }
        }

        if (result.ShapeType() == TopAbs_SOLID) {
            return std::make_unique<TopoDS_Shape>(result);
        }
        if (result.ShapeType() == TopAbs_SHELL && BRep_Tool::IsClosed(result)) {
            BRepBuilderAPI_MakeSolid solid_maker(TopoDS::Shell(result));
            if (!solid_maker.IsDone()) return nullptr;
            TopoDS_Solid solid = solid_maker.Solid();
            BRepLib::OrientClosedSolid(solid);
            return std::make_unique<TopoDS_Shape>(solid);
        }
        return nullptr;
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return nullptr;
    }
}

std::unique_ptr<TopoDS_Shape> make_bspline_solid(
    rust::Slice<const double> coords,
    uint32_t nu, uint32_t nv,
    bool u_periodic)
{
    try {
        if (coords.size() != static_cast<size_t>(nu) * nv * 3) return nullptr;
        if (nu < 2 || nv < 3) return nullptr;

        // Tensor-product truly-periodic interpolation (#120).
        //
        // Naive approach (Interpolate over augmented grid → SetUPeriodic) only
        // delivers C^0 at the seam: the non-periodic interpolator picks
        // independent boundary derivatives at u_min vs u_max, and SetUPeriodic
        // just relabels topology without fixing the derivative mismatch.
        //
        // Instead we apply GeomAPI_Interpolate (which honors a true periodic
        // boundary by solving a circulant linear system) once per V column,
        // then once per U row of the resulting intermediate poles. The final
        // poles array feeds Geom_BSplineSurface(...) directly with the
        // UPeriodic / VPeriodic flags, yielding C^(degree-1) continuity at
        // both seams.
        using HPntArray  = NCollection_HArray1<gp_Pnt>;
        using HRealArray = NCollection_HArray1<double>;
        const double tol = Precision::Confusion();

        // Build uniform parameter arrays so that every column / row uses the
        // SAME parametrization. With chord-length (the Interpolate default),
        // columns of different total length get different knot vectors and
        // the resulting tensor surface has parameter mismatches at +X axis
        // (visible as boolean-intersect degeneracies). Uniform params avoid
        // this since the input grid samples φ / θ at constant fractional
        // intervals along each direction.
        const int u_param_count = static_cast<int>(u_periodic ? nu + 1 : nu);
        Handle(HRealArray) u_params = new HRealArray(1, u_param_count);
        for (int k = 0; k < u_param_count; ++k) {
            u_params->SetValue(k + 1, static_cast<double>(k) / static_cast<double>(nu));
        }
        Handle(HRealArray) v_params = new HRealArray(1, static_cast<int>(nv + 1));
        for (uint32_t k = 0; k <= nv; ++k) {
            v_params->SetValue(static_cast<int>(k) + 1,
                               static_cast<double>(k) / static_cast<double>(nv));
        }

        // Stage 1: per-V-column interpolation along U with uniform params.
        std::vector<Handle(Geom_BSplineCurve)> u_curves;
        u_curves.reserve(nv);
        for (uint32_t j = 0; j < nv; ++j) {
            Handle(HPntArray) col = new HPntArray(1, static_cast<int>(nu));
            for (uint32_t i = 0; i < nu; ++i) {
                const size_t idx = (static_cast<size_t>(i) * nv + j) * 3;
                col->SetValue(static_cast<int>(i) + 1,
                              gp_Pnt(coords[idx], coords[idx + 1], coords[idx + 2]));
            }
            GeomAPI_Interpolate interp(col, u_params, u_periodic, tol);
            interp.Perform();
            if (!interp.IsDone()) return nullptr;
            u_curves.push_back(interp.Curve());
        }

        // Capture U knot vector / multiplicities / degree from any column;
        // GeomAPI_Interpolate uses the same chord-length parametrization for
        // all columns since the V coordinate is uniform per column.
        const int u_degree = u_curves[0]->Degree();
        const int u_npoles = u_curves[0]->NbPoles();
        const NCollection_Array1<double>& u_knots = u_curves[0]->Knots();
        const NCollection_Array1<int>&    u_mults = u_curves[0]->Multiplicities();

        NCollection_Array2<gp_Pnt> intermediate(1, u_npoles, 1, static_cast<int>(nv));
        for (uint32_t j = 0; j < nv; ++j) {
            for (int i = 1; i <= u_npoles; ++i) {
                intermediate.SetValue(i, static_cast<int>(j) + 1, u_curves[j]->Pole(i));
            }
        }

        // Stage 2: per-U-row interpolation along V (V is always periodic).
        std::vector<Handle(Geom_BSplineCurve)> v_curves;
        v_curves.reserve(u_npoles);
        for (int i = 1; i <= u_npoles; ++i) {
            Handle(HPntArray) row = new HPntArray(1, static_cast<int>(nv));
            for (uint32_t j = 0; j < nv; ++j) {
                row->SetValue(static_cast<int>(j) + 1, intermediate(i, static_cast<int>(j) + 1));
            }
            GeomAPI_Interpolate interp(row, v_params, /*periodic=*/true, tol);
            interp.Perform();
            if (!interp.IsDone()) return nullptr;
            v_curves.push_back(interp.Curve());
        }

        const int v_degree = v_curves[0]->Degree();
        const int v_npoles = v_curves[0]->NbPoles();
        const NCollection_Array1<double>& v_knots = v_curves[0]->Knots();
        const NCollection_Array1<int>&    v_mults = v_curves[0]->Multiplicities();

        // Stage 3: assemble final M_pole × N_pole pole grid and build the
        // surface with explicit periodic flags.
        NCollection_Array2<gp_Pnt> final_poles(1, u_npoles, 1, v_npoles);
        for (int i = 1; i <= u_npoles; ++i) {
            for (int j = 1; j <= v_npoles; ++j) {
                final_poles.SetValue(i, j, v_curves[i - 1]->Pole(j));
            }
        }

        Handle(Geom_BSplineSurface) surface = new Geom_BSplineSurface(
            final_poles,
            u_knots, v_knots,
            u_mults, v_mults,
            u_degree, v_degree,
            /*UPeriodic=*/u_periodic,
            /*VPeriodic=*/true);
        if (surface.IsNull()) return nullptr;

        // Side face spans the full parametric domain.
        double u1, u2, v1, v2;
        surface->Bounds(u1, u2, v1, v2);
        BRepBuilderAPI_MakeFace face_maker(surface, Precision::Confusion());
        if (!face_maker.IsDone()) return nullptr;
        TopoDS_Face side_face = face_maker.Face();

        BRepBuilderAPI_Sewing sewing(1.0e-3);
        sewing.Add(side_face);

        // For non-periodic U, cap the two U-boundary loops with planar faces.
        // For periodic U the surface is already closed into a torus — no caps.
        if (!u_periodic) {
            auto make_cap = [&](double u_at) -> TopoDS_Face {
                Handle(Geom_Curve) iso = surface->UIso(u_at);
                if (iso.IsNull()) return TopoDS_Face();
                BRepBuilderAPI_MakeEdge em(iso, v1, v2);
                if (!em.IsDone()) return TopoDS_Face();
                BRepBuilderAPI_MakeWire wm(em.Edge());
                if (!wm.IsDone()) return TopoDS_Face();
                BRepBuilderAPI_MakeFace mf(wm.Wire(), true);
                return mf.IsDone() ? mf.Face() : TopoDS_Face();
            };
            TopoDS_Face cap1 = make_cap(u1);
            TopoDS_Face cap2 = make_cap(u2);
            if (cap1.IsNull() || cap2.IsNull()) return nullptr;
            sewing.Add(cap1);
            sewing.Add(cap2);
        }

        sewing.Perform();
        TopoDS_Shape sewn = sewing.SewedShape();
        if (sewn.IsNull()) return nullptr;

        TopoDS_Shell shell;
        if (sewn.ShapeType() == TopAbs_SHELL) {
            shell = TopoDS::Shell(sewn);
        } else if (sewn.ShapeType() == TopAbs_SOLID) {
            return std::make_unique<TopoDS_Shape>(sewn);
        } else if (sewn.ShapeType() == TopAbs_FACE && u_periodic) {
            // Full torus: single closed face → wrap manually.
            BRep_Builder bb;
            bb.MakeShell(shell);
            bb.Add(shell, TopoDS::Face(sewn));
            shell.Closed(true);
        } else {
            TopExp_Explorer exp(sewn, TopAbs_SHELL);
            if (exp.More()) {
                shell = TopoDS::Shell(exp.Current());
            } else {
                return nullptr;
            }
        }

        BRepBuilderAPI_MakeSolid solid_maker(shell);
        if (!solid_maker.IsDone()) return nullptr;
        TopoDS_Solid solid = solid_maker.Solid();

        // Ensure outward-facing orientation.
        BRepClass3d_SolidClassifier classifier(
            solid, gp_Pnt(0, 0, 0), Precision::Confusion());
        if (classifier.State() == TopAbs_IN) {
            solid.Reverse();
        }

        return std::make_unique<TopoDS_Shape>(solid);
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return nullptr;
    }
}

// ==================== Streambuf bridges (ffi.cpp-internal) ====================

// std::streambuf subclass that reads from a Rust `dyn Read` via FFI callback
class RustReadStreambuf : public std::streambuf {
public:
    explicit RustReadStreambuf(RustReader& reader) : reader_(reader) {}

protected:
    int_type underflow() override {
        rust::Slice<uint8_t> slice(
            reinterpret_cast<uint8_t*>(buf_), sizeof(buf_));
        size_t n = rust_reader_read(reader_, slice);
        if (n == 0) return traits_type::eof();
        setg(buf_, buf_, buf_ + n);
        return traits_type::to_int_type(*gptr());
    }

    // Override to keep the vtable slot resolved within this TU instead of
    // referencing `std::basic_streambuf<char>::seekpos`, whose mangling depends
    // on `std::fpos<mbstate_t>` — and `mbstate_t` is a typedef to the internal
    // `_Mbstatet` on gcc 15 mingw but a different name on gcc 14, so the
    // external symbol fails to resolve when the prebuilt ships gcc 14
    // libstdc++.a but downstream links with gcc 15.
    pos_type seekpos(pos_type, std::ios_base::openmode = std::ios_base::in | std::ios_base::out) override {
        return pos_type(off_type(-1));
    }

private:
    RustReader& reader_;
    char buf_[8192];
};

// std::streambuf subclass that writes to a Rust `dyn Write` via FFI callback
class RustWriteStreambuf : public std::streambuf {
public:
    explicit RustWriteStreambuf(RustWriter& writer) : writer_(writer) {}

    ~RustWriteStreambuf() override {
        sync();
    }

protected:
    int_type overflow(int_type ch) override {
        if (ch != traits_type::eof()) {
            buf_[pos_++] = static_cast<char>(ch);
            if (pos_ >= sizeof(buf_)) {
                if (!flush_buf()) return traits_type::eof();
            }
        }
        return ch;
    }

    std::streamsize xsputn(const char* s, std::streamsize count) override {
        std::streamsize written = 0;
        while (written < count) {
            std::streamsize space = sizeof(buf_) - pos_;
            std::streamsize chunk = std::min(count - written, space);
            std::memcpy(buf_ + pos_, s + written, chunk);
            pos_ += static_cast<size_t>(chunk);
            written += chunk;
            if (pos_ >= sizeof(buf_)) {
                if (!flush_buf()) return written;
            }
        }
        return written;
    }

    int sync() override {
        return flush_buf() ? 0 : -1;
    }

    // See RustReadStreambuf::seekpos — same gcc 14/15 `_Mbstatet` mangling fix.
    pos_type seekpos(pos_type, std::ios_base::openmode = std::ios_base::in | std::ios_base::out) override {
        return pos_type(off_type(-1));
    }

private:
    bool flush_buf() {
        if (pos_ == 0) return true;
        rust::Slice<const uint8_t> slice(
            reinterpret_cast<const uint8_t*>(buf_), pos_);
        size_t n = rust_writer_write(writer_, slice);
        if (n < pos_) return false;
        pos_ = 0;
        return true;
    }

    RustWriter& writer_;
    char buf_[8192];
    size_t pos_ = 0;
};



std::unique_ptr<TopoDS_Shape> read_brep_stream(
    rust::Slice<const uint8_t> data, size_t& out_consumed)
{
    // istringstream because BinTools::Read seeks backwards to shared sub-shapes.
    std::istringstream iss(
        std::string(reinterpret_cast<const char*>(data.data()), data.size()));

    auto shape = std::make_unique<TopoDS_Shape>();
    try {
        BinTools::Read(*shape, iss);
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return nullptr;  // out_consumed deliberately untouched
    }
    if (shape->IsNull()) {
        return nullptr;  // ditto
    }

    // clear() is load-bearing: a payload read to its last byte leaves eofbit set, and
    // tellg()'s sentry then turns that into failbit and returns -1.
    iss.clear();
    out_consumed = static_cast<size_t>(iss.tellg());
    return shape;
}

bool write_brep_stream(const TopoDS_Shape& shape, RustWriter& writer) {
    RustWriteStreambuf sbuf(writer);
    std::ostream os(&sbuf);
    try {
        BinTools::Write(shape, os);
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return false;
    }
    return os.good();
}

#ifndef FEATURE_COLOR
// Plain STEP I/O — used only when FEATURE_COLOR is not defined.
std::unique_ptr<TopoDS_Shape> read_step_stream(RustReader& reader) {
    RustReadStreambuf sbuf(reader);
    std::istream is(&sbuf);

    STEPControl_Reader step_reader;
    IFSelect_ReturnStatus status = step_reader.ReadStream("stream", is);

    if (status != IFSelect_RetDone) {
        return nullptr;
    }

    step_reader.TransferRoots(Message_ProgressRange());
    return std::make_unique<TopoDS_Shape>(
        try_sew_orphan_faces(step_reader.OneShape(), nullptr));
}

bool write_step_stream(const TopoDS_Shape& shape, RustWriter& writer) {
    RustWriteStreambuf sbuf(writer);
    std::ostream os(&sbuf);
    STEPControl_Writer step_writer;
    if (step_writer.Transfer(shape, STEPControl_AsIs) != IFSelect_RetDone) {
        return false;
    }
    return step_writer.WriteStream(os) == IFSelect_RetDone;
}
#endif // !FEATURE_COLOR

std::unique_ptr<std::vector<TopoDS_Shape>> builder_split_body(
    const TopoDS_Shape& solid, rust::Slice<const double> planes,
    const std::vector<TopoDS_Face>& faces, const std::vector<TopoDS_Edge>& edges,
    rust::Slice<const uint32_t> group_sizes, rust::Slice<const double> directions,
    const CancellationToken& progress, rust::Vec<HistoryData>& histories)
{
    try {
        if (planes.size() % 6 || directions.size() != group_sizes.size() * 3
            || edges.size() > 20000 || planes.size() / 6 + faces.size() + group_sizes.size() > 64)
            return nullptr;
        Bnd_Box bounds;
        BRepBndLib::AddOptimal(solid, bounds, false, false);
        if (bounds.IsVoid()) return nullptr;
        const gp_Pnt low = bounds.CornerMin(), high = bounds.CornerMax();
        const gp_Pnt center((low.X()+high.X())/2, (low.Y()+high.Y())/2, (low.Z()+high.Z())/2);
        const double span = low.Distance(high) + 1.0;
        BOPAlgo_Splitter splitter;
        splitter.AddArgument(solid);
        splitter.SetNonDestructive(true);
        auto add_plane = [&](const gp_Pln& plane) {
            const double reach = span + center.Distance(plane.Location());
            BRepBuilderAPI_MakeFace face(plane, -reach, reach, -reach, reach);
            if (!face.IsDone()) throw Standard_Failure("Cannot construct the splitting plane");
            splitter.AddTool(face.Face());
        };
        for (size_t i=0; i<planes.size(); i+=6) {
            for (size_t j=i; j<i+6; ++j) if (!std::isfinite(planes[j])) return nullptr;
            add_plane(gp_Pln(gp_Pnt(planes[i],planes[i+1],planes[i+2]), gp_Dir(planes[i+3],planes[i+4],planes[i+5])));
        }
        for (const auto& face : faces) {
            BRepAdaptor_Surface adaptor(face);
            if (adaptor.GetType() == GeomAbs_Plane) {
                add_plane(adaptor.Plane());
            } else {
                auto surface = unwrapped_brep_surface(BRep_Tool::Surface(face));
                BRepBuilderAPI_MakeFace extended(surface, Precision::Confusion());
                if (!extended.IsDone()) throw Standard_Failure("Cannot extend this face as a splitting surface");
                splitter.AddTool(extended.Face());
            }
        }
        size_t cursor = 0;
        for (size_t group=0; group<group_sizes.size(); ++group) {
            const size_t count = group_sizes[group];
            if (!count || cursor+count>edges.size()) return nullptr;
            gp_Vec direction(directions[group*3],directions[group*3+1],directions[group*3+2]);
            if (direction.SquareMagnitude() == 0.0) {
                TopoDS_Compound wire;
                BRep_Builder builder; builder.MakeCompound(wire);
                for (size_t j=0;j<count;++j) builder.Add(wire,edges[cursor+j]);
                std::vector<gp_Pnt> samples;
                for (size_t j=0;j<count;++j) {
                    BRepAdaptor_Curve curve(edges[cursor+j]);
                    for (int k=0;k<5;++k)
                        samples.push_back(curve.Value(curve.FirstParameter()+(curve.LastParameter()-curve.FirstParameter())*k/4));
                }
                const auto origin = samples.front();
                gp_Vec baseline;
                bool defines_plane = false;
                for (const auto& point : samples) {
                    gp_Vec delta(origin, point);
                    if (baseline.Magnitude() < 1.0e-6 && delta.Magnitude() >= 1.0e-6)
                        baseline = delta.Normalized();
                    if (baseline.Magnitude() > 0 && baseline.Crossed(delta).Magnitude() > 1.0e-6)
                        defines_plane = true;
                }
                if (!defines_plane) throw Standard_Failure("Select non-collinear coplanar edges to define a plane");
                BRepBuilderAPI_FindPlane fit(wire,1.0e-6);
                if (!fit.Found()) throw Standard_Failure("Select coplanar edges that define a plane");
                add_plane(fit.Plane()->Pln());
            } else {
                direction.Normalize();
                for (size_t j=0;j<count;++j) {
                    Bnd_Box edge_bounds; BRepBndLib::AddOptimal(edges[cursor+j],edge_bounds,false,false);
                    const double reach = span + center.Distance(edge_bounds.CornerMin()) + center.Distance(edge_bounds.CornerMax());
                    gp_Trsf shift; shift.SetTranslation(direction * -reach);
                    TopoDS_Shape moved = edges[cursor+j].Moved(TopLoc_Location(shift));
                    BRepPrimAPI_MakePrism prism(moved,direction*(2*reach),false,true);
                    if (!prism.IsDone()) throw Standard_Failure("Cannot extend these sketch curves");
                    splitter.AddTool(prism.Shape());
                }
            }
            cursor += count;
            if (rust_progress_cancelled(progress)) return nullptr;
        }
        if (cursor != edges.size() || (planes.empty() && faces.empty() && edges.empty())) return nullptr;
        Handle(RustProgressIndicator) indicator = new RustProgressIndicator(progress);
        splitter.Perform(indicator->Start());
        if (splitter.HasErrors() || rust_progress_cancelled(progress)) return nullptr;
        auto result = std::make_unique<std::vector<TopoDS_Shape>>();
        for (TopExp_Explorer ex(splitter.Shape(),TopAbs_SOLID);ex.More();ex.Next()) {
            const auto piece = ex.Current();
            if (!BRepCheck_Analyzer(piece).IsValid()) return nullptr;
            result->push_back(piece);
            HistoryData history;
            append_builder_topology_history(splitter,solid,0,HistoryMaps(piece),history);
            finish_topology_history(HistoryMaps(piece),history);
            histories.push_back(std::move(history));
            if (result->size()>256) return nullptr;
        }
        return result;
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__,"split body",7,failure);
        return nullptr;
    }
}

std::unique_ptr<TopoDS_Shape> builder_wrap_emboss(
    const TopoDS_Shape& solid, const TopoDS_Face& target,
    const std::vector<TopoDS_Edge>& edges, rust::Slice<const uint32_t> wire_sizes,
    rust::Slice<const uint32_t> region_sizes, double depth, double rotation,
    double center_x, double center_y, const CancellationToken& progress, HistoryData& history)
{
    try {
        if (edges.empty() || edges.size() > 20000 || !std::isfinite(depth)
            || !std::isfinite(rotation) || !std::isfinite(center_x) || !std::isfinite(center_y)) {
            record_input_failure(__func__, "Select bounded profiles and finite dimensions"); return nullptr;
        }
        Handle(Geom_Surface) surface = unwrapped_brep_surface(BRep_Tool::Surface(target));
        BRepAdaptor_Surface adaptor(target);
        const bool cylinder = adaptor.GetType() == GeomAbs_Cylinder;
        if (!cylinder && adaptor.GetType() != GeomAbs_Cone) {
            record_input_failure(__func__, "Choose a cylindrical or conical face"); return nullptr;
        }
        double umin, umax, vmin, vmax;
        BRepTools::UVBounds(target, umin, umax, vmin, vmax);
        const double u0 = (umin + umax) * 0.5, v0 = (vmin + vmax) * 0.5;
        const double radius = cylinder ? adaptor.Cylinder().Radius() : adaptor.Cone().RefRadius();
        const double k = cylinder ? 0.0 : std::sin(adaptor.Cone().SemiAngle());
        const double r0 = radius + v0 * k;
        if (r0 <= 1.0e-6 || (!cylinder && std::abs(k) < 1.0e-8)) return nullptr;
        if (std::abs(depth) >= r0 && depth < 0) {
            record_input_failure(__func__, "Engraving depth reaches the surface axis"); return nullptr;
        }
        Bnd_Box bounds;
        for (const auto& edge : edges) BRepBndLib::AddOptimal(edge, bounds, false, false);
        double xmin, ymin, zmin, xmax, ymax, zmax;
        bounds.Get(xmin, ymin, zmin, xmax, ymax, zmax);
        if (std::abs(zmin) > 1.0e-5 || std::abs(zmax) > 1.0e-5) return nullptr;
        const double cx = (xmin+xmax)*0.5, cy = (ymin+ymax)*0.5;
        const double cs = std::cos(rotation), sn = std::sin(rotation);
        double uv_shift=0.0;
        auto mapped = [&](const gp_Pnt& p) {
            const double x = cs*(p.X()-cx)-sn*(p.Y()-cy)+center_x;
            const double y = sn*(p.X()-cx)+cs*(p.Y()-cy)+center_y;
            if (cylinder) return gp_Pnt2d(u0+x/radius-uv_shift, v0+y);
            const double rho0 = r0/k, sign = k > 0 ? 1.0 : -1.0;
            const double rho = sign*std::hypot(x, y+rho0);
            const double theta = std::atan2(x*sign, (y+rho0)*sign);
            if (std::abs(theta/k) >= M_PI || std::abs(rho*k) < 1.0e-5)
                throw Standard_Failure("Profile crosses the cone apex or overlaps a full turn");
            return gp_Pnt2d(u0+theta/k-uv_shift, rho-radius/k);
        };
        // Move the surface seam away from the profile center without moving its geometry.
        uv_shift=mapped(gp_Pnt(cx,cy,0)).X()-M_PI;
        surface=Handle(Geom_Surface)::DownCast(surface->Copy());
        surface->Rotate(cylinder ? adaptor.Cylinder().Axis() : adaptor.Cone().Axis(),uv_shift);
        std::vector<TopoDS_Wire> wires;
        std::vector<bool> holes;
        for(uint32_t count:region_sizes) for(uint32_t j=0;j<count;++j) holes.push_back(j!=0);
        if(holes.size()!=wire_sizes.size()) return nullptr;
        double min_wrap_u=std::numeric_limits<double>::infinity(), max_wrap_u=-min_wrap_u;
        size_t cursor=0;
        for (uint32_t count : wire_sizes) {
            if (count == 0 || cursor+count > edges.size()) return nullptr;
            BRepBuilderAPI_MakeWire wire;
            double oriented_area=0.0;
            for (uint32_t j=0; j<count; ++j) {
                if (rust_progress_cancelled(progress)) return nullptr;
                const auto& edge = edges[cursor++];
                double first, last;
                Handle(Geom_Curve) curve = BRep_Tool::Curve(edge, first, last);
                if (curve.IsNull()) return nullptr;
                for(int i=0;i<64;++i) {
                    const auto a=curve->Value(first+(last-first)*i/64), b=curve->Value(first+(last-first)*(i+1)/64);
                    oriented_area+=(a.X()*b.Y()-b.X()*a.Y())*(edge.Orientation()==TopAbs_REVERSED?-1:1);
                }
                Handle(Geom2d_BSplineCurve) uv;
                if (cylinder) {
                    Handle(Geom_Curve) trimmed = new Geom_TrimmedCurve(curve, first, last);
                    uv = Geom2dConvert::CurveToBSplineCurve(GeomAPI::To2d(trimmed, gp_Pln(gp::XOY())));
                    for (int pole=1; pole<=uv->NbPoles(); ++pole) {
                        const auto p = uv->Pole(pole);
                        uv->SetPole(pole, mapped(gp_Pnt(p.X(),p.Y(),0)));
                    }
                } else {
                    // Fit in parameter space, then verify the resulting surface curve in millimetres.
                    for (int n=16; n<=4096; n*=2) {
                        using HPoints2d = NCollection_HArray1<gp_Pnt2d>;
                        using HParameters = NCollection_HArray1<double>;
                        Handle(HPoints2d) points = new HPoints2d(1,n+1);
                        Handle(HParameters) params = new HParameters(1,n+1);
                        for (int i=0;i<=n;++i) {
                            const double t=first+(last-first)*i/n;
                            points->SetValue(i+1,mapped(curve->Value(t))); params->SetValue(i+1,t);
                        }
                        Geom2dAPI_Interpolate fit(points,params,false,1.0e-10); fit.Perform();
                        if (!fit.IsDone()) return nullptr;
                        uv=fit.Curve();
                        bool accurate=true;
                        for(int i=0;i<n*4;++i) {
                            const double t=first+(last-first)*(i+0.5)/(n*4);
                            const auto exact=mapped(curve->Value(t)), approx=uv->Value(t);
                            if(surface->Value(exact.X(),exact.Y()).Distance(surface->Value(approx.X(),approx.Y()))>1.0e-6) { accurate=false; break; }
                        }
                        if(accurate) break;
                        uv.Nullify();
                        if(rust_progress_cancelled(progress)) return nullptr;
                    }
                    if(uv.IsNull()) { record_input_failure(__func__, "Surface mapping did not meet its tolerance"); return nullptr; }
                }
                for(int i=0;i<=64;++i) {
                    const auto p=uv->Value(uv->FirstParameter()+(uv->LastParameter()-uv->FirstParameter())*i/64);
                    min_wrap_u=std::min(min_wrap_u,p.X()); max_wrap_u=std::max(max_wrap_u,p.X());
                    if(p.Y()<vmin-1.0e-6 || p.Y()>vmax+1.0e-6 || max_wrap_u-min_wrap_u>=2*M_PI-1.0e-6) {
                        record_input_failure(__func__, "Profiles extend past the face or overlap a full turn; adjust Center or profile size"); return nullptr;
                    }
                }
                BRepBuilderAPI_MakeEdge maker(uv,surface);
                if(!maker.IsDone()) return nullptr;
                TopoDS_Edge wrapped=maker.Edge();
                BRepLib::BuildCurve3d(wrapped,1.0e-6);
                if(edge.Orientation()==TopAbs_REVERSED) wrapped.Reverse();
                wire.Add(wrapped);
            }
            if(!wire.IsDone() || !wire.Wire().Closed()) { record_input_failure(__func__, "Profiles must form closed loops"); return nullptr; }
            TopoDS_Wire boundary=wire.Wire();
            if((oriented_area>0.0)==holes[wires.size()]) boundary.Reverse();
            wires.push_back(boundary);
        }
        if(cursor!=edges.size()) return nullptr;
        BRep_Builder builder;
        TopoDS_Compound tools; builder.MakeCompound(tools);
        BOPAlgo_Splitter split; split.AddArgument(solid); split.SetNonDestructive(true);
        cursor=0;
        for(uint32_t count:region_sizes) {
            if(count==0 || cursor+count>wires.size()) return nullptr;
            BRepBuilderAPI_MakeFace face(surface,wires[cursor],true);
            for(uint32_t j=1;j<count;++j) face.Add(wires[cursor+j]);
            if(!face.IsDone()) return nullptr;
            TopoDS_Face patch=face.Face();
            BRepLib::SameParameter(patch,1.0e-6,true);
            if(!BRepCheck_Analyzer(patch).IsValid()) return nullptr;
            GProp_GProps full, clipped;
            BRepGProp::SurfaceProperties(patch,full,1.0e-9);
            BRepAlgoAPI_Common common;
            NCollection_List<TopoDS_Shape> patch_args, target_args;
            patch_args.Append(patch); target_args.Append(target);
            common.SetArguments(patch_args); common.SetTools(target_args);
            common.SetNonDestructive(true);
            Handle(RustProgressIndicator) common_progress = new RustProgressIndicator(progress);
            common.Build(common_progress->Start());
            if(!common.IsDone()) return nullptr;
            BRepGProp::SurfaceProperties(common.Shape(),clipped,1.0e-9);
            if(std::abs(full.Mass()-clipped.Mass())>std::max(1.0e-6,full.Mass()*1.0e-6)) {
                record_input_failure(__func__, "Profiles must fit entirely inside the selected face"); return nullptr;
            }
            if(depth==0) {
                for(uint32_t j=0;j<count;++j) split.AddTool(wires[cursor+j]);
            } else {
                if(target.Orientation()==TopAbs_REVERSED) patch.Reverse();
                BRepOffset_MakeOffset thick;
                thick.Initialize(patch,depth,1.0e-6,BRepOffset_Skin,false,false,GeomAbs_Intersection,true);
                Handle(RustProgressIndicator) offset_progress = new RustProgressIndicator(progress);
                thick.MakeOffsetShape(offset_progress->Start());
                if(!thick.IsDone() || !BRepCheck_Analyzer(thick.Shape()).IsValid()) return nullptr;
                builder.Add(tools,thick.Shape());
            }
            cursor+=count;
            if(rust_progress_cancelled(progress)) return nullptr;
        }
        if(cursor!=wires.size()) return nullptr;
        TopoDS_Shape result;
        if(depth==0) {
            Handle(RustProgressIndicator) split_progress = new RustProgressIndicator(progress);
            split.Perform(split_progress->Start()); if(split.HasErrors()) return nullptr;
            result=split.Shape();
            append_builder_topology_history(split,solid,0,HistoryMaps(result),history);
        } else {
            BRepAlgoAPI_BooleanOperation operation;
            NCollection_List<TopoDS_Shape> args, operands; args.Append(solid); operands.Append(tools);
            operation.SetArguments(args); operation.SetTools(operands);
            operation.SetOperation(depth>0 ? BOPAlgo_FUSE : BOPAlgo_CUT);
            operation.SetNonDestructive(true);
            Handle(RustProgressIndicator) indicator = new RustProgressIndicator(progress);
            operation.Build(indicator->Start());
            if(!operation.IsDone()) return nullptr;
            result=operation.Shape();
            append_builder_topology_history(operation,solid,0,HistoryMaps(result),history);
        }
        if(rust_progress_cancelled(progress) || !BRepCheck_Analyzer(result).IsValid()) return nullptr;
        TopExp_Explorer solids(result,TopAbs_SOLID);
        if(!solids.More()) return nullptr;
        TopoDS_Shape single=solids.Current(); solids.Next();
        if(solids.More()) { record_input_failure(__func__, "Emboss must remain connected to one solid"); return nullptr; }
        finish_topology_history(HistoryMaps(single),history);
        return std::make_unique<TopoDS_Shape>(single);
    } catch(const Standard_Failure& failure) {
        record_standard_failure(__func__,"wrap",7,failure); return nullptr;
    }
}

} // namespace cadrum

#ifdef FEATURE_COLOR

#include <XCAFDoc_DocumentTool.hxx>
#include <XCAFDoc_ShapeTool.hxx>
#include <XCAFDoc_ColorTool.hxx>
#include <STEPCAFControl_Reader.hxx>
#include <STEPCAFControl_Writer.hxx>
#include <TDocStd_Document.hxx>
#include <TDF_ChildIterator.hxx>
#include <NCollection_Sequence.hxx>
#include <TDF_Label.hxx>
#include <Quantity_Color.hxx>

namespace cadrum {

// Face and solid keys share one map: a TShape* is unique across shape types. Solid
// color is NOT expanded onto faces — that would turn one STYLED_ITEM into N on write.
static void collect_colors(
    const Handle(TDocStd_Document)& doc,
    const Handle(XCAFDoc_ColorTool)& colorTool,
    std::unordered_map<uint64_t, std::array<float, 3>>& colorMap)
{
    for (TDF_ChildIterator it(doc->Main(), true); it.More(); it.Next()) {
        const TDF_Label& label = it.Value();
        if (!XCAFDoc_ShapeTool::IsShape(label)) continue;

        TopoDS_Shape s = XCAFDoc_ShapeTool::GetShape(label);
        if (s.IsNull()) continue;

        // Surface style first, generic style as the fallback.
        Quantity_Color color;
        if (colorTool->GetColor(label, XCAFDoc_ColorSurf, color) ||
            colorTool->GetColor(label, XCAFDoc_ColorGen, color)) {
            if (s.ShapeType() == TopAbs_FACE) {
                colorMap[reinterpret_cast<uint64_t>(s.TShape().get())] = {
                    (float)color.Red(), (float)color.Green(), (float)color.Blue()};
            } else {
                // A label's shape may be a COMPOUND/COMPSOLID — an assembly, or a
                // product of several bodies — which is a level STEP often styles.
                for (TopExp_Explorer ex(s, TopAbs_SOLID); ex.More(); ex.Next()) {
                    colorMap[reinterpret_cast<uint64_t>(ex.Current().TShape().get())] = {
                        (float)color.Red(), (float)color.Green(), (float)color.Blue()};
                }
            }
        }
    }
}

std::unique_ptr<TopoDS_Shape> read_step_color_stream(
    RustReader&          reader,
    rust::Vec<uint64_t>& out_ids,
    rust::Vec<float>&    out_rgb)
{
    try {
        // Create XDE document directly — avoids XCAFApp_Application which
        // pulls in visualization libs (TKXCAFPrs/TKTPrsStd) built with
        // BUILD_MODULE_Visualization=OFF.  Handle<> ref-counts ownership.
        Handle(TDocStd_Document) doc = new TDocStd_Document("XmlXCAF");

        STEPCAFControl_Reader cafreader;
        cafreader.SetColorMode(true);

        RustReadStreambuf sbuf(reader);
        std::istream is(&sbuf);
        if (cafreader.ReadStream("stream", is) != IFSelect_RetDone) {
            return nullptr;
        }
        if (!cafreader.Transfer(doc)) {
            return nullptr;
        }

        Handle(XCAFDoc_ShapeTool) shapeTool =
            XCAFDoc_DocumentTool::ShapeTool(doc->Main());
        Handle(XCAFDoc_ColorTool) colorTool =
            XCAFDoc_DocumentTool::ColorTool(doc->Main());

        // Collect all free shapes into a compound.
        NCollection_Sequence<TDF_Label> roots;
        shapeTool->GetFreeShapes(roots);

        BRep_Builder builder;
        TopoDS_Compound compound;
        builder.MakeCompound(compound);
        for (int i = 1; i <= roots.Length(); i++) {
            builder.Add(compound, shapeTool->GetShape(roots.Value(i)));
        }

        std::unordered_map<uint64_t, std::array<float, 3>> colorMap;
        collect_colors(doc, colorTool, colorMap);

        // Recover Solids from disjoint shells / loose faces (#129); also remaps
        // colorMap keys for faces whose TShape* changed during sewing.
        TopoDS_Shape post = try_sew_orphan_faces(compound, &colorMap);

        // Walk the POST-processed shape so sewing's new TShape* are picked up, and
        // entries it no longer holds are dropped by not being reached.
        auto emit = [&](const TopoDS_Shape& sub) {
            uint64_t id = reinterpret_cast<uint64_t>(sub.TShape().get());
            auto it = colorMap.find(id);
            if (it == colorMap.end()) return;
            out_ids.push_back(id);
            out_rgb.push_back(it->second[0]);
            out_rgb.push_back(it->second[1]);
            out_rgb.push_back(it->second[2]);
        };
        for (TopExp_Explorer ex(post, TopAbs_FACE); ex.More(); ex.Next()) {
            emit(ex.Current());
        }
        for (TopExp_Explorer ex(post, TopAbs_SOLID); ex.More(); ex.Next()) {
            emit(ex.Current());
        }

        return std::make_unique<TopoDS_Shape>(post);
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return nullptr;
    }
}

bool write_step_color_stream(
    const TopoDS_Shape&         shape,
    rust::Slice<const uint64_t> ids,
    rust::Slice<const float>    rgb,
    RustWriter&                 writer)
{
    try {
        Handle(TDocStd_Document) doc = new TDocStd_Document("XmlXCAF");

        Handle(XCAFDoc_ShapeTool) shapeTool =
            XCAFDoc_DocumentTool::ShapeTool(doc->Main());
        Handle(XCAFDoc_ColorTool) colorTool =
            XCAFDoc_DocumentTool::ColorTool(doc->Main());

        // Register the root shape.
        TDF_Label rootLabel = shapeTool->AddShape(shape, false);

        // One lookup for both levels: which explorer finds an id decides the level
        // it is written at.
        std::unordered_map<uint64_t, std::array<float, 3>> colorLookup;
        for (size_t i = 0; i < ids.size(); i++) {
            colorLookup[ids[i]] = {rgb[3*i], rgb[3*i+1], rgb[3*i+2]};
        }

        // Find/create the sub-shape label of `sub` and paint it.
        auto set_color = [&](const TopoDS_Shape& sub, const std::array<float, 3>& c) {
            TDF_Label label;
            if (!shapeTool->FindSubShape(rootLabel, sub, label)) {
                label = shapeTool->AddSubShape(rootLabel, sub);
            }
            Quantity_Color color(c[0], c[1], c[2], Quantity_TOC_RGB);
            colorTool->SetColor(label, color, XCAFDoc_ColorSurf);
        };

        // Solids first: a face style is the more specific one and must be set after.
        for (TopExp_Explorer ex(shape, TopAbs_SOLID); ex.More(); ex.Next()) {
            const TopoDS_Shape& solid = ex.Current();
            auto it = colorLookup.find(
                reinterpret_cast<uint64_t>(solid.TShape().get()));
            if (it == colorLookup.end()) continue;
            set_color(solid, it->second);
        }

        for (TopExp_Explorer ex(shape, TopAbs_FACE); ex.More(); ex.Next()) {
            const TopoDS_Shape& face = ex.Current();
            auto it = colorLookup.find(
                reinterpret_cast<uint64_t>(face.TShape().get()));
            if (it == colorLookup.end()) continue;
            set_color(face, it->second);
        }

        // Transfer XDE doc to STEP model and write to stream.
        STEPCAFControl_Writer cafwriter;
        cafwriter.SetColorMode(true);
        if (!cafwriter.Transfer(doc)) {
            return false;
        }

        RustWriteStreambuf sbuf(writer);
        std::ostream os(&sbuf);
        return cafwriter.ChangeWriter().WriteStream(os) == IFSelect_RetDone;
    } catch (const Standard_Failure& failure) {
        record_standard_failure(__func__, "native", 7, failure);
        return false;
    }
}
} // namespace cadrum

#endif // FEATURE_COLOR
