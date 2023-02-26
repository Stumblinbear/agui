use std::{any::Any, panic::RefUnwindSafe};

use agui_core::{
    render::{CanvasPainter, Paint},
    unit::{Color, FontStyle, Rect},
    widget::{BuildContext, Children, PaintContext, WidgetView},
};
use agui_macros::StatelessWidget;

const ERROR_BORDER: f32 = 5.0;

#[allow(clippy::type_complexity)]
#[derive(StatelessWidget)]
pub struct Fallible<Ok: 'static, Error: 'static> {
    pub func: Box<dyn Fn() -> Result<Ok, Error> + RefUnwindSafe>,

    pub on_ok: Box<dyn Fn(&mut BuildContext<Self>, Ok) -> Children>,
    pub on_err: Option<Box<dyn Fn(&mut BuildContext<Self>, Error) -> Children>>,

    pub on_panic: Option<Box<dyn Fn(&mut BuildContext<Self>, Box<dyn Any + Send>) -> Children>>,
}

impl<Ok: 'static, Error: 'static> PartialEq for Fallible<Ok, Error> {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

impl<Ok: 'static, Error: 'static> Fallible<Ok, Error> {
    pub fn new<F, OkF>(func: F, on_ok: OkF) -> Self
    where
        F: Fn() -> Result<Ok, Error> + RefUnwindSafe + 'static,
        OkF: Fn(&mut BuildContext<Self>, Ok) -> Children + 'static,
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
        ErrF: Fn(&mut BuildContext<Self>, Error) -> Children + 'static,
    {
        self.on_err = Some(Box::new(on_err));

        self
    }

    pub fn on_panic<ErrF>(mut self, on_panic: ErrF) -> Self
    where
        ErrF: Fn(&mut BuildContext<Self>, Box<dyn Any + Send>) -> Children + 'static,
    {
        self.on_panic = Some(Box::new(on_panic));

        self
    }
}

impl<Ok: 'static, Error: 'static> WidgetView for Fallible<Ok, Error> {
    fn build(&self, ctx: &mut BuildContext<Self>) -> Children {
        let result = std::panic::catch_unwind(&self.func);

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

        Children::none()
    }

    fn paint(&self, _ctx: &mut PaintContext<Self>, mut canvas: CanvasPainter) {
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
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use agui_core::{
        manager::WidgetManager,
        query::WidgetQueryExt,
        widget::{BuildContext, Children, WidgetView},
    };
    use agui_macros::StatelessWidget;

    use crate::Fallible;

    #[derive(StatelessWidget, Debug, Default, PartialEq)]
    struct TestWidget {}

    impl WidgetView for TestWidget {
        fn build(&self, _: &mut BuildContext<Self>) -> Children {
            Children::none()
        }
    }

    #[test]
    pub fn on_success() {
        let mut manager = WidgetManager::with_root(
            Fallible::<_, Infallible>::new(
                || Ok(42),
                |_, ok| {
                    assert_eq!(ok, 42, "should have received the correct value");

                    Children::from([TestWidget::default()])
                },
            )
            .on_err(|_, _| {
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
            .on_err(|_, _| Children::from([TestWidget::default()])),
        );

        manager.update();

        assert!(
            manager.query().by_type::<TestWidget>().next().is_some(),
            "widget should have been created"
        );
    }

    #[test]
    pub fn on_panic() {
        std::panic::set_hook(Box::new(|_| {}));

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
            .on_panic(|_, _| Children::from([TestWidget::default()])),
        );

        manager.update();

        assert!(
            manager.query().by_type::<TestWidget>().next().is_some(),
            "widget should have been created"
        );
    }
}
