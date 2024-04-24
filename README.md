# Manners

Manners is a CLI utility that generates manpages for Rust libraries. It uses the
experimental
[`rustdoc-json`](https://rust-lang.github.io/rfcs/2963-rustdoc-json.html)
feature to output documentation similar to what you would normally recieve from
rustdoc, but in manpage form.

## Usage

### Installation

Simply run `cargo install manners` to install manners; it has a total of 76
indirect dependencies, so it shouldn't take too long.

### Running

Usage is simple. Run `manners` with a list of paths to the `Cargo.toml`s of each
crate to generate documentation for. Alternatively, pass `-j` and a list of
paths to premade JSON manifests.

### Output

By default, the generated manpages get placed in `./output`. Pass
`-o/--output <path>` to specify a different directory. Note that the generated
manfiles have the section `3r` to avoid conflicts with existing manpages. The
manpages are compressed using gzip.

### Documenting `std`

If you attempt to document the standard library from source, you'll run into
unresolvable errors. Instead, run the following command to download the JSON
manifests for nightly:

```shell
$ rustup component add rust-docs-json --toolchain nightly
```

The resulting files can be found in
`~/.rustup/toolchains/nightly*/share/doc/rust/json`; run `manners` with the `-j`
flag to translate these into manpages as well.

## Known issues

Because of the nature of manpages, there are no links. While it's possible to
print the full path of the referenced item whereever there would be a link, this would
quickly clutter the screen; instead, any intra-doc links are listed at the end
under `SEE ALSO`.

Rust documentation is also much more comprehensive than the normal manpage,
consisting of far more syntax and far less prose than is typical. This cannot be
worked around, but any suggestions for improving the style or searchability of
the pages would be welcome.

## Contributing

If you find this helpful or intriguing, please contribute! Even the least
helpful bug report, or the most vague suggestion, could be of great value. Pull
requests are also appreciated, though the code is currently in a rather poor
shape.

The project is licensed under the MIT license.
