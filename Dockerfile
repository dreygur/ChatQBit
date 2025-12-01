# Build stage
FROM public.ecr.aws/docker/library/rust:1.91-alpine AS builder

RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Pin home crate to compatible version and build
# RUN cargo update home@0.5.12 --precise 0.5.9 &&
RUN cargo build --release

# Runtime stage
FROM alpine:3.21

RUN apk add --no-cache ca-certificates

WORKDIR /app

COPY --from=builder /app/target/release/chatqbit /app/chatqbit

EXPOSE 8081

CMD ["/app/chatqbit"]
