get_filename_component(IGUANODON_CMAKE_DIR "${CMAKE_CURRENT_LIST_FILE}" PATH)
get_filename_component(IGUANODON_DIST_ROOT "${IGUANODON_CMAKE_DIR}/.." ABSOLUTE)

add_library(Iguanodon::iguanodon_ffi SHARED IMPORTED)
if(ANDROID)
    set_target_properties(Iguanodon::iguanodon_ffi PROPERTIES
        IMPORTED_LOCATION "${IGUANODON_DIST_ROOT}/native/android-arm64-v8a/libiguanodon_ffi.so"
        INTERFACE_INCLUDE_DIRECTORIES "${IGUANODON_DIST_ROOT}/include"
    )
else()
    set_target_properties(Iguanodon::iguanodon_ffi PROPERTIES
        IMPORTED_LOCATION "${IGUANODON_DIST_ROOT}/native/linux-x86_64/libiguanodon_ffi.so"
        INTERFACE_INCLUDE_DIRECTORIES "${IGUANODON_DIST_ROOT}/include"
    )
endif()
