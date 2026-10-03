# The OS build and boot toolchain, scripts/setup-host.sh's Linux packages plus
# Rust and uv, for the boot job in .github/workflows/boot.yml. That workflow
# pushes it to ghcr.io tagged by this file's hash, so editing it rebuilds it.
FROM ubuntu:24.04

RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
		ca-certificates git curl build-essential gcc-aarch64-linux-gnu cmake ninja-build \
		device-tree-compiler libxml2-utils python3 mtools procps \
		qemu-system-arm qemu-efi-aarch64 ipxe-qemu \
	&& rm -rf /var/lib/apt/lists/*

COPY --from=ghcr.io/astral-sh/uv:latest /uv /usr/local/bin/uv

# GitHub sets HOME to /github/home in containers: keep Rust out of it.
ENV RUSTUP_HOME=/usr/local/rustup CARGO_HOME=/usr/local/cargo PATH=/usr/local/cargo/bin:$PATH
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path --profile minimal
# kernel/build.rs would guess it; say it.
ENV CROSS_COMPILE=aarch64-linux-gnu-
