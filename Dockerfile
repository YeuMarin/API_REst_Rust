#build statge
FROM rust:1.69-buster as builder

WORKDIR /app

#accept the build aguments
ARG DATABASE_URL

ENV DATABASE_URL=$DATABASE_URL

COPY . .

RUN cargo build --release

#Production state
FROM debian:buster-slim

WORKDIR /usr/local/bin

COPY --from=builder /app/target/release/Helados .

CMD ["./Helados"]