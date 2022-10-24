# Getting Started

## Installation

`agui` is available on [crates.io](https://crates.io/crates/agui), Rust's official package repository. Just add this to your `Cargo.toml` file:

```toml
[dependencies]
agui = "0.3" # ensure this is the latest version
```

## Hello, world!

First, you need to select your integration. To get started quickly, we're going to run with `agui_wgpu` since it's the most feature complete.

The smallest program you can start up to render something can be found in `agui_wgpu/examples/hello_world.rs`:

```rust,noplaypen
# fn main() {
    let mut ui = UIProgram::new("agui hello world")?;

    // Register some default behavior
    ui.register_default_plugins();
    ui.register_default_globals();

    let deja_vu = ui.load_font_bytes(include_bytes!("./fonts/DejaVuSans.ttf"))?;

    // Set the root node of the UI
    ui.set_root(build! {
        App {
            child: Text {
                font: deja_vu.styled(),
                text: "Hello, world!"
            }
        }
    });

    // Start the update loop
    ui.run()
# }
```

There's a little initial setup to create the GpuProgram and UI, most of what we care about is loading the font and `ui.set_root`. The `build!` macro will be [explained soon](usage/macros.md). How fonts work will be explained a bit futher in a [later section](usage/fonts.md).

With the above code, you should be left with something like this:

![Hello World](assets/hello_world.png)

Truly remarkable.

Yeah, it's not much to look at, but we'll build on this in future sections to make more elaborate (and reactive!) interfaces.
