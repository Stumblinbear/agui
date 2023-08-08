use std::path::PathBuf;

use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::WindowEvent as WinitWindowEvent,
    event::{
        AxisId, DeviceId, ElementState, Ime, KeyboardInput, ModifiersState, MouseButton,
        MouseScrollDelta, Touch, TouchPhase,
    },
    window::Theme,
};

/// Describes an event from a [`Window`].
#[derive(Debug, PartialEq)]
pub enum WindowEvent {
    /// The size of the window has changed. Contains the client area's new dimensions.
    Resized(PhysicalSize<u32>),

    /// The position of the window has changed. Contains the window's new position.
    ///
    /// ## Platform-specific
    ///
    /// - **iOS / Android / Web / Wayland:** Unsupported.
    Moved(PhysicalPosition<i32>),

    /// The window has been requested to close.
    CloseRequested,

    /// The window has been destroyed.
    Destroyed,

    /// A file has been dropped into the window.
    ///
    /// When the user drops multiple files at once, this event will be emitted for each file
    /// separately.
    DroppedFile(PathBuf),

    /// A file is being hovered over the window.
    ///
    /// When the user hovers multiple files at once, this event will be emitted for each file
    /// separately.
    HoveredFile(PathBuf),

    /// A file was hovered, but has exited the window.
    ///
    /// There will be a single `HoveredFileCancelled` event triggered even if multiple files were
    /// hovered.
    HoveredFileCancelled,

    /// The window received a unicode character.
    ///
    /// See also the [`Ime`](Self::Ime) event for more complex character sequences.
    ReceivedCharacter(char),

    /// The window gained or lost focus.
    ///
    /// The parameter is true if the window has gained focus, and false if it has lost focus.
    Focused(bool),

    /// An event from the keyboard has been received.
    KeyboardInput {
        device_id: DeviceId,
        input: KeyboardInput,
        /// If `true`, the event was generated synthetically by winit
        /// in one of the following circumstances:
        ///
        /// * Synthetic key press events are generated for all keys pressed
        ///   when a window gains focus. Likewise, synthetic key release events
        ///   are generated for all keys pressed when a window goes out of focus.
        ///   ***Currently, this is only functional on X11 and Windows***
        ///
        /// Otherwise, this value is always `false`.
        is_synthetic: bool,
    },

    /// The keyboard modifiers have changed.
    ///
    /// ## Platform-specific
    ///
    /// - **Web:** This API is currently unimplemented on the web. This isn't by design - it's an
    ///   issue, and it should get fixed - but it's the current state of the API.
    ModifiersChanged(ModifiersState),

    /// An event from an input method.
    ///
    /// **Note:** You have to explicitly enable this event using [`Window::set_ime_allowed`].
    ///
    /// ## Platform-specific
    ///
    /// - **iOS / Android / Web / Orbital:** Unsupported.
    Ime(Ime),

    /// The cursor has moved on the window.
    CursorMoved {
        device_id: DeviceId,

        /// (x,y) coords in pixels relative to the top-left corner of the window. Because the range of this data is
        /// limited by the display area and it may have been transformed by the OS to implement effects such as cursor
        /// acceleration, it should not be used to implement non-cursor-like interactions such as 3D camera control.
        position: PhysicalPosition<f64>,
    },

    /// The cursor has entered the window.
    CursorEntered { device_id: DeviceId },

    /// The cursor has left the window.
    CursorLeft { device_id: DeviceId },

    /// A mouse wheel movement or touchpad scroll occurred.
    MouseWheel {
        device_id: DeviceId,
        delta: MouseScrollDelta,
        phase: TouchPhase,
    },

    /// An mouse button press has been received.
    MouseInput {
        device_id: DeviceId,
        state: ElementState,
        button: MouseButton,
    },

    /// Touchpad magnification event with two-finger pinch gesture.
    ///
    /// Positive delta values indicate magnification (zooming in) and
    /// negative delta values indicate shrinking (zooming out).
    ///
    /// ## Platform-specific
    ///
    /// - Only available on **macOS**.
    TouchpadMagnify {
        device_id: DeviceId,
        delta: f64,
        phase: TouchPhase,
    },

    /// Smart magnification event.
    ///
    /// On a Mac, smart magnification is triggered by a double tap with two fingers
    /// on the trackpad and is commonly used to zoom on a certain object
    /// (e.g. a paragraph of a PDF) or (sort of like a toggle) to reset any zoom.
    /// The gesture is also supported in Safari, Pages, etc.
    ///
    /// The event is general enough that its generating gesture is allowed to vary
    /// across platforms. It could also be generated by another device.
    ///
    /// Unfortunatly, neither [Windows](https://support.microsoft.com/en-us/windows/touch-gestures-for-windows-a9d28305-4818-a5df-4e2b-e5590f850741)
    /// nor [Wayland](https://wayland.freedesktop.org/libinput/doc/latest/gestures.html)
    /// support this gesture or any other gesture with the same effect.
    ///
    /// ## Platform-specific
    ///
    /// - Only available on **macOS 10.8** and later.
    SmartMagnify { device_id: DeviceId },

