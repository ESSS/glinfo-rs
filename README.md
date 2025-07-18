# glinfo-rs

`glinfo` is a small utility to assess the OpenGL capabilities of the current environment. When called without parameters, the command outputs:

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

This is a Rust rewrite of [glinfo](https://github.com/ESSS/glinfo), with the aim of not depending on `Qt` and being statically linked.

The `main.rs` file was heavily inspired by [the glutin window example](https://github.com/rust-windowing/glutin/blob/0433af9018febe0696c485ed9d66c40dad41f2d4/glutin_examples/examples/window.rs#L1-L7).

## Releasing

1. Create a new [GitHub Release](https://github.com/ESSS/glinfo-rs/releases/new), using a tag in the format `vX.Y.Z`.
2. Prepare a new internal release by updating the recipe in [conda-recipes](https://github.com/esss/conda-recipes).
