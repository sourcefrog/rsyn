# An wire-compatible rsync in Rust

`rsyn` reimplements part of the rsync network protocol in pure Rust.
(It's "rsync with no C.")

This project is copyright by Google, but is not an official Google project.

## Status

`rsyn DIR` prints a recursive listing of the given local directory, by launching
a local rsync subprocess and controlling it over a pair of pipes. (This doesn't
have much external utility but it's a milestone towards implementing the
protocol correctly.)

Intended next steps are:

1. List a directory over rsync+ssh.

1. Copy a directory from rsync into an empty local directory.

1. Copy a directory from rsync into a local directory, skipping already
   up-to-date files, but downloading the full content of missing or out-of-date
   files.

1. Connect to an rsync daemon (`rsync://`): these talk a different introductory
   protocol before starting the main rsync protocol. Support downloads with the
   limitations above.

1. Support incremental rolling-sum and checksum file transfers: the actual
   "rsync algorithm".

Below this point the ordering is less certain but some options are:

1. Act as a server for rsync+ssh. In particular, use this to test rsyn against
   itself, as well as against rsync.

1. Act as an `rsync://` daemon.

1. Support some more selected command line options.

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

And, personally: I contributed to rsync 20 years ago, and it's interesting
to revisit the space with better tools, and with more experience, and see if I
can do better.

## Install

Install rust from <https://rustup.rs/> or elsewhere.

In this directory, run

    cargo build --release

and there will be a binary in `./target/release/rsyn`.

To run the tests (with `cargo test`) you'll need a local copy of rsync, but this
shouldn't be needed for the real system.

## Goals

* Interoperability with original "tridge" rsync, over (first) `rsync+ssh://` or
  (later) `rsync://`.

* Support commonly-used options. Transfer files recursively, with mtimes and
  permissions, with some exclusions.

* Command line compatibility: `rsyn -HLWK` should mean the same as in rsync,
  (if those options are supported at all). rsyn-specific options can be behind
  a `-Z` prefix, which is unused by rsync.

* Demonstrate interoperability by automatically testing rsyn against rsync.
  (Later: against various versions of rsync and maybe also openrsync.)

* No `unsafe` blocks or C FFI. (In the tool itself; obviously the underlying
  Rust libraries have some trusted implementation code.)

* Run on Linux, macOS, Windows, and other Unixes, in roughly that order.
  (Use Rust concurrency structures that are supported everywhere, rather than
  rsync's creative application of Unix-isms.)

* Be safe even against an arbitrarily malicious peer.

* Comparable performance to rsync, in terms of throughput, CPU, and memory.

* Good test coverage, both unit tests and interoperability tests.

* Work correctly on either 32 or 64-bit platforms.

* Clean code. Use Rust type checking to prevent illegal or unsafe states.

Non-goals:

* Support all the _many_ interacting options in rsync. It's grown a lot of them
  over time, some perhaps fairly niche, and they complicate the protocol and
  implementation a lot.

* Improve or evolve the protocol. It's already weird and complicated, and was
  built for a different environment than exists today. Dramatically new
  features, in my view, are better off in a different protocol.

* Support `rsh` or `remsh`! (In theory they can drop in for ssh, but rsync has a
  surprising amount of special case code for things that now seem from a
  different world.)

* Exactly identical behavior.

* Identical text/log output.

## Acknowledgements

Thanks to [Tridge](https://www.samba.org/~tridge/) for his brilliant and
generous mentorship and contributions to open source.

This project would have been far harder without Kristaps Dzonsons's
documentation of the rsync protocol in the
[openrsync](https://github.com/kristapsdz/openrsync) project.

