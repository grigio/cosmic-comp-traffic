// SPDX-License-Identifier: GPL-3.0-only

//! `zwlr_foreign_toplevel_manager_v1` (wlr-foreign-toplevel-management).
//!
//! `ext_foreign_toplevel_list_v1` only carries title/app-id, but docks and
//! taskbars (waybar's `wlr/taskbar`, sfwbar, noctalia, ...) speak this protocol
//! to learn about toplevel state and to control it.
//!
//! The state is mirrored from the windows already tracked by
//! [`super::toplevel_info::ToplevelInfoState`], and the requests are forwarded
//! to [`super::toplevel_management::ToplevelManagementHandler`].

use std::{collections::HashSet, marker::PhantomData, sync::Mutex};

use smithay::{
    input::Seat,
    output::Output,
    reexports::{
        wayland_protocols_wlr::foreign_toplevel::v1::server::{
            zwlr_foreign_toplevel_handle_v1::{self, ZwlrForeignToplevelHandleV1},
            zwlr_foreign_toplevel_manager_v1::{self, ZwlrForeignToplevelManagerV1},
        },
        wayland_server::{
            Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource,
            backend::{ClientId, GlobalId},
            protocol::wl_output::WlOutput,
        },
    },
};

use super::{
    toplevel_info::{ToplevelInfoHandler, ToplevelState, Window},
    toplevel_management::{ManagementWindow, ToplevelManagementHandler},
};

pub const WLR_FOREIGN_TOPLEVEL_MANAGER_VERSION: u32 = 3;

/// Per-manager (i.e. per-client) bookkeeping: the handles handed out to that
/// client. The window each handle belongs to lives in the handle data.
#[derive(Default)]
pub struct WlrManagerData {
    handles: Mutex<Vec<ZwlrForeignToplevelHandleV1>>,
}

pub struct WlrHandleStateInner<W: Window> {
    window: Option<W>,
    title: String,
    app_id: String,
    /// Raw `zwlr_foreign_toplevel_handle_v1::State` values last sent.
    states: Option<Vec<u32>>,
    wl_outputs: HashSet<WlOutput>,
}

impl<W: Window> WlrHandleStateInner<W> {
    fn new(window: W) -> WlrHandleState<W> {
        Mutex::new(WlrHandleStateInner {
            window: Some(window),
            title: String::new(),
            app_id: String::new(),
            states: None,
            wl_outputs: HashSet::new(),
        })
    }
}

pub type WlrHandleState<W> = Mutex<WlrHandleStateInner<W>>;

pub struct WlrForeignToplevelGlobalData {
    filter: Box<dyn for<'a> Fn(&'a Client) -> bool + Send + Sync>,
}

#[derive(Debug)]
pub struct WlrForeignToplevelState<D, W: Window> {
    dh: DisplayHandle,
    instances: Vec<ZwlrForeignToplevelManagerV1>,
    global: GlobalId,
    _dispatch_data: PhantomData<(D, W)>,
}

pub trait WlrForeignToplevelHandler: ToplevelInfoHandler + Sized {
    fn wlr_foreign_toplevel_state(&mut self) -> &mut WlrForeignToplevelState<Self, Self::Window>;
}

