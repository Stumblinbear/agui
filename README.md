<div align="center">
    <a href="https://github.com/stumblinbear/agpu">
        <img src=".github/logo.webp" alt="Logo" width="128" />
    </a>
    <br />
    An advanced, reactive UI library for Rust
    <br />
    <a href="https://github.com/stumblinbear/agui/issues/new?assignees=&labels=bug&template=BUG_REPORT.md&title=bug%3A+">
        Report a Bug
    </a>
    ·
    <a href="https://github.com/stumblinbear/agui/issues/new?assignees=&labels=enhancement&template=FEATURE_REQUEST.md&title=feat%3A+">
        Request a Feature
    </a>
    .
    <a href="https://github.com/stumblinbear/agui/discussions">
        Ask a Question
    </a>
    <br />
    <br />
    <a href="https://github.com/stumblinbear/agui/actions/workflows/rust.yml">
        <img src="https://img.shields.io/github/workflow/status/stumblinbear/agui/CI?style=flat-square">
    </a>
    <a href="https://crates.io/crates/agui">
        <img src="https://img.shields.io/crates/v/agui?style=flat-square&logo=rust">
    </a>
    <a href="https://docs.rs/agui">
        <img src="https://img.shields.io/docsrs/agui?style=flat-square">
    </a>
</div>

---

## What is agui?

Agui is an advanced reactive GUI project for Rust, inspired by Flutter and taking some concepts from other related UI systems.

## WARNING

Agui is very much still in heavy active development. The API will likely change, and it has yet to go under rigorous testing. However, that's not to say it's not ready for moderate use.

# 🛠️ Installation

Agui is available on [crates.io](https://crates.io/crates/agui), Rust's official package repository. Just add this to your `Cargo.toml` file:

```toml
[dependencies]
agui = "0.1" # ensure this is the latest version
```

# 🚀 Usage

Docs for `agui` are under development, however you can check the `agui_agpu/examples` directory for basic setup, and `agui_widgets` for many examples on widget creation.

## Creating new widgets

Currently, widgets are created using a `Widget` derive macro, and by implementing the `WidgetBuilder` trait.

```rust
#[derive(Default, Widget)]
// The default layout type is "column" but we want it to be a "row" instead.
#[widget(layout = "row")]
pub struct MyWidget {
    // We can define parameters, here.
    pub layout: Ref<Layout>,
}

impl WidgetBuilder for MyWidget {
    // Widgets can return nothing, one or more children, or an error. BuildResult is the enum we use to cover those possibilities.
    fn build(&self, ctx: &WidgetContext) -> BuildResult {
        // `ctx.set_layout` is what we use to define this widget's layout parameters.
        ctx.set_layout(Ref::clone(&self.layout));

        build! {
            Button { }
        }
    }
}
```

## What's `build!`?

The `build!` macro makes it significantly cleaner and easier to init new widgets. All it does is initialize unset fields in a struct to their `Default::default()`, and add `.into()` to the struct itself.

```rust
// It allows us to turn this:

fn build(&self, ctx: &WidgetContext) -> BuildResult {
    BuildResult::One(
        Button {
            layout: Layout::default(),
            color: Color::default(),
            child: Text {
                text: String::from("A Button")
            }
        }
    )
}

// Into this:

use agui::macros::build;

fn build(&self, ctx: &WidgetContext) -> BuildResult {
    build!{
        Button {
            child: Text {
                text: String::from("A Button")
            }
        }
    }
}
```

A more complex widget implementation (featuring global state and computed values) can be seen in [the Button widget](agui_widgets/src/button.rs).

**Functional Widgets are coming soon, which will make creating them even easier.**

# 🤝 Contributing

Contributions are encouraged, and very welcome. Feel free to check the [issues page](https://github.com/stumblinbear/agui/issues) if you wish to do so!

Please go through existing issues and pull requests to check if somebody else is already working on it. Also, make sure to run `cargo test` before you commit your changes!