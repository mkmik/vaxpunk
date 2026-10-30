# The OS build and boot toolchain, scripts/setup-host.sh's Linux packages plus
# Rust and uv, for the boot job in .github/workflows/boot.yml. That workflow
# pushes it to ghcr.io tagged by this file's hash, so editing it rebuilds it.
FROM ubuntu:24.04@sha256:fe81560990fae86dc4743202b2cb4d3df98d0bcf949173cecef15ce1127ef852

RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
		bzip2 ca-certificates git curl build-essential gcc-aarch64-linux-gnu libc6-dev-arm64-cross cmake ninja-build \
		device-tree-compiler libxml2-utils python3 mtools procps \
		qemu-system-arm qemu-efi-aarch64 ipxe-qemu \
	&& rm -rf /var/lib/apt/lists/*

COPY --from=ghcr.io/astral-sh/uv:latest@sha256:af22a52f5eafef3d78e551599d405c02df61919d18028f0cea8eda93bc9986d0 /uv /usr/local/bin/uv

# GitHub sets HOME to /github/home in containers: keep Rust out of it.
ENV RUSTUP_HOME=/usr/local/rustup CARGO_HOME=/usr/local/cargo PATH=/usr/local/cargo/bin:$PATH
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path --profile minimal
# pal/kernel/build.rs would guess it; say it.
ENV CROSS_COMPILE=aarch64-linux-gnu-
