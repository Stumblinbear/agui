use agui_core::{unit::EdgeInsets, widget::Widget};
use agui_macros::build;
use agui_primitives::{
    flex::{Column, Row},
    padding::Padding,
    sized_box::SizedBox,
};

#[test]
fn multiple_properties() {
    let _widget: Widget = build! {
        <Padding> {
            padding: EdgeInsets::all(10.0),
            child: <Row> { }
        }
    };
}

#[test]
fn alternate_constructors() {
    let _widget: Widget = build! {
        <SizedBox>::new(20.0, 10.0)
    };

    let _widget: Widget = build! {
        <SizedBox>::new(20.0, 10.0) {
            child: <Row> { }
        }
    };
}

#[test]
fn deeply_nested_widgets() {
    let _widget: Widget = build! {
        <Padding> {
            padding: EdgeInsets::all(10.0),

            child: <Column> {
                children: [
                    <Row> { },
                    <Row> { },
                ]
            }
        }
    };
}

#[test]
fn widgets_in_nested_vec_macro() {
    let _widget: Widget = build! {
        <Column> {
            children: vec![
                <Row> { },
                <Row> { }
            ]
        }
    };
}

/// ```compile_fail
/// use agui_core::widget::Widget;
/// use agui_macros::build;
///
/// let _widget: Widget = build! {};
/// ```
pub struct BuildFailsOnEmptyBody;

/// ```compile_fail
/// use agui_core::widget::Widget;
/// use agui_macros::build;
/// use agui_primitives::colored_box::ColoredBox;
///
/// let _widget: Widget = build! {
///     <ColoredBox> {
///        color:
///     }
/// };
/// ```
pub struct BuildFailsOnNoValue;
