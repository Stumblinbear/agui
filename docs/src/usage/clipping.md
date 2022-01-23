# Clipping

Clipping gets its own section because, while it's useful, it comes with some very important drawbacks. One thing to keep in mind is that child widgets are not clipped by default.

## Motivation and Usage

This part of the docs are currently [unwritten](https://github.com/Stumblinbear/agui/blob/master/docs/src/reference/animations.md). If you wish to contribute, feel free to make a pull request.

<!-- There are cases where a widget will want to *absolutely ensure* child widgets do not render outside of its bounds. Think a circular profile picture or a scroll area. In these cases, you need to set the clipping bounds of the widget using `ctx.set_clipping`. This function takes in a `Shape` and will instruct the renderer to ensure no children are drawn outside of those bounds.

```rust,noplaypen
# #[functional_widget]
fn clipped_widget(ctx: &WidgetContext, child: WidgetRef) -> BuildResult {
    ctx.set_clipping(
        Shape::RoundedRect {
            top_left: 4.0,
            top_right: 4.0,
            bottom_right: 4.0,
            bottom_left: 4.0,
        }
        .into(),
    );

    child.into()
}
```

## Consequences

This is not a cheap operation, so care should be taken to ensure this is only used as *absolutely necessary*. Each widget that clips its children will cause a new render pass to be created (a non-negligable operation), which can add up quickly. -->