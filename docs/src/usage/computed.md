# Computed Values

Computed values are an extremely useful construct. They can listen to state and react to it, but will only cause the widget they're defined in to rebuild if their return value changes. Instead of implementing an event listener system, we use computed values to achieve the same effect.

## Motivation and Usage

Sometimes you'll want to listen to some state, but your widget will not always react or otherwise respond to that state change. In cases where this can be guaranteed (and deterministically tested against), we can use computed values to achieve this effect. To demonstrate them, we'll check if the user is currently hovering over the widget by utilizing the `HoverPlugin`:

```rust,noplaypen
# #[functional_widget]
fn hovering_widget(ctx: &WidgetContext) -> BuildResult {
    let is_hovering = ctx.computed(|ctx| {
        // We use `try_use_global` here, since we don't want to test for hovering if the plugin isn't loaded
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

In this case, the computed value's closure will be called whenever the `Hovering` global state is updated, but will only mark the widget for rebuild when it returns a different value. In this case, it will only rebuild when it goes from a non-hover state to a hover state and vice versa.