# Computed Values

Computed functions are an extremely useful construct. They can listen to state and react to it, but will only cause the widget they're defined in to rebuild if their return value changes. Instead of implementing an event listener system, we use computed functions to achieve the same effect.

To demonstrate computed functions, we'll check if the user is currently hovering over the widget by utilizing the `HoverPlugin`:

```rust,noplaypen
# #[functional_widget]
fn hovering_widget(ctx: &WidgetContext) -> BuildResult {
    let is_hovering = ctx.computed(|ctx| {
        if let Some(hovering) = ctx.try_use_global::<Hovering>() {
            if hovering.read().is_hovering(ctx) {
                true
            }
        }

        false
    });

    build! {
        if is_hovering {
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

In this case, the computed function will be called whenever the `Hovering` global state is updated, but will only mark the widget for rebuild when it returns a different value. In this case, it will only rebuild when it goes from a non-hover state to a hover state and vice versa.