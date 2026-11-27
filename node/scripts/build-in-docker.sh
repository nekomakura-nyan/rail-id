#!/bin/sh
# Build the Node addon for a Linux target inside a matching container.
#
#   docker run --rm -v "$PWD:/src" -w /src/node rust:1.88-alpine \
#     sh scripts/build-in-docker.sh linux-x64-musl
#
# The image supplies the architecture; the script installs the libc, a C
# toolchain, pkg-config and Node.js. glibc images are Debian based, musl images
# are Alpine based. The TPM addon is optional: pass `--tpm` after the target.
set -eu

TARGET="${1:?target required}"
shift || true

case "$TARGET" in
  linux-*-musl)
    if [ ! -f /etc/alpine-release ]; then
      echo "musl targets need an Alpine image (for example rust:1.88-alpine)" >&2
      exit 1
    fi
    apk add --no-cache bash curl build-base musl-dev nodejs pkgconf tpm2-tss-dev >/dev/null
    # musl defaults to crt-static, which forbids cdylib output. Node addons are
    # shared objects loaded into the musl-linked Node process.
    export RUSTFLAGS="${RUSTFLAGS:-} -C target-feature=-crt-static"
    ;;
  linux-arm-gnu)
    apt-get update -qq
    apt-get install -y -qq curl gcc-arm-linux-gnueabihf libc6-dev-armhf-cross \
      pkg-config libtss2-dev xz-utils >/dev/null
    export CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_LINKER=arm-linux-gnueabihf-gcc
    ;;
  linux-riscv64-gnu)
    apt-get update -qq
    apt-get install -y -qq curl gcc-riscv64-linux-gnu libc6-dev-riscv64-cross \
      pkg-config libtss2-dev xz-utils >/dev/null
    export CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_GNU_LINKER=riscv64-linux-gnu-gcc
    ;;
  *)
    apt-get update -qq
    apt-get install -y -qq curl pkg-config libtss2-dev xz-utils >/dev/null
    ;;
esac

if ! command -v cargo >/dev/null 2>&1 && [ ! -x /root/.cargo/bin/cargo ]; then
  curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain 1.88 >/dev/null
fi
export PATH="/root/.cargo/bin:$PATH"

case "$TARGET" in
  linux-arm-gnu)
    rustup target add armv7-unknown-linux-gnueabihf >/dev/null
    ;;
  linux-riscv64-gnu)
    rustup target add riscv64gc-unknown-linux-gnu >/dev/null
    ;;
esac

if ! command -v node >/dev/null 2>&1; then
  echo "node is required to run scripts/build.js" >&2
  exit 1
fi

exec node scripts/build.js --target "$TARGET" "$@"
