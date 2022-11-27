FROM rust:latest AS Builder

RUN update-ca-certificates

# Create appuser
ENV USER=gamesite
ENV UID=10024

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    "${USER}"


WORKDIR /gamesite

COPY ./ .

RUN cargo build --release

#################### FINAL IMAGE ####################################
FROM gcr.io/distroless/cc

ENV USER=gamesite

# Import from builder.
COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

WORKDIR /gamesite

COPY --from=builder --chown=$USER:$USER /gamesite/target/release/game_site_be ./
COPY --from=builder --chown=$USER:$USER /gamesite/assets ./assets
USER gamesite

EXPOSE 9000
CMD ["/gamesite/game_site_be"]