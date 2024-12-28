FROM quay.io/pypa/manylinux_2_28_x86_64

# Install Rust
RUN curl --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Install Python dependencies
RUN python3 -m ensurepip
RUN python3 -m pip install --upgrade pip
RUN python3 -m pip install maturin

# Set the working directory
WORKDIR /io

# Copy the project files
COPY . .

# Build command
CMD ["maturin", "build", "--release"] 