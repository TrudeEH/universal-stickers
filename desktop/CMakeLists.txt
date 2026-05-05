cmake_minimum_required(VERSION 3.21)

project(
    UniversalStickersDesktop
    VERSION 0.1.0
    HOMEPAGE_URL "https://github.com/TrudeEH/universal-stickers"
    LANGUAGES CXX
)

include(GNUInstallDirs)

set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)
set(CMAKE_AUTOUIC ON)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

find_package(Qt6 REQUIRED COMPONENTS Core Gui Widgets Svg)
find_package(Qt6 QUIET COMPONENTS DBus)
find_package(KF6GlobalAccel QUIET)

find_program(CARGO_EXECUTABLE cargo REQUIRED)

set(CARGO_TARGET_DIR ${CMAKE_CURRENT_BINARY_DIR}/cargo-target)
set(RUST_CARGO_BUILD_ARGS)
set(RUST_CARGO_PROFILE debug)
set(RUST_CXXBRIDGE_DIR ${CARGO_TARGET_DIR}/cxxbridge)
set(RUST_NATIVE_STATIC_LIBS)
set(RUST_NATIVE_LIB_OUTPUTS)
set(RUST_NATIVE_LIB_COLLECT_COMMAND)
set(UNIVERSAL_STICKERS_APP_ID "dev.trude.UniversalStickers")
set(UNIVERSAL_STICKERS_ICON_SVG "${CMAKE_CURRENT_SOURCE_DIR}/../icon.svg")
set(UNIVERSAL_STICKERS_WINDOWS_ICON "${CMAKE_CURRENT_SOURCE_DIR}/resources/icons/universal-stickers.ico")
set(UNIVERSAL_STICKERS_WINDOWS_ICON_RC "${UNIVERSAL_STICKERS_WINDOWS_ICON}")
set(UNIVERSAL_STICKERS_WINDOWS_RC)

if(CMAKE_BUILD_TYPE MATCHES "^(Release|RelWithDebInfo|MinSizeRel)$")
    set(RUST_CARGO_PROFILE release)
    list(APPEND RUST_CARGO_BUILD_ARGS --release)
endif()

if(WIN32 AND EXISTS "${UNIVERSAL_STICKERS_WINDOWS_ICON}")
    configure_file(
        "${CMAKE_CURRENT_SOURCE_DIR}/resources/app.rc.in"
        "${CMAKE_CURRENT_BINARY_DIR}/universal-stickers.rc"
        @ONLY
    )
    set(UNIVERSAL_STICKERS_WINDOWS_RC "${CMAKE_CURRENT_BINARY_DIR}/universal-stickers.rc")
endif()

if(WIN32 AND CMAKE_CXX_COMPILER_ID STREQUAL "GNU")
    set(RUST_CARGO_TARGET x86_64-pc-windows-gnu)
    list(APPEND RUST_CARGO_BUILD_ARGS --target ${RUST_CARGO_TARGET})
    set(RUST_STATIC_LIB ${CARGO_TARGET_DIR}/${RUST_CARGO_TARGET}/${RUST_CARGO_PROFILE}/libuniversal_stickers_ffi.a)
elseif(WIN32)
    set(RUST_STATIC_LIB ${CARGO_TARGET_DIR}/${RUST_CARGO_PROFILE}/universal_stickers_ffi.lib)
else()
    set(RUST_STATIC_LIB ${CARGO_TARGET_DIR}/${RUST_CARGO_PROFILE}/libuniversal_stickers_ffi.a)
endif()

if(UNIX AND NOT APPLE)
    set(RUST_NATIVE_LIB_DIR ${CARGO_TARGET_DIR}/native-libs)
    set(RUST_CXXBRIDGE_STATIC_LIB ${RUST_NATIVE_LIB_DIR}/libcxxbridge1.a)
    set(RUST_SQLITE_STATIC_LIB ${RUST_NATIVE_LIB_DIR}/libsqlite3.a)
    list(APPEND RUST_NATIVE_STATIC_LIBS
        ${RUST_CXXBRIDGE_STATIC_LIB}
        ${RUST_SQLITE_STATIC_LIB}
    )
    list(APPEND RUST_NATIVE_LIB_OUTPUTS ${RUST_NATIVE_STATIC_LIBS})
    set(RUST_NATIVE_LIB_COLLECT_COMMAND
        COMMAND ${CMAKE_COMMAND}
            -DRUST_PROFILE_DIR=${CARGO_TARGET_DIR}/${RUST_CARGO_PROFILE}
            -DRUST_NATIVE_LIB_DIR=${RUST_NATIVE_LIB_DIR}
            -P ${CMAKE_CURRENT_SOURCE_DIR}/cmake/CollectRustNativeLibs.cmake
    )
endif()
set(RUST_BRIDGE_CC ${RUST_CXXBRIDGE_DIR}/universal-stickers-ffi/src/lib.rs.cc)
set_source_files_properties(${RUST_BRIDGE_CC} PROPERTIES GENERATED TRUE SKIP_AUTOGEN TRUE SKIP_AUTOMOC TRUE)

add_custom_command(
    OUTPUT
        ${RUST_STATIC_LIB}
        ${RUST_NATIVE_LIB_OUTPUTS}
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
    ${RUST_NATIVE_LIB_COLLECT_COMMAND}
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
    resources/universal-stickers.qrc
    ${UNIVERSAL_STICKERS_WINDOWS_RC}
)

add_dependencies(universal-stickers rust_ffi)
set_target_properties(universal-stickers PROPERTIES WIN32_EXECUTABLE TRUE)

