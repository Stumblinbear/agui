# Keyed Widgets

Occasionally you'll run into a situation where you are *absolutely positive* that child widgets will not react to a parent rebuilding. In these cases, keyed widgets come to the rescue. However, they must be used carefully as they may cause logical inconsistencies or non-deterministic behavior if used incorrectly. Another way to think of them are as "cache keys" instead of just keys.

There are two *types* of keys, at the time of writing:

- **Local**: These are scoped to the widget creating the key, and don't propagate down to children.
- **Global**: These are scoped to the entire application, meaning widgets can move around the tree in certain situations.

There's also a third "type", and that's unique keys (which are really just Global keys). These are explained in a bit more detail below.

# Motivation and Usage

Imagine a situation where you have a widget which only provides layout sizing to its children, and reacts to some global state to set that size. This is the exact situation for the `App` widget:

```rust,noplaypen
#[functional_widget]
fn app(ctx: &BuildContext, child: WidgetRef) -> BuildResult {
    // Fetch the `WindowSize` global, which contains app sizing information
    let window_size = ctx.use_global(WindowSize::default);

    // Set the sizing of this widget, so children may take up the entirety of the app size
    ctx.set_layout(build! {
        Layout {
            sizing: Sizing::Axis {
                width: Units::Pixels(window_size.width),
                height: Units::Pixels(window_size.height),
            }
        }
    });

    // Return the child
    child.into()
}
```

In this case, this would cause child widgets to be rebuilt whenever the app size changes (think window resizing), incurring potentially expensive tree operations and absolute state loss of all children. In this case, we can guarantee that the widget rebuilding won't change the children in any way. *Given these guarantees*, we can use a Key. Simply change the last line to this:

```rust,noplaypen
// `Key::single()` is a helper alias of `Key::Local(0)`
ctx.key(Key::single(), child).into()
```

This will cause the child widget to be cached and reused when the parent `App` widget is rebuilt, instead of recreating it from scratch. The exact meaning of this line will be elaborated on in a moment.

## Limitations

Keys are limited in how they function, and it's important to understand these limitations to use them effectively.

Most notably, only one key with the same value may exist in any given scope. Two local keys with the same value may not exist in a single widget, but two separate widgets (even if one is a child of the other) may share exact values without issue. The same cannot be said for global keys, which must exist once in the entirety of your application. If you break this convention, your application *will* `panic!` if two keys clash during a rebuild.

Additionally, keys only function if the widget is removed and re-added to the tree within a single update. If you have a keyed widget that gets removed from the tree, but then gets added back into the tree in a subsequent `update()`, then it will be regenerated anew.

## Local Keys

Local keys are just as the name implies: they're local to the widget that defined them. If your widget can function using local keys, it's highly recommended to use them over any other since they come with the fewest strings attached.

```rust,noplaypen
Key::single() // If the widget only contains one keyed widget

// Otherwise, we use

Key::local(hashable_value)

// or

Key::Local(u64)
```

## Global Keys

Global keys, just like globals, can be used throughout the entirety of your application. They **must** follow the convention of each key being entirely unique to your application, and should not be used by third party widget crates. Third parties should use unique keys, or accept a key as a parameter.

```rust,noplaypen
Key::global(hashable_value)

// or

Key::Global(u64)
```

### Unique Keys

Unique keys are global keys, but they're designed to be passed as an argument to child widgets. These are non-deterministic, and a new one should be generated each time one is created, using `Key::unique()`.
