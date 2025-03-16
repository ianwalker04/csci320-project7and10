## Simple SWIM Template

This project is a template that one can clone in order to set up
the [SWIM Window Interface](https://hendrix-cs.github.io/csci320/projects/bare_metal_editor.html) project from [CSCI 320](https://hendrix-cs.github.io/csci320/).

It demonstrates a simple interactive program that uses both keyboard and timer interrupts. 
When the user types a viewable key, it is added to a string in the middle of the screen. 

The program logic is largely in `lib.rs`, in the `SwimInterface` struct. This design pattern is highly recommended. Keep `main.rs` minimal, and encapsulate the 
application logic a struct that is defined in `lib.rs`. For your own applications, you can
use `SwimInterface` as a starting point without modifying `main.rs` very much.

Prior to building this example, be sure to install the following:
* [Qemu](https://www.qemu.org/)
* Nightly Rust:
  * `rustup override set nightly`
* `llvm-tools-preview`:
  * `rustup component add llvm-tools-preview`
* The [bootimage](https://github.com/rust-osdev/bootimage) tool:
  * `cargo install bootimage`