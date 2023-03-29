FROM rust:latest

WORKDIR /joshuamo/flyio-example
COPY . .
RUN cargo install --path .
EXPOSE 8000
CMD ["flyio-example"]