impl<D, W> WlrForeignToplevelState<D, W>
where
    D: GlobalDispatch<ZwlrForeignToplevelManagerV1, WlrForeignToplevelGlobalData>
        + Dispatch<ZwlrForeignToplevelManagerV1, WlrManagerData>
        + Dispatch<ZwlrForeignToplevelHandleV1, WlrHandleState<W>>
        + ToplevelInfoHandler<Window = W>
        + 'static,
    W: Window + 'static,
{
    pub fn new<F>(dh: &DisplayHandle, client_filter: F) -> WlrForeignToplevelState<D, W>
    where
        F: for<'a> Fn(&'a Client) -> bool + Send + Sync + 'static,
    {
        let global = dh.create_global::<D, ZwlrForeignToplevelManagerV1, _>(
            WLR_FOREIGN_TOPLEVEL_MANAGER_VERSION,
            WlrForeignToplevelGlobalData {
                filter: Box::new(client_filter),
            },
        );
        WlrForeignToplevelState {
            dh: dh.clone(),
            instances: Vec::new(),
            global,
            _dispatch_data: PhantomData,
        }
    }

    pub fn global_id(&self) -> GlobalId {
        self.global.clone()
    }

    /// Mirrors the current toplevel state to every bound manager.
    ///
    /// Cheap when nothing changed: the per-handle cache is compared before
    /// sending anything, and `done` is only sent for handles that did change.
    pub fn refresh(&mut self, toplevels: &[W]) {
        self.instances.retain(|instance| instance.is_alive());

        for instance in &self.instances {
            let Some(manager) = instance.data::<WlrManagerData>() else {
                continue;
            };
            let Ok(client) = self.dh.get_client(instance.id()) else {
                continue;
            };
            let version = instance.version().min(WLR_FOREIGN_TOPLEVEL_MANAGER_VERSION);

            let mut handles = manager.handles.lock().unwrap();

            // Toplevels that are gone get `closed` and stop being tracked. A
            // window can be unmapped while its surface is still alive, so
            // being in `toplevels` matters as much as being alive.
            handles.retain(|handle| {
                let alive = handle.data::<WlrHandleState<W>>().is_some_and(|state| {
                    state.lock().unwrap().window.as_ref().is_some_and(|window| {
                        window.alive() && toplevels.iter().any(|toplevel| toplevel == window)
                    })
                });
                if !alive && handle.is_alive() {
                    handle.closed();
                }
                alive
            });

            for window in toplevels {
                let existing = handles
                    .iter()
                    .find(|handle| {
                        handle.data::<WlrHandleState<W>>().is_some_and(|state| {
                            state.lock().unwrap().window.as_ref() == Some(window)
                        })
                    })
                    .cloned();

                let handle = match existing {
                    Some(handle) => handle,
                    None => {
                        let Ok(handle) = client
                            .create_resource::<ZwlrForeignToplevelHandleV1, _, D>(
                                &self.dh,
                                version,
                                WlrHandleStateInner::new(window.clone()),
                            )
                        else {
                            continue;
                        };
                        instance.toplevel(&handle);
                        handles.push(handle.clone());
                        handle
                    }
                };

                if update_handle(&handle, window, &client) && handle.is_alive() {
                    handle.done();
                }
            }
        }
    }
}

/// Sends the events needed to bring `handle` up to date with `window`.
/// Returns whether anything was sent (so the caller can send `done`).
fn update_handle<W>(handle: &ZwlrForeignToplevelHandleV1, window: &W, client: &Client) -> bool
where
    W: Window + 'static,
{
    let Some(state) = handle.data::<WlrHandleState<W>>() else {
        return false;
    };
    let mut state = state.lock().unwrap();
    let mut changed = false;

    let title = window.title();
    if state.title != title {
        state.title = title.clone();
        handle.title(title);
        changed = true;
    }

    let app_id = window.app_id();
    if state.app_id != app_id {
        state.app_id = app_id.clone();
        handle.app_id(app_id);
        changed = true;
    }

    let mut states = Vec::with_capacity(4);
    if window.is_maximized() {
        states.push(zwlr_foreign_toplevel_handle_v1::State::Maximized as u32);
    }
    if window.is_minimized() {
        states.push(zwlr_foreign_toplevel_handle_v1::State::Minimized as u32);
    }
    if window.is_activated() {
        states.push(zwlr_foreign_toplevel_handle_v1::State::Activated as u32);
    }
    if window.is_fullscreen() {
        states.push(zwlr_foreign_toplevel_handle_v1::State::Fullscreen as u32);
    }
    if state.states.as_ref() != Some(&states) {
        state.states = Some(states.clone());
        handle.state(
            states
                .iter()
                .flat_map(|state| state.to_ne_bytes())
                .collect::<Vec<u8>>(),
        );
        changed = true;
    }

    let outputs = window
        .user_data()
        .get::<ToplevelState>()
        .map(|state| state.lock().unwrap().outputs().to_vec())
        .unwrap_or_default();
    let wanted: HashSet<WlOutput> = outputs
        .iter()
        .flat_map(|output: &Output| output.client_outputs(client))
        .collect();

    for wl_output in wanted.difference(&state.wl_outputs) {
        handle.output_enter(wl_output);
        changed = true;
    }
    for wl_output in state.wl_outputs.difference(&wanted) {
        if wl_output.is_alive() {
            handle.output_leave(wl_output);
        }
        changed = true;
    }
    if state.wl_outputs != wanted {
        state.wl_outputs = wanted;
    }

    changed
}

