# Steam Runtime 3.0 sniper SDK plus the repository's pinned Rust toolchain.
# Build this image; do not rustup during cargo build.
#
# Keep the digest in sync with packaging/linux/image (manifest digest, not config).
FROM registry.gitlab.steamos.cloud/steamrt/sniper/sdk@sha256:1c33c507bc75d012e77df5727f93b0d5b8c3f7c8d4142ba5f7a16882cc92e014

ARG RUST_CHANNEL=1.95.0
ENV RUSTUP_HOME=/opt/rust/rustup \
	CARGO_HOME=/opt/rust/cargo \
	PATH=/opt/rust/cargo/bin:$PATH

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
	sh -s -- -y --default-toolchain "${RUST_CHANNEL}" --profile minimal --no-modify-path \
	&& /opt/rust/cargo/bin/rustup target add x86_64-unknown-linux-gnu

WORKDIR /src
