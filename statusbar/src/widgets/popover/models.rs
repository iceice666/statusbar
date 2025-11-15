use relm4::gtk;

/// Initialization data for PopoverComponent
#[derive(Clone)]
pub struct PopoverInit {
    pub parent: Option<gtk::Widget>,
    pub title: String,
    pub margin: i32,
    pub spacing: i32,
    pub width: Option<i32>,
}

/// Items that can be displayed in the popover
#[derive(Clone, Debug)]
pub enum PopoverItem {
    /// Two-column detail row with label and value
    DetailRow {
        label: String,
        value: String,
        value_css: String,
    },
    /// Horizontal separator line
    Separator,
    /// Custom widget for special cases (e.g., Resources config section)
    Custom(gtk::Widget),
}

/// Messages for PopoverComponent
#[derive(Debug, Clone)]
pub enum PopoverMsg {
    /// Update the popover title
    UpdateTitle(String),
    /// Update all popover items
    UpdateItems(Vec<PopoverItem>),
    /// Toggle popover visibility
    Toggle,
}
