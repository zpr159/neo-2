# Dockerfile — Neo AGI Operating System (Multi-stage)
#
# Build: docker build -f deployment/docker/Dockerfile -t neo .
# Run:   docker run --gpus all -p 8080:8080 neo

# === Stage 1: Rust builder ===
FROM rust:1.75-bookworm AS rust-builder

WORKDIR /build
COPY Cargo.toml Cargo.lock* ./
COPY core/ core/
COPY runtime/ runtime/
COPY neural-engine/ neural-engine/
COPY memory/ memory/
COPY knowledge-graph/ knowledge-graph/
COPY reasoning/ reasoning/
COPY executive/ executive/
COPY capabilities/ capabilities/
COPY agents/ agents/
COPY workflows/ workflows/
COPY tools/ tools/
COPY distributed/ distributed/
COPY security/ security/
COPY plugins/ plugins/
COPY sdk/ sdk/
COPY testing/ testing/

RUN cargo build --release --workspace && \
    strip target/release/neo-*

# === Stage 2: C++ builder ===
FROM ubuntu:22.04 AS cpp-builder

RUN apt-get update && apt-get install -y \
    cmake g++ make \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY CMakeLists.txt ./
COPY core/cpp/ core/cpp/
COPY runtime/cpp/ runtime/cpp/
COPY neural-engine/cpp/ neural-engine/cpp/
COPY memory/cpp/ memory/cpp/
COPY knowledge-graph/cpp/ knowledge-graph/cpp/
COPY reasoning/cpp/ reasoning/cpp/
COPY distributed/cpp/ distributed/cpp/
COPY security/cpp/ security/cpp/

RUN cmake -B build -DCMAKE_BUILD_TYPE=Release -DNEO_ENABLE_CUDA=OFF && \
    cmake --build build --parallel $(nproc)

# === Stage 3: Python builder ===
FROM python:3.12-slim AS python-builder

WORKDIR /build
COPY pyproject.toml ./
COPY neural-network-framework/python/ neural-network-framework/python/
COPY inference/python/ inference/python/
COPY knowledge-graph/python/ knowledge-graph/python/
COPY reasoning/python/ reasoning/python/
COPY capabilities/python/ capabilities/python/
COPY agents/python/ agents/python/
COPY tools/python/ tools/python/
COPY sdk/python/ sdk/python/
COPY plugins/python/ plugins/python/
COPY testing/python/ testing/python/

RUN pip install --no-cache-dir -e ./neural-network-framework/python \
    -e ./sdk/python

# === Stage 4: Runtime image ===
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libgomp1 \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd -r neo && useradd -r -g neo neo

WORKDIR /opt/neo

COPY --from=rust-builder /build/target/release/neo-* /usr/local/bin/
COPY --from=cpp-builder /build/build/lib/ /usr/local/lib/
COPY --from=python-builder /usr/local/lib/python3.12/site-packages/ /usr/local/lib/python3.12/site-packages/

COPY config/ /etc/neo/config/
COPY deployment/docker/entrypoint.sh /opt/neo/entrypoint.sh

RUN chmod +x /opt/neo/entrypoint.sh && \
    mkdir -p /data/neo /var/log/neo /var/lib/neo && \
    chown -R neo:neo /data/neo /var/log/neo /var/lib/neo /opt/neo

USER neo
EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD /usr/local/bin/neo-healthcheck || exit 1

ENTRYPOINT ["/opt/neo/entrypoint.sh"]
CMD ["neo-executive", "--config", "/etc/neo/config/production.toml"]
