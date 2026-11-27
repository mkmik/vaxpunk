# The OS build and boot toolchain, scripts/setup-host.sh's Linux packages plus
# Rust and uv, for the boot job in .github/workflows/boot.yml. That workflow
# pushes it to ghcr.io tagged by this file's hash, so editing it rebuilds it.
FROM ubuntu:24.04@sha256:562111b88caa79c71391c0c4d3c2114a7d5c35915c20561fd893cb1e07929e6b

RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
		bzip2 ca-certificates git curl build-essential gcc-aarch64-linux-gnu libc6-dev-arm64-cross cmake ninja-build \
		device-tree-compiler libxml2-utils python3 mtools procps \
		qemu-system-arm qemu-efi-aarch64 ipxe-qemu \
	&& rm -rf /var/lib/apt/lists/*

COPY --from=ghcr.io/astral-sh/uv:latest@sha256:a3ea1357342c1a4bf9a0c53a3deb1b55379aefa037338d8113d5771efa01774f /uv /usr/local/bin/uv

# GitHub sets HOME to /github/home in containers: keep Rust out of it.
ENV RUSTUP_HOME=/usr/local/rustup CARGO_HOME=/usr/local/cargo PATH=/usr/local/cargo/bin:$PATH
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path --profile minimal
# pal/kernel/build.rs would guess it; say it.
ENV CROSS_COMPILE=aarch64-linux-gnu-