if(APPLE)
    set_target_properties(universal-stickers PROPERTIES
        MACOSX_BUNDLE TRUE
        MACOSX_BUNDLE_BUNDLE_NAME "Universal Stickers"
        MACOSX_BUNDLE_GUI_IDENTIFIER "${UNIVERSAL_STICKERS_APP_ID}"
        MACOSX_BUNDLE_SHORT_VERSION_STRING "${PROJECT_VERSION}"
        MACOSX_BUNDLE_BUNDLE_VERSION "${PROJECT_VERSION}"
        MACOSX_BUNDLE_COPYRIGHT "Copyright Universal Stickers Maintainers"
    )
endif()

target_include_directories(universal-stickers PRIVATE
    src
    ${RUST_CXXBRIDGE_DIR}
    ${RUST_CXXBRIDGE_DIR}/universal-stickers-ffi/src
)

target_link_libraries(universal-stickers PRIVATE
    Qt6::Core
    Qt6::Gui
    Qt6::Svg
    Qt6::Widgets
    ${RUST_STATIC_LIB}
    ${RUST_NATIVE_STATIC_LIBS}
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

install(TARGETS universal-stickers
    BUNDLE DESTINATION .
    RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
)

if(WIN32)
    qt_generate_deploy_app_script(
        TARGET universal-stickers
        OUTPUT_SCRIPT universal_stickers_deploy_script
        NO_UNSUPPORTED_PLATFORM_ERROR
        NO_TRANSLATIONS
    )
    install(SCRIPT ${universal_stickers_deploy_script})
elseif(UNIX AND NOT APPLE)
    configure_file(
        ${CMAKE_CURRENT_SOURCE_DIR}/packaging/dev.trude.UniversalStickers.desktop.in
        ${CMAKE_CURRENT_BINARY_DIR}/${UNIVERSAL_STICKERS_APP_ID}.desktop
        @ONLY
    )
    configure_file(
        ${CMAKE_CURRENT_SOURCE_DIR}/packaging/dev.trude.UniversalStickers.metainfo.xml.in
        ${CMAKE_CURRENT_BINARY_DIR}/${UNIVERSAL_STICKERS_APP_ID}.metainfo.xml
        @ONLY
    )
    install(
        FILES ${CMAKE_CURRENT_BINARY_DIR}/${UNIVERSAL_STICKERS_APP_ID}.desktop
        DESTINATION ${CMAKE_INSTALL_DATADIR}/applications
    )
    install(
        FILES ${CMAKE_CURRENT_BINARY_DIR}/${UNIVERSAL_STICKERS_APP_ID}.metainfo.xml
        DESTINATION ${CMAKE_INSTALL_DATADIR}/metainfo
    )
    install(
        FILES ${UNIVERSAL_STICKERS_ICON_SVG}
        DESTINATION ${CMAKE_INSTALL_DATADIR}/icons/hicolor/scalable/apps
        RENAME ${UNIVERSAL_STICKERS_APP_ID}.svg
    )
endif()

install(
    FILES
        "${CMAKE_CURRENT_SOURCE_DIR}/../README.md"
        "${CMAKE_CURRENT_SOURCE_DIR}/../LICENSE.txt"
    DESTINATION ${CMAKE_INSTALL_DATADIR}/doc/universal-stickers
)

set(CPACK_PACKAGE_NAME "universal-stickers")
set(CPACK_PACKAGE_VENDOR "Universal Stickers")
set(CPACK_PACKAGE_CONTACT "Universal Stickers Maintainers")
set(CPACK_PACKAGE_DESCRIPTION_SUMMARY "Small sticker and GIF picker built with Qt and Rust.")
set(CPACK_PACKAGE_DESCRIPTION_FILE "${CMAKE_CURRENT_SOURCE_DIR}/../README.md")
set(CPACK_PACKAGE_INSTALL_DIRECTORY "Universal Stickers")
set(CPACK_PACKAGE_EXECUTABLES "universal-stickers" "Universal Stickers")
set(CPACK_PACKAGE_HOMEPAGE_URL "${CMAKE_PROJECT_HOMEPAGE_URL}")
set(CPACK_PACKAGE_VERSION ${PROJECT_VERSION})
set(CPACK_RESOURCE_FILE_LICENSE "${CMAKE_CURRENT_SOURCE_DIR}/../LICENSE.txt")
set(CPACK_SOURCE_IGNORE_FILES "/build/;/.git/;/.idea/;/.vscode/;/.vs/;\\\\.user$")

if(WIN32)
    set(CPACK_WIX_VERSION 4)
    set(CPACK_WIX_UPGRADE_GUID "5BA0C74F-4651-4EA5-AB10-B147CC1F2057")
    if(EXISTS "${UNIVERSAL_STICKERS_WINDOWS_ICON}")
        set(CPACK_WIX_PRODUCT_ICON "${UNIVERSAL_STICKERS_WINDOWS_ICON}")
    endif()
elseif(UNIX AND NOT APPLE)
    set(CPACK_DEBIAN_FILE_NAME DEB-DEFAULT)
    set(CPACK_DEBIAN_PACKAGE_MAINTAINER "Universal Stickers Maintainers")
    set(CPACK_DEBIAN_PACKAGE_SECTION "utils")
    set(CPACK_DEBIAN_PACKAGE_SHLIBDEPS ON)
    set(CPACK_RPM_FILE_NAME RPM-DEFAULT)
    set(CPACK_RPM_PACKAGE_LICENSE "MIT")
    set(CPACK_RPM_PACKAGE_GROUP "Applications/Utilities")
    set(CPACK_RPM_PACKAGE_RELEASE 1)
endif()

include(CPack)
