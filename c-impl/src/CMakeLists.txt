function(add_files_from_directory DIR)
    file(GLOB_RECURSE SRC_FILES
        "${DIR}/*.c"
    )

    list(FILTER SRC_FILES EXCLUDE REGEX "test_.*\\.c")

    target_sources(ws_uring_lib PRIVATE ${SRC_FILES})

    file(GLOB TEST_FILES
        "${DIR}/test_*.c"
    )

    foreach(TEST_FILE ${TEST_FILES})
        get_filename_component(TEST_NAME ${TEST_FILE} NAME_WE)
        add_executable(${TEST_NAME} ${TEST_FILE})
        target_link_libraries(${TEST_NAME} PRIVATE ws_uring_lib)
    endforeach()
endfunction()

add_library(ws_uring_lib "")

add_files_from_directory("${CMAKE_CURRENT_SOURCE_DIR}")

add_subdirectory(domain)

# Find liburing and wolfssl, then link against them.
find_package(PkgConfig REQUIRED)
pkg_check_modules(LIBURING REQUIRED liburing)
pkg_check_modules(WOLFSSL REQUIRED wolfssl)

add_executable(main main.c)
target_include_directories(main PRIVATE 
    ${CMAKE_SOURCE_DIR}/include/stx/stx ${LIBURING_INCLUDE_DIRS} ${WOLFSSL_INCLUDE_DIRS})

target_link_libraries(main PRIVATE ws_uring_lib 
    ${LIBURING_LINK_LIBRARIES} ${WOLFSSL_LINK_LIBRARIES})
