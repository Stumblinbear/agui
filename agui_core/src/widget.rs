use std::{
    any::TypeId,
    fmt::Debug,
    rc::{Rc, Weak},
};

use downcast_rs::{impl_downcast, Downcast};
use generational_arena::Index as GenerationalIndex;

use crate::{
    context::WidgetContext,
    unit::{Key, LayoutType},
};

#[non_exhaustive]
pub enum BuildResult {
    Empty,

    One(WidgetRef),
    Many(Vec<WidgetRef>),

    Error(Box<dyn std::error::Error>),
}

impl BuildResult {
    /// # Errors
    /// 
    /// Returns a boxed error if the widget failed to build correctly.
    pub fn take(self) -> Result<Vec<WidgetRef>, Box<dyn std::error::Error>> {
        match self {
            BuildResult::Empty => Ok(vec![]),
            BuildResult::One(widget) => Ok(vec![widget]),
            BuildResult::Many(widgets) => Ok(widgets),
            BuildResult::Error(err) => Err(err),
        }
    }
}

impl From<WidgetRef> for BuildResult {
    fn from(widget: WidgetRef) -> Self {
        Self::One(widget)
    }
}

impl From<&WidgetRef> for BuildResult {
    fn from(widget: &WidgetRef) -> Self {
        Self::One(WidgetRef::clone(widget))
    }
}

impl From<&Vec<WidgetRef>> for BuildResult {
    fn from(widgets: &Vec<WidgetRef>) -> Self {
        if widgets.is_empty() {
            Self::Empty
        } else {
            Self::Many(widgets.clone())
        }
    }
}

pub trait WidgetType {
    fn get_type_id(&self) -> TypeId;
    fn get_type_name(&self) -> &'static str;
}

pub trait WidgetLayout {
    fn layout_type(&self) -> LayoutType;
}

pub trait WidgetImpl: Downcast {
    fn build(&self, ctx: &WidgetContext) -> BuildResult;
}

pub trait Widget: Debug + WidgetType + WidgetLayout + WidgetImpl {}

impl_downcast!(Widget);

pub enum WidgetRef {
    None,
    Owned(Rc<dyn Widget>),
    Borrowed(Weak<dyn Widget>),
    Keyed {
        owner_id: Option<WidgetID>,
        key: Key,
        widget: Box<WidgetRef>,
    },
}

impl Debug for WidgetRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::Owned(widget) => Debug::fmt(widget, f),
            Self::Borrowed(layout) => match layout.upgrade() {
                Some(widget) => Debug::fmt(&widget, f),
                None => write!(f, "Gone"),
            },
            Self::Keyed { key, widget, .. } => {
                write!(f, "Keyed {{ key: {:?}, widget: {:?} }}", key, widget)
            }
        }
    }
}

impl Default for WidgetRef {
    fn default() -> Self {
        Self::None
    }
}

impl Clone for WidgetRef {
    fn clone(&self) -> Self {
        match self {
            Self::None => Self::None,
            Self::Owned(widget) => Self::Borrowed(Rc::downgrade(widget)),
            Self::Borrowed(widget) => Self::Borrowed(Weak::clone(widget)),
            Self::Keyed { widget, .. } => Self::clone(widget),
        }
    }
}

impl WidgetRef {
    pub fn new<W>(widget: W) -> Self
    where
        W: Widget,
    {
        Self::Owned(Rc::new(widget))
    }

    /// Returns true if the widget is still allocated in memory.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        match self {
            Self::None => false,
            Self::Owned(_) => true,
            Self::Borrowed(weak) => weak.strong_count() != 0,
            Self::Keyed { widget, .. } => widget.is_valid(),
        }
    }

    #[must_use]
    pub fn try_get(&self) -> Option<Rc<dyn Widget>> {
        match self {
            Self::None => None,
            Self::Owned(widget) => Some(Rc::clone(widget)),
            Self::Borrowed(weak) => weak.upgrade(),
            Self::Keyed { widget, .. } => widget.try_get(),
        }
    }

    /// # Panics
    ///
    /// Will panic if the widget no longer exists, or the reference is empty.
    #[must_use]
    pub fn get(&self) -> Rc<dyn Widget> {
        match self {
            Self::None => panic!("widget ref points to nothing"),
            Self::Owned(widget) => Rc::clone(widget),
            Self::Borrowed(weak) => {
                Rc::clone(&weak.upgrade().expect("cannot dereference a dropped widget"))
            }
            Self::Keyed { widget, .. } => widget.get(),
        }
    }

    #[must_use]
    /// # Panics
    ///
    /// Will panic if the widget no longer exists, or the reference is empty.
    pub fn get_type_id(&self) -> TypeId {
        self.get().get_type_id()
    }

    #[must_use]
    /// # Panics
    ///
    /// Will panic if the widget no longer exists, or the reference is empty.
    pub fn get_type_name(&self) -> &'static str {
        self.get().get_type_name()
    }

    /// Returns none of the widget is not the `W` type, or if it has been deallocated.
    #[must_use]
    pub fn try_downcast_ref<W>(&self) -> Option<Rc<W>>
    where
        W: Widget,
    {
        match self.try_get()?.downcast_rc::<W>() {
            Ok(widget) => Some(widget),
            Err(..) => None,
        }
    }

    /// Returns none of the widget is not the `W` type, or if it has been deallocated.
    #[must_use]
    pub fn downcast_ref<W>(&self) -> Rc<W>
    where
        W: Widget,
    {
        self.try_downcast_ref()
            .expect("failed to downcast widget ref")
    }
}

impl From<&Self> for WidgetRef {
    fn from(widget: &Self) -> Self {
        Self::clone(widget)
    }
}

#[allow(clippy::from_over_into)]
impl Into<Vec<Self>> for WidgetRef {
    fn into(self) -> Vec<Self> {
        vec![Self::clone(&self)]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WidgetID(GenerationalIndex, usize);

impl std::fmt::Display for WidgetID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.into_raw_parts().0)
    }
}

impl WidgetID {
    #[must_use]
    pub const fn from(index: GenerationalIndex, depth: usize) -> Self {
        Self(index, depth)
    }

    #[must_use]
    pub const fn id(&self) -> GenerationalIndex {
        self.0
    }

    #[must_use]
    pub const fn depth(&self) -> usize {
        self.1
    }
}

impl Default for WidgetID {
    fn default() -> Self {
        Self(GenerationalIndex::from_raw_parts(0, 0), 0)
    }
}

impl From<usize> for WidgetID {
    fn from(val: usize) -> Self {
        Self(GenerationalIndex::from_raw_parts(val, 0), 0)
    }
}
