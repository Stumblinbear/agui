# Widgets

A widget is anything that _exists_ in the UI tree. It could be a visual element, a management system, or just plain-ol-data. You can find a full list of them in the [Widget Reference](../widgets/widgets.html).

## Primitives

`agui` comes with a set of extremely simple widgets that are referred to as Primitives. These generally cover the most basic renderable ~things~ or other extremely useful widgets that don't offer much opinionated functionality, but are still very useful. Render integrations hook into these to draw the actual visual elements, giving a very small barrier to entry to have all features of `agui`. You can find a full list of them in the [Primitive Widget Reference](../widgets/primitives.html).

## Creating a Widget

A widget consists of two things: its settings and a build function. In Rust, this is just a `struct` with an `impl StatelessWidget`. We're going to start simple, with a basic box on the screen:

```rust,noplaypen
pub struct MyWidget { }

impl StatelessWidget for MyWidget {
    fn build(&self, ctx: &mut StatelessBuildContext) -> BuildResult {
        BuildResult::None
    }
}
```

If you run this... Nothing will happen. Which makes sense, as we don't have any widgets that actually render anything. Lets add one and give it a size.

```rust,noplaypen
impl StatelessWidget for MyWidget {
    fn build(&self, ctx: &mut StatelessBuildContext) -> BuildResult {
        build! {
            Button {
                layout: Layout {
                    sizing: Sizing::Set { width: 64.0, height 32.0 }
                }
            }
        }
    }
}
```

This should render a button on screen that's 64x32 pixels. Pretty swick, if I do say so myself. `Button` is one of our built-in widgets; its implementation is not important just yet, so don't worry about that for now. One important thing to note is clipping is not enabled by default. We'll cover why that is and the implications of that in a [later section](./clipping.md).
