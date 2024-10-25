# Configuration Variables
# System compiler, can be overridden by the user
CC ?= cc
# Default installation prefix
PREFIX ?= /usr/local
# Verbose output for debugging
VERBOSE ?= 0
# Build type (release or debug)
BUILD_TYPE ?= release
# Generated C header file
HEADER_FILE := target/include/stl-thumb.h
# Target directory for the libs
TEST_LIB_DIR := target/lib

# Rust build flags based on build type
CARGO_FLAGS := --release
ifeq ($(BUILD_TYPE),debug)
	CARGO_FLAGS := --debug
endif

# Verbosity settings
ifeq ($(VERBOSE),1)
	VERBOSE_FLAG := --verbose
	CARGO_C_FLAGS := $(VERBOSE_FLAG)
	AR_FLAGS := rv
else
	VERBOSE_FLAG :=
	CARGO_C_FLAGS :=
	AR_FLAGS := rcs
endif

# Set RUSTFLAGS to avoid static linking against macOS SDK libraries
export RUSTFLAGS := -C link-arg=-dynamiclib

# Default target
all: build-shared build-static

# Check for required tools
check:
	@command -v cargo >/dev/null 2>&1 || { echo >&2 "Error: Cargo is not installed. Please install it to proceed."; exit 1; }
	@cargo cbuild --version >/dev/null 2>&1 || { echo >&2 "Error: cargo-c is not installed. Please install it with 'cargo install cargo-c'."; exit 1; }
	@command -v cbindgen >/dev/null 2>&1 || { echo >&2 "Error: cbindgen is not installed. Please install it with 'cargo install cbindgen'."; exit 1; }
	@command -v $(CC) >/dev/null 2>&1 || { echo >&2 "Error: $(CC) is not installed. Please install it to proceed."; exit 1; }

# Generate C headers using cbindgen
generate-header: check
	@echo "Generating C header with cbindgen..."
	@cbindgen --output $(HEADER_FILE)

# Build shared library
build-shared: check
	@echo "Building shared library..."
	@cargo cbuild $(CARGO_FLAGS) $(CARGO_C_FLAGS) --library-type cdylib

# Build static library
build-static: check
	@echo "Building static library..."
	@cargo cbuild $(CARGO_FLAGS) $(CARGO_C_FLAGS) --library-type staticlib

# Install both shared and static libraries
install: build-shared build-static
	@echo "Installing libraries to $(PREFIX)..."
	@cargo cinstall $(CARGO_FLAGS) $(CARGO_C_FLAGS) --prefix $(PREFIX)

# Compile and run the test linking against OpenSSL and libstd-thumb
test: generate-header build-static
	@command -v pkg-config >/dev/null 2>&1 || { echo >&2 "Error: pkg-config is not installed. Please install it to proceed."; exit 1; }
	@pkg-config --exists openssl || { echo >&2 "Error: OpenSSL is not found. Please install it to proceed."; exit 1; }
	@mkdir -p $(TEST_LIB_DIR)
	@echo "Building libstd-thumb..."
	@find ./target -type f -name 'libstl_thumb.a' -exec cp {} $(TEST_LIB_DIR)/ \; -quit
	@echo "Compiling and running test..."
	@$(CC) test/test.c -I$(dir $(HEADER_FILE)) -L$(TEST_LIB_DIR) $(shell pkg-config --cflags --libs openssl) -lstl_thumb -framework QuartzCore -framework Metal -lobjc -o test/test

# Clean build artifactsm
clean:
	@rm -f $(HEADER_FILE)
	@rm -rf $(TEST_LIB_DIR)
	@rm -f test/test
	@cargo clean

# Print debug information
print-vars:
	@echo "Compiler: $(CC)"
	@echo "Build Type: $(BUILD_TYPE)"
	@echo "Install Prefix: $(PREFIX)"
	@echo "Verbose Output: $(VERBOSE)"

# Phony targets
.PHONY: all check generate-header build-shared build-static test install clean print-vars
