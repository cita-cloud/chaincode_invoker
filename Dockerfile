FROM rust:slim-buster AS buildstage
WORKDIR /build
RUN /bin/sh -c set -eux;\
    rustup component add rustfmt;\
    apt-get update;\
    apt-get install -y --no-install-recommends git protobuf-compiler;\
    rm -rf /var/lib/apt/lists/*;
COPY . /build/
RUN cargo build --release --examples
FROM debian:buster-slim
COPY --from=buildstage /build/target/release/asset-transfer-basic /usr/bin/
COPY --from=buildstage /build/target/release/asset-transfer-secured-agreement /usr/bin/
CMD ["asset-transfer-basic"]
