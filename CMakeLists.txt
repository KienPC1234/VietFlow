cmake_minimum_required(VERSION 3.21)

project(VietFlow
    VERSION 0.1.0
    DESCRIPTION "High-performance Vietnamese input engine"
    LANGUAGES C CXX
)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# Distribution directory
set(VIETFLOW_DIST_DIR "${CMAKE_CURRENT_SOURCE_DIR}/dist" CACHE PATH "Output directory for unified release distribution")

# Build options
option(VIETFLOW_BUILD_UI "Build Qt 6 QML configuration UI" ON)
option(VIETFLOW_SEPARATE_RUNTIME "Distribute Qt UI runtime separately instead of single-file embedding" OFF)
option(VIETFLOW_PACK_VELOPACK "Generate Velopack installer and delta packages" OFF)
option(VIETFLOW_PACK_MSI "Generate machine-wide MSI installer during Velopack packaging" OFF)

find_program(CARGO_EXECUTABLE cargo REQUIRED)
find_program(PWSH_EXECUTABLE NAMES pwsh powershell)

# -----------------------------------------------------------------------------
# 1. Rust Core Engine Target
# -----------------------------------------------------------------------------
set(CARGO_MANIFEST "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml")

add_custom_target(vietflow_core
    COMMAND ${CARGO_EXECUTABLE} build --manifest-path "${CARGO_MANIFEST}" --release -p vietflow-core
    WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
    COMMENT "Compiling vietflow-core (Rust engine & C ABI)"
)

add_custom_target(vietflow_test
    COMMAND ${CARGO_EXECUTABLE} test --manifest-path "${CARGO_MANIFEST}" --workspace
    WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
    COMMENT "Running VietFlow automated test suite"
)

# -----------------------------------------------------------------------------
# 2. Qt 6 UI Frontend
# -----------------------------------------------------------------------------
if(VIETFLOW_BUILD_UI)
    find_package(Qt6 6.5 COMPONENTS Quick QuickControls2 Gui QUIET)
    if(Qt6_FOUND)
        add_subdirectory(ui)
        if(TARGET vietflow-ui)
            add_dependencies(vietflow-ui vietflow_core)
        endif()
    else()
        message(WARNING "Qt 6.5+ not found. UI target will be skipped. Set CMAKE_PREFIX_PATH to your Qt installation.")
    endif()
endif()

# -----------------------------------------------------------------------------
# 3. Distribution & Packaging Targets
# -----------------------------------------------------------------------------
if(WIN32 AND PWSH_EXECUTABLE)
    # Target: dist (General release build)
    set(DIST_ARGS -DistDir "${VIETFLOW_DIST_DIR}")
    if(VIETFLOW_SEPARATE_RUNTIME)
        list(APPEND DIST_ARGS -Separate)
    endif()
    if(VIETFLOW_PACK_VELOPACK)
        list(APPEND DIST_ARGS -Velopack)
    endif()
    if(VIETFLOW_PACK_MSI)
        list(APPEND DIST_ARGS -Msi)
    endif()

    add_custom_target(dist
        COMMAND ${PWSH_EXECUTABLE} -NoProfile -ExecutionPolicy Bypass -File "${CMAKE_CURRENT_SOURCE_DIR}/scripts/build.ps1" ${DIST_ARGS}
        WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
        COMMENT "Building unified release distribution to ${VIETFLOW_DIST_DIR}"
    )

    # Target: package_velopack (Direct Velopack packaging target)
    add_custom_target(package_velopack
        COMMAND ${PWSH_EXECUTABLE} -NoProfile -ExecutionPolicy Bypass -File "${CMAKE_CURRENT_SOURCE_DIR}/scripts/build.ps1" -DistDir "${VIETFLOW_DIST_DIR}" -Velopack
        WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
        COMMENT "Packaging VietFlow with Velopack (Setup.exe & nupkg)"
    )

    # Target: package_msi (Direct Velopack MSI packaging target)
    add_custom_target(package_msi
        COMMAND ${PWSH_EXECUTABLE} -NoProfile -ExecutionPolicy Bypass -File "${CMAKE_CURRENT_SOURCE_DIR}/scripts/build.ps1" -DistDir "${VIETFLOW_DIST_DIR}" -Velopack -Msi
        WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
        COMMENT "Packaging VietFlow with Velopack and MSI installer"
    )
else()
    # Non-Windows distribution target
    add_custom_target(dist
        COMMAND ${CMAKE_COMMAND} -E make_directory "${VIETFLOW_DIST_DIR}"
        COMMAND ${CMAKE_COMMAND} -E copy_directory "${CMAKE_CURRENT_SOURCE_DIR}/target/release" "${VIETFLOW_DIST_DIR}"
        DEPENDS vietflow_core
        COMMENT "Assembling platform distribution into ${VIETFLOW_DIST_DIR}"
    )
endif()

