use gtk::prelude::*;
use relm4::{gtk::Widget, prelude::*};

use super::models::{PopoverInit, PopoverItem, PopoverMsg};

/// Reusable popover component with reactive updates
pub struct PopoverComponent {
    title: String,
    items: Vec<PopoverItem>,
    content_box: gtk::Box,
    root: gtk::Popover,
}

#[relm4::component(pub)]
impl SimpleComponent for PopoverComponent {
    type Init = PopoverInit;
    type Input = PopoverMsg;
    type Output = ();

    view! {
        #[root]
        gtk::Popover {}
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // Create the popover content manually
        let main_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(init.spacing)
            .margin_top(init.margin)
            .margin_bottom(init.margin)
            .margin_start(init.margin)
            .margin_end(init.margin)
            .build();

        if let Some(width) = init.width {
            main_box.set_width_request(width);
        }

        // Title label
        let title_label = gtk::Label::builder()
            .label(&init.title)
            .css_classes(vec!["popover-title"])
            .halign(gtk::Align::Start)
            .build();
        main_box.append(&title_label);

        // Content box for dynamic items
        let content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(4)
            .build();
        main_box.append(&content_box);

        // Set the main box as the popover child
        root.set_child(Some(&main_box));

        let model = PopoverComponent {
            title: init.title.clone(),
            items: Vec::new(),
            content_box: content_box.clone(),
            root: root.clone(),
        };

        let widgets = view_output!();

        // Set parent widget for popover (if provided)
        if let Some(parent) = &init.parent {
            root.set_parent(parent);
        }

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            PopoverMsg::UpdateTitle(title) => {
                self.title = title;
                // Update title label if needed
                if let Some(child) = self.root.child() {
                    if let Ok(main_box) = child.downcast::<gtk::Box>() {
                        if let Some(title_label) = main_box.first_child() {
                            if let Ok(label) = title_label.downcast::<gtk::Label>() {
                                label.set_label(&self.title);
                            }
                        }
                    }
                }
            }
            PopoverMsg::UpdateItems(items) => {
                self.items = items;
                self.render_items();
            }
            PopoverMsg::Toggle => {
                if self.root.is_visible() {
                    self.root.popdown();
                } else {
                    self.root.popup();
                }
            }
        }
    }
}

impl PopoverComponent {
    /// Render all items into the content box
    fn render_items(&self) {
        // Clear existing content
        while let Some(child) = self.content_box.first_child() {
            self.content_box.remove(&child);
        }

        // Add new items
        for item in &self.items {
            match item {
                PopoverItem::DetailRow {
                    label,
                    value,
                    value_css,
                } => {
                    let row = Self::create_detail_row(label, value, value_css);
                    self.content_box.append(&row);
                }
                PopoverItem::Separator => {
                    let separator = gtk::Separator::builder()
                        .orientation(gtk::Orientation::Horizontal)
                        .margin_top(8)
                        .margin_bottom(8)
                        .build();
                    self.content_box.append(&separator);
                }
                PopoverItem::Custom(widget) => {
                    self.content_box.append(widget);
                }
            }
        }
    }

    /// Create a two-column detail row widget
    fn create_detail_row(label: &str, value: &str, value_css: &str) -> gtk::Box {
        let row = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .css_classes(vec!["detail-row"])
            .build();

        let label_widget = gtk::Label::builder()
            .label(label)
            .css_classes(vec!["detail-label"])
            .halign(gtk::Align::Start)
            .hexpand(true)
            .build();

        let value_widget = gtk::Label::builder()
            .label(value)
            .css_classes(vec!["detail-value", value_css])
            .halign(gtk::Align::End)
            .build();

        row.append(&label_widget);
        row.append(&value_widget);
        row
    }

    pub fn set_parent(&self, parent: &impl IsA<Widget>) {
        self.root.set_parent(parent);
    }
}
