# Check for required tools
$(shell command -v cargo >/dev/null 2>&1 || { echo >&2 "Error: Cargo is not installed. Please install it to proceed."; exit 1; })
$(shell command -v rustc >/dev/null 2>&1 || { echo >&2 "Error: rustc is not installed. Please install it to proceed."; exit 1; })
$(shell cargo cbuild --version >/dev/null 2>&1 || { echo >&2 "Error: cargo-c is not installed. Please install it with 'cargo install cargo-c'."; exit 1; })

# Configuration Variables
# System compiler, can be overridden by the user
CC ?= cc

# Basic check for the compiler
$(shell $(CC) --version >/dev/null 2>&1 || { echo >&2 "Error: The default compiler $(CC) is not working properly. Please check your compiler installation or specify a different compiler."; exit 1; })

# Default installation prefix
PREFIX ?= /usr/local
# Verbose output for debugging
VERBOSE ?= 0
# Build type (release or debug)
BUILD_TYPE ?= release

# Determine toolchain path for the current host using rustc
TOOLCHAIN := $(shell rustc --version --verbose | grep "host" | awk '{ print $$2 }')
# Target directory for the libs
TARGET_DIR := $(realpath $(dir $(lastword $(MAKEFILE_LIST))))/target/$(TOOLCHAIN)/$(BUILD_TYPE)

# Rust build flags based on build type
CARGO_FLAGS := --release
ifeq ($(BUILD_TYPE),debug)
	CARGO_FLAGS := --debug
endif

# Verbosity settings
ifeq ($(VERBOSE),1)
	AR_FLAGS := rv
	VERBOSE_FLAG := --verbose
	CARGO_C_FLAGS := $(VERBOSE_FLAG)
else
	AR_FLAGS := rcs
	VERBOSE_FLAG :=
	CARGO_C_FLAGS :=
endif

# Set RUSTFLAGS to avoid static linking against macOS SDK libraries
export RUSTFLAGS := -C link-arg=-dynamiclib

# Set PKG_CONFIG_PATH to the toolchain's build directory
PKG_CONFIG_PATH := $(TARGET_DIR)
export PKG_CONFIG_PATH

# Default target
all: build-shared build-static

# Build shared library
ifeq ($(shell uname),Darwin)
build-shared:
	@echo "Building shared library..."
	@cargo cbuild $(CARGO_FLAGS) $(CARGO_C_FLAGS) --library-type cdylib
# cargo-c is generating broken .pc files on macOS, so we need to fix them
	@sed -i ''-E '/^(Libs|Libs.private):/ s/ -framework//g; /^(Libs|Libs.private):/ s/ ([A-Z][a-zA-Z]+)/ -framework \1/g' $(PKG_CONFIG_PATH)/{stl_thumb.pc,stl_thumb-uninstalled.pc}
else
build-shared:
	@echo "Building shared library..."
	@cargo cbuild $(CARGO_FLAGS) $(CARGO_C_FLAGS) --library-type cdylib
endif

# Build static library
ifeq ($(shell uname),Darwin)
build-static:
	@echo "Building static library..."
	@cargo cbuild $(CARGO_FLAGS) $(CARGO_C_FLAGS) --library-type staticlib
# cargo-c is generating broken .pc files on macOS, so we need to fix them
	@sed -i '' -E '/^(Libs|Libs.private):/ s/ -framework//g; /^(Libs|Libs.private):/ s/ ([A-Z][a-zA-Z]+)/ -framework \1/g' $(PKG_CONFIG_PATH)/{stl_thumb.pc,stl_thumb-uninstalled.pc}
else
build-static:
	@echo "Building static library..."
	@cargo cbuild $(CARGO_FLAGS) $(CARGO_C_FLAGS) --library-type staticlib
endif

# Install both shared and static libraries
install: build-shared build-static
	@echo "Installing libraries to $(PREFIX)..."
	@cargo cinstall $(CARGO_FLAGS) $(CARGO_C_FLAGS) --prefix $(PREFIX)

# Compile and run the test linking against OpenSSL and libstd-thumb
test: build-static
	@command -v pkg-config >/dev/null 2>&1 || { echo >&2 "Error: pkg-config is not installed. Please install it to proceed."; exit 1; }
	@pkg-config --exists openssl || { echo >&2 "Error: OpenSSL is not found. Please install it to proceed."; exit 1; }
	@echo "Compiling test..."
	@$(CC) test/test.c `pkg-config --cflags --libs openssl` `pkg-config --cflags --libs stl_thumb` -o test/test

# Clean build artifacts
clean:
	@rm -f test/test
	@cargo clean

# Print debug information
print-vars:
	@echo "Compiler: $(CC)"
	@echo "Toolchain: $(TOOLCHAIN)"
	@echo "Build Type: $(BUILD_TYPE)"
	@echo "Install Prefix: $(PREFIX)"
	@echo "Verbose Output: $(VERBOSE)"

# Phony targets
.PHONY: all build-shared build-static test install clean print-vars
