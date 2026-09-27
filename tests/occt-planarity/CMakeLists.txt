cmake_minimum_required(VERSION 3.16)
project(occt_planarity_regression LANGUAGES CXX)
enable_testing()

if(NOT OCCT_ROOT)
  message(FATAL_ERROR "Set OCCT_ROOT to the OCCT installation under test")
endif()

add_executable(occt_planarity planarity.cpp)
target_compile_features(occt_planarity PRIVATE cxx_std_17)
find_path(OCCT_INCLUDE_DIR GeomLib_IsPlanarSurface.hxx PATHS
  "${OCCT_ROOT}/include/opencascade" "${OCCT_ROOT}/inc" "${OCCT_ROOT}/include"
  NO_DEFAULT_PATH REQUIRED)
target_include_directories(occt_planarity PRIVATE "${OCCT_INCLUDE_DIR}")
foreach(toolkit TKGeomBase TKG3d TKG2d TKMath TKernel)
  find_library(${toolkit}_LIBRARY NAMES ${toolkit} PATHS
    "${OCCT_ROOT}/lib" "${OCCT_ROOT}/win64/vc14/lib"
    "${OCCT_ROOT}/win64/gcc/lib" "${OCCT_ROOT}/win64/clang/lib"
    NO_DEFAULT_PATH REQUIRED)
  target_link_libraries(occt_planarity PRIVATE "${${toolkit}_LIBRARY}")
endforeach()
if(UNIX AND NOT APPLE)
  target_link_libraries(occt_planarity PRIVATE pthread ${CMAKE_DL_LIBS})
endif()
add_test(NAME occt_planarity COMMAND occt_planarity)
