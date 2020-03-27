# srf_filter

## Generating subtitle_rendering_data.rs

### Setup

Make sure that `protoc` is installed.

On OS X:

```console
brew install protoc
```

On Ubuntu:

```console
apt-get install protobuf-compiler
```

Install [`protoc-gen-rust`][1] using:

```console
cargo install --version xxx protobuf-codegen
```

Where `xxx` should be the version used for the `protobuf`
dependency in `Cargo.toml`.

### Generate

```console
protoc \
    --proto_path ../proto` \
    --rust_out src \
    ../proto/subtitle_rendering_data.proto
```

[1]: https://github.com/stepancheg/rust-protobuf/tree/master/protobuf-codegen
