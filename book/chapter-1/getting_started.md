# Getting started

This book explains how **content_scan** works: the traits you implement, the scanner that drives them, and the result tree you read afterwards.

- [What is Content Scanner](what_is.md) — the problem the crate solves and the identify / analyze / extract model.
- [Installation](installation.md) — how to depend on `content_scan` from crates.io, a git checkout, or a local path.
- [Your first scan](first_scan.md) — a small program that defines a content type, identifies a buffer by magic bytes, and records a result.

You should already be comfortable with Rust traits and Cargo. Later chapters assume that first scan as shared vocabulary: *content*, *identifier*, *analyzer*, *context*, and `ScanResult`.
