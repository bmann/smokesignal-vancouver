# syntax=docker/dockerfile:1.4
FROM rust:latest AS build

RUN cargo install sqlx-cli@0.8.2 --no-default-features --features postgres
RUN cargo install sccache --version ^0.8
ENV RUSTC_WRAPPER=sccache SCCACHE_DIR=/sccache

RUN USER=root cargo new --bin smokesignal
RUN mkdir -p /app/
WORKDIR /app/

RUN --mount=type=bind,source=src,target=src \
    --mount=type=bind,source=migrations,target=migrations \
    --mount=type=bind,source=static,target=static \
    --mount=type=bind,source=i18n,target=i18n \
    --mount=type=bind,source=templates,target=templates \
    --mount=type=bind,source=build.rs,target=build.rs \
    --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
    --mount=type=bind,source=Cargo.lock,target=Cargo.lock \
    --mount=type=cache,target=/app/target/ \
    --mount=type=cache,target=$SCCACHE_DIR,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    <<EOF
set -e
cargo build --locked --release --bin smokesignal --target-dir . --no-default-features -F embed
EOF

RUN groupadd -g 1500 -r smokesignal && useradd -u 1501 -r -g smokesignal -d /var/lib/smokesignal -m smokesignal
RUN chown -R smokesignal:smokesignal /app/release/smokesignal

FROM gcr.io/distroless/cc

LABEL org.opencontainers.image.title="Smoke Signal"
LABEL org.opencontainers.image.description="An event and RSVP management application."
LABEL org.opencontainers.image.licenses="MIT"
LABEL org.opencontainers.image.authors="Nick Gerakines <nick.gerakines@gmail.com>"
LABEL org.opencontainers.image.source="https://tangled.sh/@smokesignal.events/smokesignal"
LABEL org.opencontainers.image.version="1.0.2"

WORKDIR /var/lib/smokesignal
USER smokesignal:smokesignal

COPY --from=build /etc/passwd /etc/passwd
COPY --from=build /etc/group /etc/group
COPY --from=build /app/release/smokesignal /var/lib/smokesignal/
COPY static /var/lib/smokesignal/static

ENV HTTP_STATIC_PATH=/var/lib/smokesignal/static

ENV RUST_LOG=info
ENV RUST_BACKTRACE=full

ENTRYPOINT ["/var/lib/smokesignal/smokesignal"]
