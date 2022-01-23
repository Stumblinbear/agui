# State

Widgets may contain their own, self contained state, which persists across rebuilds (usually—more on that in a minute). Whenever the state changes, the widget's `build()` function will be re-invoked, and its children will be rebuilt. This is your main tool for implementing a user interface that can react to user events. Lets write up a quick counter example to demonstrate this:

```rust,noplaypen
// Requires the `HoverPlugin` for the `Button` to function properly, so
// make sure you call `ui.init_plugin(HoveringPlugin::default);`

# #[functional_widget]
fn counter_widget(ctx: &WidgetContext, font: FontId) -> BuildResult {
    let num = ctx.use_state(|| 0);

    build! {
        Column {
            children: [
                Text::is(font, 32.0, format!("clicked: {} times", num.read())),
                Button {
                    child: Padding {
                        padding: Margin::All(10.0.into()),
                        child: Text::is(font, 32.0, "A Button".into())
                    },
                    on_pressed: Callback::from(move |()| {
                        *num.write() += 1;
                    })
                }
            ]
        }
    }
}
```

Any time you use `num.write()`, it will cause any listening widgets to be rebuilt on the next update, so ensure you only call it when you *actually* change something. The first time `ctx.use_state(|| 0)` is used, the closure within the method is called to initialize the state. In this case, it will be initialized to zero. On subsequent rebuilds of the widget, the previous state that it was in will be persisted. However, this only applies if the parent of the widget is not rebuilt.

If you want to create state, but not listen to changes to it, you can instead use `ctx.init_state`. This is useful for widgets that manage state that children respond to, but state that itself doesn't react to. If you were to use that above, instead of `use_state`, the value would have changed internally, but you wouldn't see any change to the UI.

## Globals

A global acts much the same way as state, but it exists once in your application and is shared amongst all widgets, no matter how deep they are in your tree.

```rust,noplaypen
# #[functional_widget]
fn widget_with_global(ctx: &WidgetContext) -> BuildResult {
    let state = ctx.use_global(|| 0);

    build! {
        Drawable {
            layout: Layout {
                sizing: Sizing::Set { width: 64.0, height 32.0 }
            }
        }
    }
}
```

In this case, `use_global` will fetch the global state or initialize it to zero if it does not already exist.

## Parental Rebuilds

When a widget is rebuilt, its state is persisted. However, any children it has will be reinitialized, meaning *their* state will be destroyed. This means you need to be careful in how you structure your interface to reduce rebuilds, and to work around this limitation. For more information on this, you can see [Limiting Rebuilds](limiting-rebuilds.md).