impl<D, W> GlobalDispatch<ZwlrForeignToplevelManagerV1, WlrForeignToplevelGlobalData, D>
    for WlrForeignToplevelState<D, W>
where
    D: GlobalDispatch<ZwlrForeignToplevelManagerV1, WlrForeignToplevelGlobalData>
        + Dispatch<ZwlrForeignToplevelManagerV1, WlrManagerData>
        + Dispatch<ZwlrForeignToplevelHandleV1, WlrHandleState<W>>
        + WlrForeignToplevelHandler<Window = W>
        + 'static,
    W: Window + 'static,
{
    fn bind(
        state: &mut D,
        _dh: &DisplayHandle,
        _client: &Client,
        resource: New<ZwlrForeignToplevelManagerV1>,
        _global_data: &WlrForeignToplevelGlobalData,
        data_init: &mut DataInit<'_, D>,
    ) {
        let instance = data_init.init(resource, WlrManagerData::default());
        state.wlr_foreign_toplevel_state().instances.push(instance);
    }

    fn can_view(client: Client, global_data: &WlrForeignToplevelGlobalData) -> bool {
        (global_data.filter)(&client)
    }
}

impl<D, W> WlrForeignToplevelState<D, W>
where
    D: WlrForeignToplevelHandler<Window = W> + 'static,
    W: Window + 'static,
{
    fn remove_instance(&mut self, instance: &ZwlrForeignToplevelManagerV1) {
        self.instances.retain(|i| i != instance);
    }
}

impl<D, W> Dispatch<ZwlrForeignToplevelManagerV1, WlrManagerData, D>
    for WlrForeignToplevelState<D, W>
where
    D: GlobalDispatch<ZwlrForeignToplevelManagerV1, WlrForeignToplevelGlobalData>
        + Dispatch<ZwlrForeignToplevelManagerV1, WlrManagerData>
        + Dispatch<ZwlrForeignToplevelHandleV1, WlrHandleState<W>>
        + WlrForeignToplevelHandler<Window = W>
        + 'static,
    W: Window + 'static,
{
    fn request(
        state: &mut D,
        _client: &Client,
        obj: &ZwlrForeignToplevelManagerV1,
        request: zwlr_foreign_toplevel_manager_v1::Request,
        data: &WlrManagerData,
        _dh: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        if let zwlr_foreign_toplevel_manager_v1::Request::Stop = request {
            let handles = std::mem::take(&mut *data.handles.lock().unwrap());
            for handle in handles {
                if handle.is_alive() {
                    handle.closed();
                }
            }
            obj.finished();
            state.wlr_foreign_toplevel_state().remove_instance(obj);
        }
    }

    fn destroyed(
        state: &mut D,
        _client: ClientId,
        resource: &ZwlrForeignToplevelManagerV1,
        _data: &WlrManagerData,
    ) {
        state.wlr_foreign_toplevel_state().remove_instance(resource);
    }
}

fn window_from_handle<W: Window + 'static>(handle: &ZwlrForeignToplevelHandleV1) -> Option<W> {
    handle
        .data::<WlrHandleState<W>>()
        .and_then(|state| state.lock().unwrap().window.clone())
}

impl<D, W> Dispatch<ZwlrForeignToplevelHandleV1, WlrHandleState<W>, D>
    for WlrForeignToplevelState<D, W>
