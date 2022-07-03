FROM rust:1.62.0@sha256:8220e4fbb22a07b78e6472cdf8f5fb8913a04974c26b130177b73a8a64334541 as builder

ENV TARGET=x86_64-unknown-linux-musl
RUN rustup target add ${TARGET}

RUN rm -f /etc/apt/apt.conf.d/docker-clean; echo 'Binary::apt::APT::Keep-Downloaded-Packages "true";' > /etc/apt/apt.conf.d/keep-cache

# borrowed (Ba Dum Tss!) from
# https://github.com/pablodeymo/rust-musl-builder/blob/7a7ea3e909b1ef00c177d9eeac32d8c9d7d6a08c/Dockerfile#L48-L49
RUN --mount=type=cache,target=/var/cache/apt --mount=type=cache,target=/var/lib/apt \
    apt-get update && \
    apt-get --no-install-recommends install -y \
    build-essential \
    musl-dev \
    musl-tools

# The following block
# creates an empty app, and we copy in Cargo.toml and Cargo.lock as they represent our dependencies
# This allows us to copy in the source in a different layer which in turn allows us to leverage Docker's layer caching
# That means that if our dependencies don't change rebuilding is much faster
WORKDIR /build
RUN cargo new rust-end-to-end-application
WORKDIR /build/rust-end-to-end-application
COPY Cargo.toml Cargo.lock ./
RUN --mount=type=cache,target=/build/rust-end-to-end-application/target \
    cargo build --release --target ${TARGET}


# now we copy in the source which is more prone to changes and build it
COPY src ./src
# --release not needed, it is implied with install
RUN --mount=type=cache,target=/build/rust-end-to-end-application/target \
    cargo install --path . --target ${TARGET} --root /output

FROM alpine:3.16.0@sha256:686d8c9dfa6f3ccfc8230bc3178d23f84eeaf7e457f36f271ab1acc53015037c

RUN addgroup -S appgroup && adduser -S appuser -G appgroup
USER appuser

WORKDIR /app
COPY --from=builder /output/bin/rust-end-to-end-application /app
USER appuser
ENTRYPOINT ["/app/rust-end-to-end-application"]
