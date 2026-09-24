# Casette — SageMath-backed native Linux calculator using Rust & egui
#
# Quick start:
#   make           # debug build of all workspace crates
#   make run       # build + launch the Casette GUI
#   make test      # run unit, lowering, and kernel integration tests
#   make check     # compiler check + clippy static analysis
#   make release   # optimized release build
#   make install   # install to ~/.local/bin and install desktop shortcut

PREFIX ?= $(HOME)/.local
BINDIR ?= $(PREFIX)/bin
APPDIR ?= $(PREFIX)/share/applications
ICONDIR ?= $(PREFIX)/share/icons/hicolor/512x512/apps

.PHONY: all build run test check fmt release clean install uninstall help

all: build

build:
	cargo build

run:
	cargo run -p casette-gui

test:
	cargo test --workspace

check:
	cargo check --workspace
	cargo clippy --workspace --all-targets

fmt:
	cargo fmt --all

release:
	cargo build --release -p casette-gui

clean:
	cargo clean

install: release
	install -d $(BINDIR)
	install -m 755 target/release/casette $(BINDIR)/casette
	install -d $(APPDIR)
	@echo "[Desktop Entry]" > $(APPDIR)/casette.desktop
	@echo "Type=Application" >> $(APPDIR)/casette.desktop
	@echo "Name=Casette" >> $(APPDIR)/casette.desktop
	@echo "Comment=SageMath-backed CAS calculator with persistent session tape" >> $(APPDIR)/casette.desktop
	@echo "Exec=$(BINDIR)/casette" >> $(APPDIR)/casette.desktop
	@echo "Terminal=false" >> $(APPDIR)/casette.desktop
	@echo "Categories=Science;Math;Education;" >> $(APPDIR)/casette.desktop
	@echo "Installed casette to $(BINDIR)/casette and desktop file to $(APPDIR)/casette.desktop"

uninstall:
	rm -f $(BINDIR)/casette
	rm -f $(APPDIR)/casette.desktop

help:
	@echo "Casette Build System (Rust & egui)"
	@echo ""
	@echo "Targets:"
	@echo "  make          Build all workspace crates in debug mode"
	@echo "  make run      Launch the Casette desktop GUI"
	@echo "  make test     Run entire unit and integration test suite"
	@echo "  make check    Run cargo check and cargo clippy on workspace"
	@echo "  make fmt      Format all Rust source code with cargo fmt"
	@echo "  make release  Build optimized release binary in target/release/casette"
	@echo "  make install  Install casette binary and .desktop entry to ~/.local"
	@echo "  make clean    Remove build artifacts (cargo clean)"
