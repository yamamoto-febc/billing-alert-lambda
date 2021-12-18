default: run

DOCKER_PLATFORM ?= linux/amd64
RUST_VERSION    ?= 1.57
RUST_ARCH       ?= x86_64-unknown-linux-gnu
DEPS            ?= bootstrap/src/*.rs bootstrap/Cargo.toml Cargo.toml Cargo.lock

RELEASE_DIR     := ${PWD}/target/${RUST_ARCH}/release
TARGET_BIN      := ${RELEASE_DIR}/bootstrap
TARGET_ZIP      := lambda.zip

build:
	cargo build --release --target ${RUST_ARCH}

.PHONY: buildx
buildx: ${TARGET_BIN}

${TARGET_BIN}: ${DEPS}
	docker run -it --rm --platform ${DOCKER_PLATFORM} \
	  -v "$${PWD}":/usr/src/myapp -w /usr/src/myapp rust:${RUST_VERSION} \
	  make build

.PHONY: zip
zip:  ${TARGET_ZIP}
${TARGET_ZIP}: ${TARGET_BIN}
	zip -j "${TARGET_ZIP}" "${TARGET_BIN}"

.PHONY: run
run: ${TARGET_BIN}
	cat event.json | docker run -i --rm \
	  --user "$(id -u)":"$(id -g)" \
	  -e DOCKER_LAMBDA_USE_STDIN=1 \
	  -e RUST_BACKTRACE=1 \
	  -e SLACK_POST_URL \
	  -e SLACK_CHANNEL \
	  -v "${RELEASE_DIR}":/var/task lambci/lambda:provided.al2


.PHONY: clean
clean:
	rm -f "${TARGET_ZIP}"; \
	cargo clean

.PHONY: fmt
fmt:
	cargo fmt
