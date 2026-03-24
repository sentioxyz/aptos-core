### Aptos Tracer Image ###

FROM debian-base AS aptos-tracer

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    apt-get update && apt-get --no-install-recommends --allow-downgrades -y \
    install \
    wget \
    curl \
    perl-base \
    libtinfo6 \
    git \
    socat \
    python3-botocore \
    awscli \
    gnupg2 \
    pigz

COPY  --link --from=tracer-builder /aptos/dist/aptos-tracer /usr/local/bin/aptos-tracer

ENV RUST_LOG_FORMAT=json

# add build info
ARG BUILD_DATE
ENV BUILD_DATE ${BUILD_DATE}
ARG GIT_TAG
ENV GIT_TAG ${GIT_TAG}
ARG GIT_BRANCH
ENV GIT_BRANCH ${GIT_BRANCH}
ARG GIT_SHA
ENV GIT_SHA ${GIT_SHA}
