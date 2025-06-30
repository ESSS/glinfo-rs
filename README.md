glinfo-rs
=========

`glinfo` is a small utility to assess the OpenGL capabilities of the current environment. When called without parameters, the command will create a hidden OpenGL context using Qt and write to standard output contents similar to:

```
LibGL Vendor: NVIDIA Corporation
Renderer: GeForce GTX 980/PCIe/SSE2
Version: 4.6.0 NVIDIA 388.13
Shading Language: 4.60 NVIDIA
```

If the context can't be created, the command will output:

```
ERROR: <ERROR MESSAGE>
```

This is a Rust rewrite of [glinfo](https://github.com/ESSS/glinfo), with the objective of not depending on `Qt` and being statically linked.

The `main.rs` file was heavily inspired by [the glutin window example](https://github.com/rust-windowing/glutin/blob/0433af9018febe0696c485ed9d66c40dad41f2d4/glutin_examples/examples/window.rs#L1-L7).