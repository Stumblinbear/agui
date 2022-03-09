# Hovering

The `HoverPlugin` listens to the `Mouse` global, detecting what widgets the mouse is currently hovering over. This can be used for animations, click detection, and in many other scenarios where mouse interaction is required.

## Motivation and Usage

Without user interaction, a user interface isn't exactly, well, a user interface. There's also great value in reducing exact mouse position events into a single listener that can then "broadcast" coarser events to any widget that cares about them. Imagine if every button in your widget tree was getting updated every single time the mouse position changed—it would cause an unfortunate amount of update calls for an event that, realistically, doesn't need to be that fine-grained.

So, the `HoverPlugin` solves that problem. It consumes mouse positions and writes to the `Hovering` global only when the widget you're hovering over changes, saving CPU and reducing the errors that could occur if every widget was implementing this functionality themselves.

To use it, it's highly recommended to listen to it within a computed value, so your widget is only rebuilt when its hover state changes.

```rust,noplaypen
# #[functional_widget]
fn hovering_widget(ctx: &BuildContext) -> BuildResult {
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
            Button {
                layout: Layout {
                    sizing: Sizing::Set { width: 64.0, height 32.0 }
                }
            }
        }else{
            Button {
                layout: Layout {
                    sizing: Sizing::Set { width: 32.0, height 64.0 }
                }
            }
        }
    }
}
```

No matter how often the `Hovering` global changes, the widget will only be rebuilt when `is_hovering` matches and the function returns `true`. If you didn't use the computed value, the widget would be rebuilt every time the currently hovered widget changed, which wouldn't be good.