    /// Touchpad rotation event with two-finger rotation gesture.
    ///
    /// Positive delta values indicate rotation counterclockwise and
    /// negative delta values indicate rotation clockwise.
    ///
    /// ## Platform-specific
    ///
    /// - Only available on **macOS**.
    TouchpadRotate {
        device_id: DeviceId,
        delta: f32,
        phase: TouchPhase,
    },

    TouchpadPressure {
        device_id: DeviceId,
        pressure: f32,
        stage: i64,
    },

    /// Motion on some analog axis. May report data redundant to other, more specific events.
    AxisMotion {
        device_id: DeviceId,
        axis: AxisId,
        value: f64,
    },

    /// Touch event has been received
    ///
    /// ## Platform-specific
    ///
    /// - **macOS:** Unsupported.
    Touch(Touch),

    /// The window's scale factor has changed.
    ///
    /// The following user actions can cause DPI changes:
    ///
    /// * Changing the display's resolution.
    /// * Changing the display's scale factor (e.g. in Control Panel on Windows).
    /// * Moving the window to a display with a different scale factor.
    ///
    /// After this event callback has been processed, the window will be resized to whatever value
    /// is pointed to by the `new_inner_size` reference. By default, this will contain the size suggested
    /// by the OS, but it can be changed to any value.
    ///
    /// For more information about DPI in general, see the [`dpi`](crate::dpi) module.
    ScaleFactorChanged {
        scale_factor: f64,
        new_inner_size: PhysicalSize<u32>,
    },

    /// The system window theme has changed.
    ///
    /// Applications might wish to react to this to change the theme of the content of the window
    /// when the system changes the window theme.
    ///
    /// ## Platform-specific
    ///
    /// - **iOS / Android / X11 / Wayland / Orbital:** Unsupported.
    ThemeChanged(Theme),

    /// The window has been occluded (completely hidden from view).
    ///
    /// This is different to window visibility as it depends on whether the window is closed,
    /// minimised, set invisible, or fully occluded by another window.
    ///
    /// Platform-specific behavior:
    /// - **iOS / Android / Web / Wayland / Windows / Orbital:** Unsupported.
    Occluded(bool),
}

impl From<WinitWindowEvent<'_>> for WindowEvent {
    fn from(event: WinitWindowEvent) -> Self {
        match event {
            WinitWindowEvent::Resized(size) => Self::Resized(size),
            WinitWindowEvent::Moved(position) => Self::Moved(position),
            WinitWindowEvent::CloseRequested => Self::CloseRequested,
            WinitWindowEvent::Destroyed => Self::Destroyed,
            WinitWindowEvent::DroppedFile(path) => Self::DroppedFile(path),
            WinitWindowEvent::HoveredFile(path) => Self::HoveredFile(path),
            WinitWindowEvent::HoveredFileCancelled => Self::HoveredFileCancelled,
            WinitWindowEvent::ReceivedCharacter(c) => Self::ReceivedCharacter(c),
            WinitWindowEvent::Focused(focused) => Self::Focused(focused),
            WinitWindowEvent::KeyboardInput {
                device_id,
                input,
                is_synthetic,
            } => Self::KeyboardInput {
                device_id,
                input,
                is_synthetic,
            },
            WinitWindowEvent::ModifiersChanged(modifiers) => Self::ModifiersChanged(modifiers),
            WinitWindowEvent::Ime(ime) => Self::Ime(ime),
            WinitWindowEvent::CursorMoved {
                device_id,
                position,
                ..
            } => Self::CursorMoved {
                device_id,
                position,
            },
            WinitWindowEvent::CursorEntered { device_id } => Self::CursorEntered { device_id },
            WinitWindowEvent::CursorLeft { device_id } => Self::CursorLeft { device_id },
            WinitWindowEvent::MouseWheel {
                device_id,
                delta,
                phase,
                ..
            } => Self::MouseWheel {
                device_id,
                delta,
                phase,
            },
            WinitWindowEvent::MouseInput {
                device_id,
                state,
                button,
                ..
            } => Self::MouseInput {
                device_id,
                state,
                button,
            },
            WinitWindowEvent::TouchpadMagnify {
                device_id,
                delta,
                phase,
                ..
            } => Self::TouchpadMagnify {
                device_id,
                delta,
                phase,
            },
            WinitWindowEvent::SmartMagnify { device_id } => Self::SmartMagnify { device_id },
            WinitWindowEvent::TouchpadRotate {
                device_id,
                delta,
                phase,
                ..
            } => Self::TouchpadRotate {
                device_id,
                delta,
                phase,
            },
            WinitWindowEvent::TouchpadPressure {
                device_id,
                pressure,
                stage,
                ..
            } => Self::TouchpadPressure {
                device_id,
                pressure,
                stage,
            },
            WinitWindowEvent::AxisMotion {
                device_id,
                axis,
                value,
                ..
            } => Self::AxisMotion {
                device_id,
                axis,
                value,
            },
            WinitWindowEvent::Touch(touch) => Self::Touch(touch),
            WinitWindowEvent::ScaleFactorChanged {
                scale_factor,
                new_inner_size,
            } => Self::ScaleFactorChanged {
                scale_factor,
                new_inner_size: *new_inner_size,
            },
            WinitWindowEvent::ThemeChanged(theme) => Self::ThemeChanged(theme),
            WinitWindowEvent::Occluded(occluded) => Self::Occluded(occluded),
        }
    }
}
