# Agui Primitives

[![Crates.io](https://img.shields.io/crates/v/agui_primitives?style=flat-square&logo=rust)](https://crates.io/crates/agui_primitives)

## What is Agui Primitives?

Agui Primitives hold basic widgets and core renderables for [`agui`](https://crates.io/crates/agui). Any third party renderer intending to add support for most widgets should only need to support the core renderables in this crate, such as `Drawable`.

### Why are none of these using `#[functional_widget]`?

It is a design decision to keep `agui_primitives` lean and clean. While I find this macro useful, I don't believe it belongs in this crate as its usage and generated code may change in the future. This crate should be stable and resistant to change, so that any render integration drawing its primitives will have long-lasting compatibility.