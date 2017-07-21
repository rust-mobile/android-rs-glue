FROM tomaka/rust-android

COPY . /root/cargo-apk
RUN cargo install --path /root/cargo-apk/cargo-apk
RUN rm -rf /root/cargo-apk

WORKDIR /root
