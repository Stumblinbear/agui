use std::hash::BuildHasherDefault;

use agui_core::{callback::Callback, unit::Font, widget::Widget};
use agui_elements::{
    inherited::InheritedWidget,
    stateful::{ContextWidgetStateMut, StatefulBuildContext, StatefulWidget, WidgetState},
};
use agui_macros::{InheritedWidget, StatefulWidget};
use rustc_hash::FxHasher;

#[derive(StatefulWidget, Debug)]
pub struct Fonts {
    fonts: im_rc::HashMap<String, Font, BuildHasherDefault<FxHasher>>,

    child: Widget,
}

impl StatefulWidget for Fonts {
    type State = FontsState;

    fn create_state(&self) -> Self::State {
        FontsState {
            initial_fonts: self.fonts.clone(),
            added_fonts: im_rc::HashMap::default(),
        }
    }
}

pub struct FontsState {
    initial_fonts: im_rc::HashMap<String, Font, BuildHasherDefault<FxHasher>>,
    added_fonts: im_rc::HashMap<String, Font, BuildHasherDefault<FxHasher>>,
}

impl WidgetState for FontsState {
    type Widget = Fonts;

    fn build(&mut self, ctx: &mut StatefulBuildContext<Self>) -> Widget {
        AvailableFonts {
            available_fonts: self.added_fonts.clone().union(self.initial_fonts.clone()),

            add_font_cb: ctx.callback(move |ctx, (name, font): (String, Font)| {
                ctx.set_state(move |state| {
                    state.added_fonts.insert(name.to_string(), font);
                });
            }),

            child: ctx.widget.child.clone(),
        }
        .into()
    }
}

#[derive(InheritedWidget)]
pub struct AvailableFonts {
    available_fonts: im_rc::HashMap<String, Font, BuildHasherDefault<FxHasher>>,

    // Allows us to modify the StatefulWidget state
    add_font_cb: Callback<(String, Font)>,

    child: Widget,
}

impl InheritedWidget for AvailableFonts {
    fn child(&self) -> Widget {
        self.child.clone()
    }

    fn should_notify(&self, old_widget: &Self) -> bool {
        self.available_fonts != old_widget.available_fonts
            || self.add_font_cb != old_widget.add_font_cb
    }
}

impl AvailableFonts {
    pub fn get_font(&self, name: &str) -> Option<&Font> {
        self.available_fonts.get(name)
    }

    pub fn add_font(&self, name: String, font: Font) {
        self.add_font_cb.call((name, font));
    }
}

// #[cfg(test)]
// mod tests {
//     use std::cell::RefCell;

//     use agui_core::{engine::widgets::WidgetManager, unit::Font, widget::Widget};
//     use agui_elements::stateless::{StatelessBuildContext, StatelessWidget};
//     use agui_macros::{build, StatelessWidget};

//     use crate::sized_box::SizedBox;

//     use super::{AvailableFonts, Fonts};

//     #[derive(Default)]
//     struct TestResult {
//         retrieved_font: Option<Font>,
//     }

//     thread_local! {
//         static TEST_HOOK: RefCell<TestResult> = RefCell::default();
//     }

//     #[derive(Default, StatelessWidget)]
//     struct TestWidget;

//     impl StatelessWidget for TestWidget {
//         fn build(&self, ctx: &mut StatelessBuildContext<Self>) -> Widget {
//             let available_fonts = ctx
//                 .depend_on_inherited_widget::<AvailableFonts>()
//                 .expect("failed to get available fonts");

//             TEST_HOOK.with(|result| {
//                 result.borrow_mut().retrieved_font =
//                     available_fonts.get_font("test font family").cloned();
//             });

//             SizedBox::square(0.0).into()
//         }
//     }

//     #[test]
//     fn can_retrieve_from_available_fonts() {
//         let mut manager = WidgetManager::default_with_root(build! {
//             <Fonts> {
//                 fonts: im_rc::HashMap::from_iter([(
//                     String::from("test font family"),
//                     Font::from_family("test font family"),
//                 )]),

//                 child: <TestWidget> {},
//             }
//         });

//         TEST_HOOK.with(|result| {
//             assert_ne!(
//                 result.borrow().retrieved_font,
//                 None,
//                 "font was not retrieved"
//             );
//         })
//     }
// }
