# Macros

Before we get too much further, you must understand some of the macros we'll be using in this guide. There are two main ones that `agui` provides.

## The `build!` macro

This is a convenience macro. It's not technically required (in fact it's quite easy to never use it) but it makes our life a bit easier. In exchange for some black-box magic, you get better maintainability and better looking code.

```rust,noplaypen
// Before:
fn build(&self, ctx: &WidgetContext) -> BuildResult {
    BuildResult::One(
        Button {
            child: Drawable {
                layout: Layout {
                    sizing: Sizing::Set { width: 64.0, height 32.0 },
                    ..Layout::default()
                }.into(),
                ..Drawable::default()
            },
            ..Button::default()
        }.into()
    )
}

// After:
fn build(&self, ctx: &WidgetContext) -> BuildResult {
    build!{
        Button {
            child: Drawable {
                layout: Layout {
                    sizing: Sizing::Set { width: 64.0, height 32.0 }
                }
            }
        }
    }
}
```

Doesn't that look nice? Essentially all it is doing is adding `.into()` to your blocks and adding `Default::default()` to your structs. Note that it does make many assumptions, notably that every struct will `#[derive(Default)]`.

## #[functional_widget]

The vast majority of widgets are simple fields followed by a single `build()` function. This means we have room for simplification: why not just make our function our widget? Well alright then. Ask and ye shall receive.

```rust,noplaypen
#[functional_widget]
// The macro will turn `snake_case` into `PascalCase` for the widget name
fn example_widget(ctx: &WidgetContext, layout: Ref<Layout>, child: WidgetRef) -> BuildResult {
    ctx.set_layout(layout);
    
    build!{
        Button {
            child: child
        }
    }
}
```

See? Instead of establishing a struct called `ExampleWidget` with the fields of `layout` and `child`, we can just make a function and tag it with the macro. The `ctx: &WidgetContext` parameter is required, and any following arguments are added as a struct field. Of course, all of this comes with assumptions and potential overhead. Any field used here must implement `Default + Clone` in some form or another, so that the widget may call the `example_widget` function without issue.