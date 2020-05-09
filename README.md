# Wire-compatible rsync client in Rust

[![crates.io](https://img.shields.io/crates/v/rsyn.svg)](https://crates.io/crates/rsyn)
[![docs.rs](https://docs.rs/rsyn/badge.svg)](https://docs.rs/rsyn)
[![Tests](https://github.com/sourcefrog/rsyn/workflows/rust/badge.svg?branch=master)](https://github.com/sourcefrog/rsyn/actions?query=workflow%3Arust)

`rsyn` reimplements part of the rsync network protocol in pure Rust. (It's
"rsync with no C.")

rsyn supports protocol version 27, which is supported by rsync versions from
2004 and later, and by openrsync.

## Install

1. Install Rust from <https://rustup.rs/> or elsewhere.

2. In the rsyn source tree, run

   cargo install --path .

To run the interoperability tests (with `cargo test`) you'll need a copy of
rsync installed.

## Usage

`rsyn DIR` prints a recursive listing of the given local directory, by launching
an rsync subprocess and controlling it over a pair of pipes.

`rsyn USER@HOST:DIR` or `rsyn HOST:DIR` lists a remote directory, connecting to
the rsync server over SSH.

## Roadmap

Intended next steps are:

- [x] List a local directory from a local subprocess.

- [x] List a directory over SSH.

- [ ] Copy a directory from rsync over SSH into an empty local directory.

- [ ] Copy a directory from rsync into a local directory, skipping already
      up-to-date files, but downloading the full content of missing or
      out-of-date files.

- [ ] Connect to an rsync daemon (`rsync://`): these talk a different
      introductory protocol before starting the main rsync protocol. Support
      downloads with the limitations above.

- [ ] Support incremental rolling-sum and checksum file transfers: the actual
      "rsync algorithm".

- [ ] Support the commonly-used `-a` option.

- [ ] Upload a directory to rsync over SSH.

Below this point the ordering is less certain but some options are:

- [ ] Act as a server for rsync+ssh. In particular, use this to test rsyn
      against itself, as well as against rsync.

- [ ] Act as an `rsync://` daemon.

- [ ] Support some more selected command line options.

## Why do this?

rsync does by-hand parsing of a complicated binary network protocol in C.
Although that was a reasonble option in the 90s, today it looks dangerous.
Fuzzers find cases where a malicious peer can crash rsync, and worse may be
possible.

The rsync C code is quite convoluted, with many interacting options and
parameters stored in global variables affecting many different parts of the
control flow, including how structures are encoded and decoded.

rsync is still fairly widely deployed, and does a good job. A safer
interoperable implementation could be useful.

And, personally: I contributed to rsync many years ago, and it's interesting to
revisit the space with better tools, and with more experience, and see if I can
do better.

## Goals

- rsyn will interoperate with recent versions of upstream "tridge" rsync, over
  (first) rsync+ssh or (later) `rsync://`.

- rsyn will support commonly-used rsync options and scenarios. The most
  important are to transfer files recursively, with mtimes and permissions, with
  exclusion patterns.

- rsyn will offer a clean public library Rust API through which transfers can be
  initiated and observed in-process. As is usual for Rust libraries, the API
  is not guaranteed to be stable before 1.0.

- Every command line option in rsyn should have the same meaning as in rsync.

  It's OK if some of the many rsync options are not supported.

  The exception is that rsyn-specific options will start with `--Z` to
  distinguish them and avoid collisions.

- rsyn's test suite should demonstrate interoperability by automatically testing
  rsyn against rsync. (Later versions might demonstrate compatibility against
  various different versions of rsync, and maybe also against openrsync.)

- rsyn should have no `unsafe` blocks. (The underlying Rust libraries have some
  trusted implementation code and link in some C code.)

- rsyn will run on Linux, macOS, Windows, and other Unixes, in both 64-bit and
  (if the OS supports it) 32-bit mode.

  rsyn will use Rust concurrency structures that are supported everywhere,
  rather than rsync's creative application of Unix-isms such as sockets shared
  between multiple processes.

- rsyn should be safe even against an arbitrarily malicious peer.

  In particular, paths received from the peer should be carefully validated to
  prevent
  [path traversal bugs](https://cwe.mitre.org/data/definitions/1219.html).

- rsyn should show comparable performance to rsync, in terms of throughput, CPU,
  and memory.

- rsyn should have good test coverage: both unit tests and interoperability
  tests.

- rsyn code should be clean and understandable Rust code. (The rsync code is now
  quite convoluted.) rsyn will use Rust type checking to prevent illegal or
  unsafe states. Interacting options should be factored into composed types,
  rather than forests of `if` statements.

### Non-goals

- rsyn will not necessarily support every single option and feature in rsync.

  rsync has a lot of options, which (at least in the rsync codebase) interact in
  complicated ways. Some seem to have niche audiences, or to be obsolete, such
  as special support for `rsh` or HP-UX `remsh`.

- rsyn speaks the protocol defined by rsync's implementation, and does not
  aspire to evolve the protocol or to add rsyn-specific upgrades.

  rsync's protocol is already fairly weird and complicated, and was built for a
  different environment than exists today. Dramatically new features, in my
  view, are better off in a clean-slate protocol.

- rsyn need not address security weaknesses in the rsync protocol.

  rsync's block-hashing, file-hashing, and daemon mode authentication use MD4,
  which is not advisable today. This can't be unilaterally changed by rsyn while
  keeping compatibility.

  For sensitive data or writable directories, or really any traffic over
  less-than-fully-trusted networks, I'd strongly recommend running rsync over
  SSH.

- rsyn need not generate exactly identical text/log output.

## Acknowledgements

Thanks to [Tridge](https://www.samba.org/~tridge/) for his brilliant and
generous mentorship and contributions to open source.

This project would have been far harder without Kristaps Dzonsons's
documentation of the rsync protocol in the
[openrsync](https://github.com/kristapsdz/openrsync) project.

## License

[Apache 2.0](LICENSE).

## Contributing

I'd love to accept patches to this project. Please read the
[contribution guidelines](CONTRIBUTING.md) and
[code of conduct](CODE_OF_CONDUCT.md).

## Disclaimer

This is not an official Google project. It is not supported by Google, and
Google specifically disclaims all warranties as to its quality, merchantability,
or fitness for a particular purpose.
