// /// Server AKA root daemon...

// use anyhow::Result;
// use zbus::dbus_interface;
// use zbus::dbus_proxy;
// use zbus::blocking::{ConnectionBuilder, Connection};
// use crate::app_track::DesktopAppInfo;
// use crate::app_track::TopLevel;
// use crate::events::HammockEvent;
// use crate::hammock::Hammock;

// struct AppHandler {
//     hammock: Hammock,
// }

// #[dbus_interface(name = "dev.calebs.Hammock1.AppHandler")]
// impl AppHandler {
//     fn app_launched(&mut self, app_info: DesktopAppInfo) {
//         trace!("New application launched: {:?}", app_info);
//         match hammock.handle_event(HammockEvent::NewApplication(app_info)) {
//             Ok(_) => {},
//             Err(e) => {
//                 warn!("{}", e);
//             }
//         };
//     }

//     fn new_top_level(&mut self, toplevel: TopLevel) {
//         trace!("New toplevel: {:?}", toplevel);
//         match hammock.handle_event(HammockEvent::NewTopLevel(toplevel)) {
//             Ok(_) => {},
//             Err(e) => {
//                 warn!("{}", e);
//             }
//         };
//     }

//     fn top_level_changed(&mut self, toplevel: TopLevel) {
//         trace!("Toplevel changed: {:?}", toplevel);
//         match hammock.handle_event(HammockEvent::TopLevelChanged(toplevel)) {
//             Ok(_) => {},
//             Err(e) => {
//                 warn!("{}", e);
//             }
//         };
//     }

//     fn top_level_closed(&mut self, toplevel: TopLevel) {
//         trace!("Toplevel closed: {:?}", toplevel);
//         match hammock.handle_event(HammockEvent::TopLevelClosed(toplevel)) {
//             Ok(_) => {},
//             Err(e) => {
//                 warn!("{}", e);
//             }
//         };
//     }
// }

// /// Implements the D-Bus service that the root daemon runs
// pub struct Server {
//     connection: Connection,
// }

// impl Server {
//     pub fn new(hammock: Hammock) -> Result<Self> {
//         let app_handler = AppHandler { hammock };
//         let connection = ConnectionBuilder::session()?
//             .name("dev.calebs.Hammock1.AppHandler")?
//             .serve_at("/dev/calebs/Hammock1/AppHandler", app_handler)?
//             .build()?;

//         Ok(Self {
//             connection,
//         })
//     }
// }
