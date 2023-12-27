FROM rust:latest

WORKDIR /app

COPY . /app

ENV PORT=8080
ENV UPLOAD_DIR="uploads"

RUN cargo build --release

CMD ["/app/target/release/server"]
