# Limiting Rebuilds

Rebuilds are at the heart of how `agui` works. Whenever state changes, the widgets in the tree that may change are updated. However, this is a very naïve system and much of the responsibility for limiting these rebuilds is up to the developer (you). We'll go over the problem, and devise a few solutions for it, however ultimately the decision is up to you.

## Builders

Builders are essentially closure-derived widgets. You can create these ad-hoc to limit the scope of rebuilds to a sub-tree of widgets, because they're essentially parent-widgets themselves with their own `WidgetContext`.

```rust,noplaypen
# #[functional_widget]
fn widget_with_builder(ctx: &WidgetContext) -> BuildResult {
    build! {
        Builder::new(move |ctx| {
            // `ctx` is a new `WidgetContext` which will not affect the parent widget

            let state = ctx.use_state(|| 0);

            build! {
                Drawable {
                    layout: Layout {
                        sizing: Sizing::Set { width: 64.0, height 32.0 }
                    }
                }
            }
        }
    }
}
```

## Globals

Another option is utilizing global state.  You can create state, then create sub-widgets which listen to that state, resulting in potentially fewer rebuilds of your application with little effort. However, this has the effect of making it difficult to grok exactly what your application is doing if used incorrectly, and potentially makes limiting the scope of rebuilds more troublesome as your application grows. [You can read more about it here.](state.md).

## Providers and Consumers

Instead of global state, you can use [Providers](../plugins/providers_and_consumers.md). This is an optional plugin which makes state available to its subtree of widgets, however it's not free. Whenever a child needs to access the state, it needs to traverse the tree to find a parent which is providing that state. This is *often* negligable, but as your application grows it may become more pronounced if the children that use the state are deeper in the tree.