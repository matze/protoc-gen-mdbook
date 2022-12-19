# protoc-gen-mdbook

A protoc plugin to generate [mdBook](https://rust-lang.github.io/mdBook/) pages
documenting services and related messages.


## Build

Install a recent Rust toolchain and build the binary with

    $ cargo b --release


## Usage

Like every protoc plugin, `protoc-gen-mdbook` must be in your `PATH` and can be
called by passing the `mdbook_out` parameter. Hence you can render Markdown
pages with

    $ protoc --mdbook_out=. path/to/*.proto

Note that you need a custom version of [highlight.js](https://highlightjs.org)
that includes support for protocol buffers if you want the rendered message
types to be syntax highlighted.


## Option

`protoc-gen-mdbook` interprets the `mdbook_opt` option passed to `protoc` as a
switch to render a single Markdown page rather than one per proto file with the
option being the filename.

    $ protoc --mdbook_out=. --mdbook_opt=single.md path/to/*.proto
