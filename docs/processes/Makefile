SHELL := /bin/sh

BREW_PREFIX := $(shell brew --prefix 2>/dev/null)
PREFIX ?= $(if $(BREW_PREFIX),$(BREW_PREFIX),/usr/local)
LIBDIR ?= $(PREFIX)/lib
INCLUDEDIR ?= $(PREFIX)/include
PKGCONFIGDIR ?= $(LIBDIR)/pkgconfig

.PHONY: all pre build

all: help

help:
	@printf "\033[1;37mMakefile Targets\033[00m\n"
	@grep -E '^[a-z]+[^:]+:.*## '  Makefile | sort | while read -r l; do printf "  \033[1;32m$$(echo $$l | cut -f 1 -d':')\033[00m:$$(echo $$l | cut -f 3- -d'#')\n"; done
	@echo ""

pre: ## check required tools, libraries, and local build configuration
	@printf '\033[1;37m%s\033[00m\n' 'Checking build prerequisites'
	@set -eu; \
	os_name="$$(uname -s)"; \
	case "$$os_name" in \
		Darwin) install_cmd='brew install' ;; \
		Linux) install_cmd='sudo apt-get install' ;; \
		*) install_cmd='install manually' ;; \
	esac; \
	failed=0; \
	check_cmd() { \
		label="$$1"; \
		cmd="$$2"; \
		hint="$$3"; \
		if command -v "$$cmd" >/dev/null 2>&1; then \
			printf '\033[1;32m%s\033[00m %s\n' '  ✓' "$$label"; \
		else \
			printf '\033[1;31m%s %s\033[00m → \033[1;37m%s %s\033[00m\n' '  ✗' "$$label" "$$install_cmd" "$$hint"; \
			failed=1; \
		fi; \
	}; \
	check_cmd 'pandoc' pandoc pandoc; \
	check_cmd 'd2' d2 d2;

build: pre index.html ## Build the documentation

index.html:
	pandoc index.md \
		--lua-filter=d2-filter.lua \
		--standalone \
		--embed-resources \
		--toc \
		-f markdown \
		--css ./assets/style.css \
		-o index.html
