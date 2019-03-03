FROM tomaka/rust-android

RUN rustup update

# For dep caching
RUN mkdir -p /root/cargo-apk/cargo-apk 
COPY cargo-apk/Cargo.toml /root/cargo-apk/cargo-apk/Cargo.toml
COPY cargo-apk/Cargo.lock /root/cargo-apk/cargo-apk/Cargo.lock
RUN mkdir /root/cargo-apk/cargo-apk/src && echo "// dummy file" > /root/cargo-apk/cargo-apk/src/main.rs
RUN cargo install --path /root/cargo-apk/cargo-apk || true

COPY . /root/cargo-apk
RUN cargo install --path /root/cargo-apk/cargo-apk
RUN rm -rf /root/cargo-apk

RUN mkdir /root/src
WORKDIR /root/src
