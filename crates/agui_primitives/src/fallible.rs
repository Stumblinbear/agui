use std::{any::Any, panic::RefUnwindSafe};

use agui_core::{
    render::canvas::paint::Paint,
    unit::{Color, FontStyle, Rect},
    widget::{BuildContext, BuildResult, WidgetBuilder},
};

const ERROR_BORDER: f32 = 5.0;

pub struct Fallible<Ok: 'static, Error: 'static> {
    pub func: Box<dyn Fn() -> Result<Ok, Error> + RefUnwindSafe>,

    pub on_ok: Box<dyn Fn(&mut BuildContext<Self>, Ok) -> BuildResult>,
    pub on_err: Option<Box<dyn Fn(&mut BuildContext<Self>, Error) -> BuildResult>>,

    pub on_panic: Option<Box<dyn Fn(&mut BuildContext<Self>, Box<dyn Any + Send>) -> BuildResult>>,
}

impl<Ok: 'static, Error: 'static> Fallible<Ok, Error> {
    pub fn new<F, OkF>(func: F, on_ok: OkF) -> Self
    where
        F: Fn() -> Result<Ok, Error> + RefUnwindSafe + 'static,
        OkF: Fn(&mut BuildContext<Self>, Ok) -> BuildResult + 'static,
    {
        Self {
            func: Box::new(func),

            on_ok: Box::new(on_ok),
            on_err: None,

            on_panic: None,
        }
    }

    pub fn on_err<ErrF>(mut self, on_err: ErrF) -> Self
    where
        ErrF: Fn(&mut BuildContext<Self>, Error) -> BuildResult + 'static,
    {
        self.on_err = Some(Box::new(on_err));

        self
    }

    pub fn on_panic<ErrF>(mut self, on_panic: ErrF) -> Self
    where
        ErrF: Fn(&mut BuildContext<Self>, Box<dyn Any + Send>) -> BuildResult + 'static,
    {
        self.on_panic = Some(Box::new(on_panic));

        self
    }
}

impl<Ok: 'static, Error: 'static> WidgetBuilder for Fallible<Ok, Error> {
    fn build(&self, ctx: &mut BuildContext<Self>) -> BuildResult {
        // Take the old hook then restore it after calling `func`
        let old_hook = std::panic::take_hook();

        std::panic::set_hook(Box::new(|_| {}));

        let result = std::panic::catch_unwind(&self.func);

        std::panic::set_hook(old_hook);

        match result {
            Ok(result) => match result {
                Ok(ok) => return (self.on_ok)(ctx, ok),

                Err(err) => {
                    if let Some(on_err) = &self.on_err {
                        return (on_err)(ctx, err);
                    }
                }
            },

            Err(err) => {
                if let Some(on_panic) = &self.on_panic {
                    return (on_panic)(ctx, err);
                }
            }
        };

        // Create a generic error widget
        ctx.on_draw(|_, mut canvas| {
            let rect: Rect = canvas.get_size().into();

            let red = Paint {
                color: Color::from_rgb((1.0, 0.0, 0.0)),
                ..Paint::default()
            };

            canvas.draw_rect_at(rect, &red);

            canvas.draw_rect_at(
                Rect {
                    x: rect.x + ERROR_BORDER,
                    y: rect.y + ERROR_BORDER,
                    width: rect.width - ERROR_BORDER * 2.0,
                    height: rect.height - ERROR_BORDER * 2.0,
                },
                &Paint {
                    color: Color::from_rgb((1.0, 1.0, 0.0)),
                    ..Paint::default()
                },
            );

            canvas.draw_text(&red, FontStyle::default(), "an error occured");
        });

        BuildResult::empty()
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use agui_core::{
        manager::WidgetManager,
        query::WidgetQueryExt,
        widget::{BuildContext, BuildResult, WidgetBuilder},
    };

    use crate::Fallible;

    #[derive(Debug, Default)]
    struct TestWidget {}

    impl WidgetBuilder for TestWidget {
        fn build(&self, _: &mut BuildContext<Self>) -> BuildResult {
            BuildResult::empty()
        }
    }

    #[test]
    pub fn on_success() {
        let mut manager = WidgetManager::with_root(
            Fallible::<_, Infallible>::new(
                || Ok(42),
                |_, ok| {
                    assert_eq!(ok, 42, "should have received the correct value");

                    BuildResult::with_children([TestWidget::default()])
                },
            )
            .on_err(|_, _| {
                panic!("should not have been called");
            })
            .on_panic(|_, _| {
                panic!("should not have been called");
            }),
        );

        manager.update();

        assert!(
            manager.query().by_type::<TestWidget>().next().is_some(),
            "widget should have been created"
        );
    }

    #[test]
    pub fn on_error() {
        let mut manager = WidgetManager::with_root(
            Fallible::<Infallible, _>::new(
                || Err(13),
                |_, _| {
                    panic!("should not have been called");
                },
            )
            .on_err(|_, _| BuildResult::with_children([TestWidget::default()]))
            .on_panic(|_, _| {
                panic!("should not have been called");
            }),
        );

        manager.update();

        assert!(
            manager.query().by_type::<TestWidget>().next().is_some(),
            "widget should have been created"
        );
    }

    #[test]
    pub fn on_panic() {
        let mut manager = WidgetManager::with_root(
            Fallible::<Infallible, Infallible>::new(
                || panic!("aaaaaaaaaaaaaaaaaaa"),
                |_, _| {
                    panic!("should not have been called");
                },
            )
            .on_err(|_, _| {
                panic!("should not have been called");
            })
            .on_panic(|_, _| BuildResult::with_children([TestWidget::default()])),
        );

        manager.update();

        assert!(
            manager.query().by_type::<TestWidget>().next().is_some(),
            "widget should have been created"
        );
    }
}
