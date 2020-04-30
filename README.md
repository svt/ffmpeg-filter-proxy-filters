# ffmpeg-filter-proxy-filters

This repo contains filter implementations for [ffmpeg-filter-proxy][1].

All the filters so far are written in [Rust][2] and it's assumed that
you have the [Rust][2] toolchain installed if you want to build or test
any of the filters in this repo.

## [Subtitle Rendering Format filter](srf_filter)

A filter used to overlay subtitles at SVT.

The subtitle rendering format is essentially just a couple of [protobuf][3]
message [types](srf_filter/proto/subtitle_rendering_data.proto).

## [Timecoded SVG filter](tsvg_filter)

A filter used to overlay [SVG][4].

To see how the data should be formatted:

```console
gzip -dc samples/sample.tsvg | less
```

## [SVG filter](svg_filter)

A filter used to overlay a single [SVG][4] file, e.g. a logo.
 
## [JVM filter](jvm_filter)

A filter that embeds a Java Virtual Machine.

Let's you write the filter implementation in your favorite JVM language.

## Filter Runner

The filter runner can be used to test a filter implementation without
using [FFmpeg][5].

SRF (on Mac):

```console
(cd srf_filter; cargo build --release)
(cd filter_runner; cargo run -- ../srf_filter/target/release/libsrf_filter.dylib -c "srf=../samples/sample.srf" -o ../srf.png)
```

TSVG (on Mac):

```console
(cd tsvg_filter; cargo build --release)
(cd filter_runner; cargo run -- ../tsvg_filter/target/release/libtsvg_filter.dylib -c "tsvg=../samples/sample.tsvg" -o ../tsvg.png)
```

SVG (on Mac):

```console
(cd svg_filter; cargo build --release)
(cd filter_runner; cargo run -- ../svg_filter/target/release/libsvg_filter.dylib -c "svg=../samples/sample.svg" -o ../svg.png)
```

For more info about the available options:

```console
(cd filter_runner; cargo run -- --help)
```

## License

Copyright 2020 Sveriges Television AB

This software is released under the Apache 2.0 License.

## Primary Maintainers

Christer Sandberg <https://github.com/chrsan>

[1]: https://github.com/SVT/ffmpeg-filter-proxy
[2]: https://www.rust-lang.org
[3]: https://developers.google.com/protocol-buffers
[4]: https://developer.mozilla.org/en-US/docs/Web/SVG
[5]: https://www.ffmpeg.org
