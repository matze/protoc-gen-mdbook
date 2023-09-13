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


## Options

You can control the output behavior of `protoc-gen-mdbook` using `mdbook_opt`
option passed to `protoc` driver. The option is a comma-separated list of
key-value pairs separated by a colon. The following keys are understood:

* `output`: a filename for the entire output. If not set, multiple files will be
  generated.
* `optimize`: right now can be `doxygen` to optimize for inclusion in Doxygen
  Markdown documentation, most importantly to fix header links. All other values
  are ignored.

A call to output to a single file optimized for Doxygen would look like this:

    $ protoc --mdbook_out=. --mdbook_opt=output:single.md,optimize:doxygen path/to/*.proto
