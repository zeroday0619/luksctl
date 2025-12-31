.PHONY: all build release debug clean install uninstall help

PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/bin

CARGO := cargo
INSTALL := install
RM := rm -f

BINARIES := luks_mount luks_umount

all: release

build: release

release:
	$(CARGO) build --release

debug:
	$(CARGO) build

clean:
	$(CARGO) clean

install: release
	@echo "Installing to $(BINDIR)..."
	$(INSTALL) -d $(BINDIR)
	$(INSTALL) -m 755 target/release/luks_mount $(BINDIR)/luks_mount
	$(INSTALL) -m 755 target/release/luks_umount $(BINDIR)/luks_umount
	@echo "Installation complete!"
	@echo "  - $(BINDIR)/luks_mount"
	@echo "  - $(BINDIR)/luks_umount"

uninstall:
	@echo "Uninstalling from $(BINDIR)..."
	$(RM) $(BINDIR)/luks_mount
	$(RM) $(BINDIR)/luks_umount
	@echo "Uninstallation complete!"

help:
	@echo "luksctl Makefile"
	@echo ""
	@echo "Usage:"
	@echo "  make              Build release version"
	@echo "  make build        Build release version"
	@echo "  make release      Build release version"
	@echo "  make debug        Build debug version"
	@echo "  make clean        Clean build artifacts"
	@echo "  make install      Install to $(BINDIR) (requires sudo)"
	@echo "  make uninstall    Remove from $(BINDIR) (requires sudo)"
	@echo ""
	@echo "Variables:"
	@echo "  PREFIX=$(PREFIX)"
	@echo "  BINDIR=$(BINDIR)"
	@echo ""
	@echo "Examples:"
	@echo "  sudo make install"
	@echo "  sudo make PREFIX=/opt/luksctl install"
	@echo "  sudo make uninstall"
