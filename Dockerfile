FROM rust:slim-buster as build
WORKDIR /usr/src/stbcs
COPY . .
RUN cargo build --release

FROM debian:buster-slim
COPY --from=build /usr/src/stbcs/target/release/stbcs /usr/bin/
CMD ["stbcs"]

