# Function to add all files from a directory
function(add_files_from_directory DIR)
    # Collect all source files in the directory
    file(GLOB_RECURSE SRC_FILES
        "${DIR}/*.c"
    )

    # Filter out the test files
    list(FILTER SRC_FILES EXCLUDE REGEX "test_.*\\.c")

    # Add source files to the library
    target_sources(ws_uring_lib PRIVATE ${SRC_FILES})

    # Collect all test files in the directory
    file(GLOB TEST_FILES
        "${DIR}/test_*.c"
    )

    # Loop through each test file and create an executable
    foreach(TEST_FILE ${TEST_FILES})
        # Get the filename without the extension
        get_filename_component(TEST_NAME ${TEST_FILE} NAME_WE)

        # Create an executable for each test
        add_executable(${TEST_NAME} ${TEST_FILE})

        # Link the test executable against the project library
        target_link_libraries(${TEST_NAME} PRIVATE ws_uring_lib)
    endforeach()
endfunction()

# Create the library target
add_library(ws_uring_lib "")

# Recursively add files from the src directory and its subdirectories
add_files_from_directory("${CMAKE_CURRENT_SOURCE_DIR}")

# Optionally, if you want to include additional subdirectories explicitly
add_subdirectory(abc1)
add_subdirectory(abc2) 

add_executable(main main.c)
target_link_libraries(main PRIVATE ws_uring_lib)