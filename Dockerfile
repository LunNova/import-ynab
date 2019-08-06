# ------------------------------------------------------------------------------
# Cargo Build Stage
# ------------------------------------------------------------------------------

FROM rust:latest as cargo-build

#RUN apt-get update && apt-get install libssl-dev -y && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/import-ynab

COPY Cargo.toml Cargo.lock ./

RUN mkdir src/

RUN echo "#[cfg(test)] fn main() {println!(\"if you see this, the build broke\")}" > src/lib.rs

RUN cargo build --release

RUN rm -f target/release/deps/import-ynab* src/lib.rs

COPY . .

RUN touch src/lib.rs

RUN cargo build --release --offline

# ------------------------------------------------------------------------------
# Final Stage
# ------------------------------------------------------------------------------

FROM ubuntu:rolling

RUN addgroup --gid 1000 import-ynab && \
    adduser --disabled-login --shell /bin/sh --uid 1000 --ingroup import-ynab import-ynab && \
    apt-get update && apt-get install libssl1.1 libcurl4 -y && rm -rf /var/lib/apt/lists/*

WORKDIR /home/import-ynab/

COPY --from=cargo-build /usr/src/import-ynab/target/release/import-ynab import-ynab

RUN chown import-ynab:import-ynab import-ynab

USER import-ynab

VOLUME /home/import-ynab/secrets/

CMD ["./import-ynab"]