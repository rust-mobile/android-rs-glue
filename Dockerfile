FROM tomaka/rust-android

RUN apt-get update
RUN apt-get install -yq pkg-config libssl-dev

COPY . /root/cargo-apk
RUN cargo install --path /root/cargo-apk/cargo-apk
RUN rm -rf /root/cargo-apk

RUN mkdir /root/src
WORKDIR /root/src
