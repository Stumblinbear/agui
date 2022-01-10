# Providers & Consumers

The provider pattern can provide *(hah!)* some much needed structure to your state. That's why `agui` provides *(hah)* this plugin in `agui_widgets`, to provide *(hah—okay, I'll stop)* some standardization to this extremely useful pattern.

## Motivation and Usage

As your application grows, using globals can get messy. It becomes increasingly difficult to understand where state is mutated, and what widgets are listening to said state. It also makes your code significantly less reusable. In contrast to widget state or globals, the provider pattern acts as the middleground between these two possibilities: it makes state available *only to children* of a widget, rather than to the entirety of the widget tree.

A perfect example of where this pattern shines is in `Theme`, of which we cover in more detail [in this section](../reference/globals/themes.html). Themes are generally "global" (in the sense that you want everything to look the same), but sometimes you want the style of your widgets to be different in a certain part of your application. The widgets `agui` provides use `Theme` extensively, making it simple for you to style your application however you desire.

### Providing State

To provide some state, we just need to provide it somewhere in the widget tree:

```rust,noplaypen
use agui::widgets::plugins::provider::ProviderExt;

# #[functional_widget]
fn provider_widget(ctx: &WidgetContext, child: WidgetRef) -> BuildResult {
    // The generic isn't required, here; it's just used for clarity.
    let some_number = ctx.use_state::<usize, _>(|| 0);

    // `ProviderExt` gives an easy-to-use extension trait onto `Notify` (which is what `use_state` and `init_state` return).
    some_number.provide(ctx);

    // This child, and all children within it, will now have access to `some_number`, as long as they Consume it properly.
    child.into()
}
```

### Consuming State

Consuming from a provided state is also extremely simple; the main difference in usage between this pattern and globals is that `use_global` will init non-existent values, but the Provider pattern will return `None` if it doesn't exist in the tree.

```rust,noplaypen
use agui::widgets::plugins::provider::ConsumerExt;

# #[functional_widget]
fn provider_widget(ctx: &WidgetContext, child: WidgetRef) -> BuildResult {
    // This will be ignored by `ctx.consume` since it's not provided.
    let some_number = ctx.use_state::<usize, _>(|| 0);

    // `ConsumerExt` gives an easy-to-use extension trait onto `WidgetContext`.
    if let Some(some_number) = ctx.consume::<usize>() {
        // Use `some_number`, here.
    }

    BuildResult::None
}
```