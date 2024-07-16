FROM docker.io/library/rust:1.79-bookworm

RUN DEBIAN_FRONTEND=noninteractive apt-get update && \
	apt-get install -yq --no-install-recommends \
	curl wget git make gcc-arm-none-eabi build-essential \
	cmake libusb-1.0 libusb-1.0-0-dev gdb-multiarch \
	&& rm -rf /var/lib/apt/lists/*

RUN ln -s /usr/bin/gdb-multiarch /usr/bin/arm-none-eabi-gdb

RUN rustup target add thumbv7em-none-eabi
RUN rustup component add llvm-tools-preview
RUN rustup component add rustfmt
RUN cargo install --locked cargo-binutils@0.3.6

# Install specific version of stlink tools
RUN git clone --single-branch -b "fix-gdb-osabi" https://github.com/bauen1/stlink.git /tmp/stlink && \
	cd /tmp/stlink && \
	make clean && \
	make release && \
	make install && \
	ldconfig && \
	rm -r /tmp/stlink
