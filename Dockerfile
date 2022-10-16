# Configure environment variables
ARG ALPINE_VERSION="latest"
ARG PACKAGE="ipiis"

# Be ready for serving
FROM docker.io/alpine:${ALPINE_VERSION} as server

# Configure default environment variables
ENV ipiis_client_account_primary_address="127.0.0.1:9801"
ENV ipiis_server_port="9801"

# Server Configuration
EXPOSE 9801/tcp
EXPOSE 9801/udp
WORKDIR /usr/local/bin
CMD [ "/bin/sh" ]

# Install dependencies
RUN apk add --no-cache libgcc

# Be ready for building
FROM docker.io/rust:1-alpine${ALPINE_VERSION} as builder

# Install dependencies
RUN apk add --no-cache musl-dev

# Load source files
ADD . /src
WORKDIR /src

# Build it!
RUN mkdir /out \
    && cargo build --all --workspace --release \
    && find ./target/release/ -maxdepth 1 -type f -perm +a=x -print0 | xargs -0 -I {} mv {} /out \
    && mv ./LICENSE-* / \
    && rm -rf /src

# Copy executable files
FROM server
COPY --from=builder /out/* /usr/local/bin/
COPY --from=builder /LICENSE-* /usr/share/licenses/${PACKAGE}/
