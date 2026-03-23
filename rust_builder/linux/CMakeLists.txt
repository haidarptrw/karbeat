# The Flutter tooling requires that developers have CMake 3.10 or later
# installed. You should not increase this version, as doing so will cause
# the plugin to fail to compile for some customers of the plugin.
cmake_minimum_required(VERSION 3.10)

# Project-level configuration.
set(PROJECT_NAME "rust_lib_karbeat")
project(${PROJECT_NAME} LANGUAGES CXX)

include("../cargokit/cmake/cargokit.cmake")
# Point cargokit to the actual crate that builds the FFI library
# The workspace root (../../rust) is a workspace manifest without a [package]
# section; cargokit expects a package manifest. Use the crate folder instead.
# Use the crate's library name so the bundled filename matches the
# flutter_rust_bridge-generated loader (libkarbeat_flutter_ffi.so).
apply_cargokit(${PROJECT_NAME} ../../rust/karbeat-flutter-ffi karbeat_flutter_ffi "")

# List of absolute paths to libraries that should be bundled with the plugin.
# This list could contain prebuilt libraries, or libraries created by an
# external build triggered from this build file.
set(${PROJECT_NAME}_bundled_libraries
  "${${PROJECT_NAME}_cargokit_lib}"
  PARENT_SCOPE
)