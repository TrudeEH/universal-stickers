cmake_minimum_required(VERSION 3.21)

project(UniversalStickersDesktop LANGUAGES CXX)

set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)
set(CMAKE_AUTOUIC ON)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

find_package(Qt6 REQUIRED COMPONENTS Core Gui Widgets)
find_package(Qt6 QUIET COMPONENTS DBus)
find_package(KF6GlobalAccel QUIET)

find_program(CARGO_EXECUTABLE cargo REQUIRED)

set(CARGO_TARGET_DIR ${CMAKE_CURRENT_BINARY_DIR}/cargo-target)
set(RUST_CARGO_BUILD_ARGS)
set(RUST_CXXBRIDGE_DIR ${CARGO_TARGET_DIR}/cxxbridge)

if(WIN32 AND CMAKE_CXX_COMPILER_ID STREQUAL "GNU")
    set(RUST_CARGO_TARGET x86_64-pc-windows-gnu)
    list(APPEND RUST_CARGO_BUILD_ARGS --target ${RUST_CARGO_TARGET})
    set(RUST_STATIC_LIB ${CARGO_TARGET_DIR}/${RUST_CARGO_TARGET}/debug/libuniversal_stickers_ffi.a)
elseif(WIN32)
    set(RUST_STATIC_LIB ${CARGO_TARGET_DIR}/debug/universal_stickers_ffi.lib)
else()
    set(RUST_STATIC_LIB ${CARGO_TARGET_DIR}/debug/libuniversal_stickers_ffi.a)
endif()
set(RUST_BRIDGE_CC ${RUST_CXXBRIDGE_DIR}/universal-stickers-ffi/src/lib.rs.cc)
set_source_files_properties(${RUST_BRIDGE_CC} PROPERTIES GENERATED TRUE SKIP_AUTOGEN TRUE SKIP_AUTOMOC TRUE)

add_custom_command(
    OUTPUT ${RUST_STATIC_LIB}
    BYPRODUCTS ${RUST_BRIDGE_CC}
    DEPENDS
        ${CMAKE_CURRENT_SOURCE_DIR}/../Cargo.toml
        ${CMAKE_CURRENT_SOURCE_DIR}/../Cargo.lock
        ${CMAKE_CURRENT_SOURCE_DIR}/../core/Cargo.toml
        ${CMAKE_CURRENT_SOURCE_DIR}/../core/src/lib.rs
        ${CMAKE_CURRENT_SOURCE_DIR}/../ffi/Cargo.toml
        ${CMAKE_CURRENT_SOURCE_DIR}/../ffi/build.rs
        ${CMAKE_CURRENT_SOURCE_DIR}/../ffi/src/lib.rs
    COMMAND ${CMAKE_COMMAND} -E env
        CARGO_TARGET_DIR=${CARGO_TARGET_DIR}
        CXX_x86_64_pc_windows_gnu=${CMAKE_CXX_COMPILER}
        ${CARGO_EXECUTABLE} build -p universal-stickers-ffi ${RUST_CARGO_BUILD_ARGS}
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}/..
    COMMENT "Building Rust FFI library"
    VERBATIM
)

add_custom_target(rust_ffi ALL DEPENDS ${RUST_STATIC_LIB})

add_executable(universal-stickers
    ${RUST_BRIDGE_CC}
    src/main.cpp
    src/MainWindow.cpp
    src/MainWindow.h
    src/StickerTile.cpp
    src/StickerTile.h
    src/ClipboardService.cpp
    src/ClipboardService.h
    src/HotkeyService.cpp
    src/HotkeyService.h
    src/RustBridge.h
)

add_dependencies(universal-stickers rust_ffi)

target_include_directories(universal-stickers PRIVATE
    src
    ${RUST_CXXBRIDGE_DIR}
    ${RUST_CXXBRIDGE_DIR}/universal-stickers-ffi/src
)

target_link_libraries(universal-stickers PRIVATE
    Qt6::Core
    Qt6::Gui
    Qt6::Widgets
    ${RUST_STATIC_LIB}
)

if(TARGET Qt6::DBus)
    target_link_libraries(universal-stickers PRIVATE Qt6::DBus)
endif()

if(TARGET KF6::GlobalAccel)
    target_compile_definitions(universal-stickers PRIVATE HAS_KGLOBALACCEL=1)
    target_link_libraries(universal-stickers PRIVATE KF6::GlobalAccel)
endif()

if(WIN32)
    target_link_libraries(universal-stickers PRIVATE user32 shell32 advapi32 bcrypt ntdll userenv ws2_32)
elseif(UNIX AND NOT APPLE)
    target_link_libraries(universal-stickers PRIVATE dl pthread m)
endif()
