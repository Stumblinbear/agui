# Conditional Rendering

Often you'll want to render a widget conditionally. Thankfully, this is extremely straightforward to do, as the `build!` macro supports pretty much all Rust syntax.

```rust,noplaypen
# #[functional_widget]
fn conditional_widget(ctx: &WidgetContext, toggle_something: bool) -> BuildResult {
    build!{
        if toggle_something {
            Drawable {
                layout: Layout {
                    sizing: Sizing::Set { width: 64.0, height 32.0 }
                }
            }
        }else{
            Drawable {
                layout: Layout {
                    sizing: Sizing::Set { width: 32.0, height 64.0 }
                }
            }
        }
    }
}
```

To render based on state is just as easy, just read the state and check against it:

```rust,noplaypen
# #[functional_widget]
fn conditional_widget(ctx: &WidgetContext) -> BuildResult {
    let some_state = ctx.use_state(|| true);

    build! {
        if *some_state.read() {
            Drawable {
                layout: Layout {
                    sizing: Sizing::Set { width: 64.0, height 32.0 }
                }
            }
        }else{
            Drawable {
                layout: Layout {
                    sizing: Sizing::Set { width: 32.0, height 64.0 }
                }
            }
        }
    }
}
```