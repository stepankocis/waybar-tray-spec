use ksni::{
    Tray,
    menu::{MenuItem, StandardItem},
};
use tokio::sync::mpsc;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Action {
    Quit,
}

/// Holds only what the tray shows. `main` owns the data source and pushes
/// changes in with `Handle::update`, and only when the visible state changed.
pub struct AppTray {
    actions: mpsc::UnboundedSender<Action>,
}

impl AppTray {
    pub fn new(actions: mpsc::UnboundedSender<Action>) -> Self {
        Self { actions }
    }
}

impl Tray for AppTray {
    fn id(&self) -> String {
        crate::APP.into()
    }

    fn title(&self) -> String {
        crate::APP.into()
    }

    // An icon-theme name costs nothing per update. See SPEC.md, "Assets".
    fn icon_name(&self) -> String {
        "application-x-executable-symbolic".into()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: crate::APP.into(),
            ..Default::default()
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![
            StandardItem {
                label: "Quit".into(),
                icon_name: "application-exit-symbolic".into(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.actions.send(Action::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quit_item_enqueues_quit_action() {
        let (actions, mut events) = mpsc::unbounded_channel();
        let mut tray = AppTray::new(actions);
        let MenuItem::Standard(item) = tray.menu().remove(0) else {
            panic!("expected the Quit item");
        };
        assert_eq!(item.label, "Quit");
        (item.activate)(&mut tray);
        assert_eq!(events.try_recv().unwrap(), Action::Quit);
    }
}