where
    D: GlobalDispatch<ZwlrForeignToplevelManagerV1, WlrForeignToplevelGlobalData>
        + Dispatch<ZwlrForeignToplevelManagerV1, WlrManagerData>
        + Dispatch<ZwlrForeignToplevelHandleV1, WlrHandleState<W>>
        + WlrForeignToplevelHandler<Window = W>
        + ToplevelManagementHandler<Window = W>
        + 'static,
    W: Window + ManagementWindow + 'static,
{
    fn request(
        state: &mut D,
        _client: &Client,
        obj: &ZwlrForeignToplevelHandleV1,
        request: zwlr_foreign_toplevel_handle_v1::Request,
        _data: &WlrHandleState<W>,
        dh: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        let Some(window) = window_from_handle::<W>(obj) else {
            return;
        };

        match request {
            zwlr_foreign_toplevel_handle_v1::Request::Activate { seat } => {
                state.activate(dh, &window, Seat::<D>::from_resource(&seat));
            }
            zwlr_foreign_toplevel_handle_v1::Request::Close => {
                state.close(dh, &window);
            }
            zwlr_foreign_toplevel_handle_v1::Request::SetMaximized => {
                state.maximize(dh, &window);
            }
            zwlr_foreign_toplevel_handle_v1::Request::UnsetMaximized => {
                state.unmaximize(dh, &window);
            }
            zwlr_foreign_toplevel_handle_v1::Request::SetMinimized => {
                state.minimize(dh, &window);
            }
            zwlr_foreign_toplevel_handle_v1::Request::UnsetMinimized => {
                state.unminimize(dh, &window);
            }
            zwlr_foreign_toplevel_handle_v1::Request::SetFullscreen { output } => {
                state.fullscreen(dh, &window, output.as_ref().and_then(Output::from_resource));
            }
            zwlr_foreign_toplevel_handle_v1::Request::UnsetFullscreen => {
                state.unfullscreen(dh, &window);
            }
            // set_rectangle is only a hint for interactive window dragging
            // previews; the requests above are what taskbars actually use.
            zwlr_foreign_toplevel_handle_v1::Request::SetRectangle { .. } => {}
            zwlr_foreign_toplevel_handle_v1::Request::Destroy => {}
            _ => {}
        }
    }

    fn destroyed(
        state: &mut D,
        _client: ClientId,
        resource: &ZwlrForeignToplevelHandleV1,
        _data: &WlrHandleState<W>,
    ) {
        for instance in &state.wlr_foreign_toplevel_state().instances {
            let Some(manager) = instance.data::<WlrManagerData>() else {
                continue;
            };
            manager
                .handles
                .lock()
                .unwrap()
                .retain(|handle| handle != resource);
        }
    }
}

macro_rules! delegate_wlr_foreign_toplevel {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty, $window: ty) => {
        smithay::reexports::wayland_server::delegate_global_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols_wlr::foreign_toplevel::v1::server::zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1: $crate::wayland::protocols::wlr_foreign_toplevel::WlrForeignToplevelGlobalData
        ] => $crate::wayland::protocols::wlr_foreign_toplevel::WlrForeignToplevelState<Self, $window>);
        smithay::reexports::wayland_server::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols_wlr::foreign_toplevel::v1::server::zwlr_foreign_toplevel_manager_v1::ZwlrForeignToplevelManagerV1: $crate::wayland::protocols::wlr_foreign_toplevel::WlrManagerData
        ] => $crate::wayland::protocols::wlr_foreign_toplevel::WlrForeignToplevelState<Self, $window>);
        smithay::reexports::wayland_server::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols_wlr::foreign_toplevel::v1::server::zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1: $crate::wayland::protocols::wlr_foreign_toplevel::WlrHandleState<$window>
        ] => $crate::wayland::protocols::wlr_foreign_toplevel::WlrForeignToplevelState<Self, $window>);
    };
}
pub(crate) use delegate_wlr_foreign_toplevel;
