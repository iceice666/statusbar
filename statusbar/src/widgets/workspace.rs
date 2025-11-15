use gtk::prelude::*;
use niri_ipc::{Event, Request, Response, Workspace as NiriWorkspace, socket::Socket};
use relm4::prelude::*;
use relm4::factory::FactoryVecDeque;
use std::thread;

// Workspace button factory component
#[derive(Debug, Clone)]
pub struct WorkspaceButton {
    workspace: NiriWorkspace,
}

#[derive(Debug)]
pub enum WorkspaceButtonMsg {
    Clicked,
}

#[relm4::factory(pub)]
impl FactoryComponent for WorkspaceButton {
    type Init = NiriWorkspace;
    type Input = WorkspaceButtonMsg;
    type Output = u64;
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        #[root]
        gtk::Button {
            set_label: &self.workspace.idx.to_string(),

            #[watch]
            set_css_classes: &Self::compute_css_classes(&self.workspace),

            connect_clicked => WorkspaceButtonMsg::Clicked,
        }
    }

    fn init_model(workspace: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self { workspace }
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        match msg {
            WorkspaceButtonMsg::Clicked => {
                let _ = sender.output(self.workspace.id);
            }
        }
    }
}

impl WorkspaceButton {
    fn compute_css_classes(workspace: &NiriWorkspace) -> Vec<&'static str> {
        let mut classes = vec!["workspace-button"];

        if workspace.is_focused {
            classes.push("workspace-focused");
        } else if workspace.is_active {
            classes.push("workspace-active");
        } else if workspace.active_window_id.is_some() {
            classes.push("workspace-occupied");
        } else {
            classes.push("workspace-empty");
        }

        classes
    }
}

// Main workspace widget
pub struct WorkspaceWidget {
    workspace_buttons: FactoryVecDeque<WorkspaceButton>,
}

#[derive(Debug, Clone)]
pub enum WorkspaceMsg {
    UpdateWorkspaces(Vec<NiriWorkspace>),
    SwitchWorkspace(u64),
}

#[relm4::component(pub)]
impl SimpleComponent for WorkspaceWidget {
    type Init = ();
    type Input = WorkspaceMsg;
    type Output = ();

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 4,
            set_css_classes: &["workspace-widget", "widget"],

            #[local_ref]
            workspace_buttons_box -> gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 0,
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let workspace_buttons = FactoryVecDeque::builder()
            .launch(gtk::Box::default())
            .forward(sender.input_sender(), |output| {
                WorkspaceMsg::SwitchWorkspace(output)
            });

        let model = WorkspaceWidget {
            workspace_buttons,
        };

        let workspace_buttons_box = model.workspace_buttons.widget();
        let widgets = view_output!();

        // Spawn thread to listen for niri events
        let sender_clone = sender.clone();
        thread::spawn(move || {
            if let Err(e) = Self::listen_niri_events(sender_clone) {
                eprintln!("Niri IPC error: {}", e);
            }
        });

        // Request initial workspace state
        thread::spawn(move || {
            if let Ok(workspaces) = Self::get_workspaces() {
                sender.input(WorkspaceMsg::UpdateWorkspaces(workspaces));
            }
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            WorkspaceMsg::UpdateWorkspaces(workspaces) => {
                let mut guard = self.workspace_buttons.guard();
                guard.clear();
                for workspace in workspaces {
                    guard.push_back(workspace);
                }
            }
            WorkspaceMsg::SwitchWorkspace(id) => {
                if let Err(e) = Self::switch_to_workspace(id) {
                    eprintln!("Failed to switch workspace: {}", e);
                }
            }
        }
    }
}

impl WorkspaceWidget {
    fn listen_niri_events(sender: ComponentSender<Self>) -> Result<(), String> {
        let mut socket = Socket::connect().map_err(|e| e.to_string())?;

        let reply = socket
            .send(Request::EventStream)
            .map_err(|e| e.to_string())?;

        if matches!(reply, Ok(Response::Handled)) {
            let mut read_event = socket.read_events();

            while let Ok(event) = read_event() {
                match event {
                    Event::WorkspacesChanged { workspaces } => {
                        let mut workspace_list: Vec<NiriWorkspace> = workspaces;
                        workspace_list.sort_by_key(|w| w.idx);
                        sender.input(WorkspaceMsg::UpdateWorkspaces(workspace_list));
                    }
                    Event::WorkspaceActivated { id: _, focused: _ } => {
                        // Request fresh workspace state on activation
                        if let Ok(workspaces) = Self::get_workspaces() {
                            sender.input(WorkspaceMsg::UpdateWorkspaces(workspaces));
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    fn get_workspaces() -> Result<Vec<NiriWorkspace>, String> {
        let mut socket = Socket::connect().map_err(|e| e.to_string())?;
        let reply = socket
            .send(Request::Workspaces)
            .map_err(|e| e.to_string())?;

        match reply {
            Ok(Response::Workspaces(workspaces)) => {
                let mut workspace_list: Vec<NiriWorkspace> = workspaces.into_iter().collect();
                workspace_list.sort_by_key(|w| w.idx);
                Ok(workspace_list)
            }
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(e),
        }
    }

    fn switch_to_workspace(id: u64) -> Result<(), String> {
        let mut socket = Socket::connect().map_err(|e| e.to_string())?;
        let request = Request::Action(niri_ipc::Action::FocusWorkspace {
            reference: niri_ipc::WorkspaceReferenceArg::Id(id),
        });

        let _ = socket.send(request).map_err(|e| e.to_string())?;
        Ok(())
    }
}
