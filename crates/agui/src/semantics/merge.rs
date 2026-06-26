use accesskit::{Action, Node, Role};

/// Whether a node of `role` takes its accessible name from its content, per WAI-ARIA "Name From".
pub(super) fn names_from_content(role: Role) -> bool {
    matches!(
        role,
        Role::Button
            | Role::DefaultButton
            | Role::CheckBox
            | Role::RadioButton
            | Role::Switch
            | Role::Link
            | Role::Heading
            | Role::MenuItem
            | Role::MenuItemCheckBox
            | Role::MenuItemRadio
            | Role::ListBoxOption
            | Role::MenuListOption
            | Role::Tab
            | Role::TreeItem
            | Role::Cell
            | Role::GridCell
            | Role::RowHeader
            | Role::ColumnHeader
            | Role::Row
            | Role::Tooltip
            | Role::Label
    )
}

/// The value a control contributes when an ancestor computes its name from content, or `None` when the
/// node is not a value-bearing control. A password input yields nothing.
pub(super) fn control_value(node: &Node) -> Option<String> {
    let is_control = matches!(
        node.role(),
        Role::TextInput
            | Role::MultilineTextInput
            | Role::SearchInput
            | Role::EmailInput
            | Role::NumberInput
            | Role::PhoneNumberInput
            | Role::UrlInput
            | Role::DateInput
            | Role::DateTimeInput
            | Role::WeekInput
            | Role::MonthInput
            | Role::TimeInput
            | Role::ComboBox
            | Role::EditableComboBox
            | Role::Slider
            | Role::SpinButton
            | Role::ScrollBar
            | Role::ProgressIndicator
            | Role::Meter
    );

    if !is_control {
        return None;
    }

    if let Some(value) = node.value() {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_owned());
        }
    }

    node.numeric_value().map(|value| value.to_string())
}

/// The accesskit actions unioned onto a host when a child folds into it. accesskit exposes no action
/// iterator, so its full set is listed here and grows when accesskit defines a new action.
const MERGED_ACTIONS: [Action; 22] = [
    Action::Click,
    Action::Focus,
    Action::Blur,
    Action::Collapse,
    Action::Expand,
    Action::CustomAction,
    Action::Decrement,
    Action::Increment,
    Action::HideTooltip,
    Action::ShowTooltip,
    Action::ReplaceSelectedText,
    Action::ScrollDown,
    Action::ScrollLeft,
    Action::ScrollRight,
    Action::ScrollUp,
    Action::ScrollIntoView,
    Action::ScrollToPoint,
    Action::SetScrollOffset,
    Action::SetTextSelection,
    Action::SetSequentialFocusNavigationStartingPoint,
    Action::SetValue,
    Action::ShowContextMenu,
];

/// Folds an absorbed `child`'s accessibility properties into `host`. Value and numeric value fill an empty
/// host slot. Actions and flags union onto it. The name is computed separately by the walk.
pub(super) fn merge_node(host: &mut Node, child: &Node) {
    if host.value().is_none()
        && let Some(value) = child.value()
    {
        host.set_value(value);
    }
    if host.numeric_value().is_none()
        && let Some(value) = child.numeric_value()
    {
        host.set_numeric_value(value);
    }

    for action in MERGED_ACTIONS {
        if child.supports_action(action) {
            host.add_action(action);
        }
    }

    // accesskit keeps its flag mask private and exposes only per-flag accessors, so the union is listed.
    macro_rules! union_flags {
        ($host:ident, $child:ident, $($getter:ident => $setter:ident),+ $(,)?) => {
            $(if $child.$getter() { $host.$setter(); })+
        };
    }
    union_flags! {
        host, child,
        is_hidden => set_hidden,
        is_multiselectable => set_multiselectable,
        is_required => set_required,
        is_visited => set_visited,
        is_busy => set_busy,
        is_live_atomic => set_live_atomic,
        is_modal => set_modal,
        is_touch_transparent => set_touch_transparent,
        is_read_only => set_read_only,
        is_disabled => set_disabled,
        is_italic => set_italic,
        is_line_breaking_object => set_is_line_breaking_object,
        is_page_breaking_object => set_is_page_breaking_object,
        is_spelling_error => set_is_spelling_error,
        is_grammar_error => set_is_grammar_error,
        is_search_match => set_is_search_match,
        is_suggestion => set_is_suggestion,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_fills_an_empty_value_from_the_child() {
        let mut host = Node::new(Role::Slider);
        let mut child = Node::new(Role::Label);
        child.set_value("50%");

        merge_node(&mut host, &child);

        assert_eq!(host.value(), Some("50%"));
    }

    #[test]
    fn merge_keeps_the_host_value_over_the_child() {
        let mut host = Node::new(Role::Slider);
        host.set_value("host");
        let mut child = Node::new(Role::Label);
        child.set_value("child");

        merge_node(&mut host, &child);

        assert_eq!(host.value(), Some("host"));
    }

    #[test]
    fn merge_unions_actions() {
        let mut host = Node::new(Role::Button);
        let mut child = Node::new(Role::Label);
        child.add_action(Action::Click);

        merge_node(&mut host, &child);

        assert!(host.supports_action(Action::Click));
    }

    #[test]
    fn merge_unions_flags() {
        let mut host = Node::new(Role::Button);
        let mut child = Node::new(Role::Label);
        child.set_disabled();

        merge_node(&mut host, &child);

        assert!(host.is_disabled());
    }
}
