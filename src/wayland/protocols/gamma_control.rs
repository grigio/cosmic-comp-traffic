// SPDX-License-Identifier: GPL-3.0-only

use smithay::{
    output::{Output, WeakOutput},
    reexports::{
        wayland_protocols_wlr::gamma_control::v1::server::{
            zwlr_gamma_control_manager_v1::{self, ZwlrGammaControlManagerV1},
            zwlr_gamma_control_v1::{self, ZwlrGammaControlV1},
        },
        wayland_server::{
            Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource,
            backend::GlobalId,
        },
    },
};
use std::{
    collections::HashSet,
    fs::File,
    io, mem,
    os::{fd::OwnedFd, unix::fs::FileExt},
};
use tracing::warn;
use wayland_backend::server::ClientId;

/// Server side of `wlr-gamma-control-unstable-v1`.
///
/// The protocol gives one privileged client per output exclusive access to the
/// hardware gamma ramp, which is what night light / redshift clients use: they
/// do not draw a red overlay, they reprogram the CRTC lookup table.
pub trait GammaControlHandler {
    fn gamma_control_state(&mut self) -> &mut GammaControlState;

    /// Number of entries in each of the output's three ramps, if the output has
    /// a legacy gamma table. `None` means "no gamma control for this output"
    /// and makes the compositor send `failed`.
    fn gamma_table_size(&mut self, output: &Output) -> Option<u32>;

    /// Whether two outputs share a physical CRTC (and therefore one hardware
    /// ramp). Defaults to identity; the KMS backend widens this to mirrors.
    fn gamma_outputs_share_crtc(&self, a: &Output, b: &Output) -> bool {
        a == b
    }

    /// Apply `table` (red, green and blue ramps concatenated, `3 * size`
    /// entries) or restore the identity ramp with `None`.
    fn set_output_gamma(&mut self, output: &Output, table: Option<Vec<u16>>);
}

#[derive(Debug)]
pub struct GammaControlState {
    global: GlobalId,
    controls: HashSet<ZwlrGammaControlV1>,
    /// Last table per output. The hardware LUT is part of the CRTC state, so it
    /// can be dropped by a modeset, DPMS off/on or a hotplug re-creation; this
    /// is what `reapply` puts back. The output is weak so a removed output is
    /// not kept alive by its gamma table.
    tables: Vec<(WeakOutput, Vec<u16>)>,
}

impl GammaControlState {
    fn store_table(&mut self, output: Output, table: Vec<u16>) {
        let weak = output.downgrade();
        match self.tables.iter_mut().find(|(known, _)| *known == weak) {
            Some((_, known_table)) => *known_table = table,
            None => self.tables.push((weak, table)),
        }
    }

    fn forget_table(&mut self, output: &Output) -> bool {
        let weak = output.downgrade();
        let before = self.tables.len();
        self.tables.retain(|(known, _)| *known != weak);
        before != self.tables.len()
    }

    pub fn new<D, F>(dh: &DisplayHandle, client_filter: F) -> GammaControlState
    where
        D: GlobalDispatch<ZwlrGammaControlManagerV1, GammaControlManagerGlobalData> + 'static,
        F: for<'a> Fn(&'a Client) -> bool + Clone + Send + Sync + 'static,
    {
        let global = dh.create_global::<D, ZwlrGammaControlManagerV1, _>(
            1,
            GammaControlManagerGlobalData {
                filter: Box::new(client_filter.clone()),
            },
        );

        GammaControlState {
            global,
            controls: HashSet::new(),
            tables: Vec::new(),
        }
    }

    pub fn global_id(&self) -> GlobalId {
        self.global.clone()
    }

    /// Drop tables for outputs that no longer exist. Without this the stored
    /// `Output` handle would keep a removed output alive indefinitely.
    pub fn cleanup(&mut self) {
        self.tables.retain(|(output, _)| output.is_alive());
    }

    /// Re-send every stored table to the hardware. This is called after DPMS on
    /// and after an output configuration, which may have re-created CRTC
    /// surfaces and cleared their `GAMMA_LUT`.
    pub fn reapply<D: GammaControlHandler>(state: &mut D) {
        let tables = mem::take(&mut state.gamma_control_state().tables);
        let mut kept = Vec::with_capacity(tables.len());
        for (weak, table) in tables {
            let Some(output) = weak.upgrade() else {
                continue;
            };
            if table.len() % 3 == 0
                && state
                    .gamma_table_size(&output)
                    .is_some_and(|size| table.len() / 3 == size as usize)
            {
                state.set_output_gamma(&output, Some(table.clone()));
                kept.push((weak, table));
                continue;
            }

            // A modeset changed the CRTC ramp size. Drop the stale table and
            // tell its owner, which can then re-upload one of the right size.
            warn!(output = %output.name(), "Dropping stale gamma table after ramp size change");
            let controls: Vec<_> = state
                .gamma_control_state()
                .controls
                .iter()
                .cloned()
                .collect();
            for control in controls {
                if control
                    .data::<GammaControlData>()
                    .and_then(|data| data.output.upgrade())
                    .is_some_and(|other| state.gamma_outputs_share_crtc(&other, &output))
                {
                    control.failed();
                    state.gamma_control_state().controls.remove(&control);
                }
            }
        }
        state.gamma_control_state().tables = kept;
    }
}

pub struct GammaControlManagerGlobalData {
    filter: Box<dyn for<'a> Fn(&'a Client) -> bool + Send + Sync>,
}

#[derive(Debug)]
pub struct GammaControlData {
    output: WeakOutput,
}

/// Releases a control that owns `output`, restoring the identity ramp.
///
/// Controls that failed (`failed` event) are never inserted into `controls`;
/// calling this for one is a no-op, so a failed client cannot reset the table
/// of the client that actually owns the output.
fn release_control<D>(state: &mut D, obj: &ZwlrGammaControlV1, data: &GammaControlData)
where
    D: GammaControlHandler,
{
    if !state.gamma_control_state().controls.remove(obj) {
        return;
    }
    if let Some(output) = data.output.upgrade() {
        state.gamma_control_state().forget_table(&output);
        state.set_output_gamma(&output, None);
    }
}

impl<D> GlobalDispatch<ZwlrGammaControlManagerV1, GammaControlManagerGlobalData, D>
    for GammaControlState
where
    D: GlobalDispatch<ZwlrGammaControlManagerV1, GammaControlManagerGlobalData>
        + Dispatch<ZwlrGammaControlManagerV1, ()>
        + 'static,
{
    fn bind(
        _state: &mut D,
        _dh: &DisplayHandle,
        _client: &Client,
        resource: New<ZwlrGammaControlManagerV1>,
        _global_data: &GammaControlManagerGlobalData,
        data_init: &mut DataInit<'_, D>,
    ) {
        data_init.init(resource, ());
    }

    fn can_view(client: Client, global_data: &GammaControlManagerGlobalData) -> bool {
        (global_data.filter)(&client)
    }
}

impl<D> Dispatch<ZwlrGammaControlManagerV1, (), D> for GammaControlState
where
    D: GlobalDispatch<ZwlrGammaControlManagerV1, GammaControlManagerGlobalData>
        + Dispatch<ZwlrGammaControlManagerV1, ()>
        + Dispatch<ZwlrGammaControlV1, GammaControlData>
        + GammaControlHandler
        + 'static,
{
    fn request(
        state: &mut D,
        _client: &Client,
        _obj: &ZwlrGammaControlManagerV1,
        request: zwlr_gamma_control_manager_v1::Request,
        _data: &(),
        _dh: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            zwlr_gamma_control_manager_v1::Request::GetGammaControl { id, output } => {
                let output = Output::from_resource(&output);
                let control = data_init.init(
                    id,
                    GammaControlData {
                        output: output.as_ref().map(|o| o.downgrade()).unwrap_or_default(),
                    },
                );

                let Some(output) = output else {
                    control.failed();
                    return;
                };

                // Exclusive per physical CRTC: mirrored outputs share one ramp,
                // so a control for either one must block the other.
                let other_outputs: Vec<_> = state
                    .gamma_control_state()
                    .controls
                    .iter()
                    .filter_map(|other| {
                        other
                            .data::<GammaControlData>()
                            .and_then(|data| data.output.upgrade())
                    })
                    .collect();
                let taken = other_outputs
                    .iter()
                    .any(|other| state.gamma_outputs_share_crtc(other, &output));

                match state.gamma_table_size(&output) {
                    Some(size) if !taken => {
                        control.gamma_size(size);
                        state.gamma_control_state().controls.insert(control);
                    }
                    _ => control.failed(),
                }
            }
            zwlr_gamma_control_manager_v1::Request::Destroy => {}
            _ => unreachable!(),
        }
    }
}

impl<D> Dispatch<ZwlrGammaControlV1, GammaControlData, D> for GammaControlState
where
    D: Dispatch<ZwlrGammaControlV1, GammaControlData> + GammaControlHandler + 'static,
{
    fn request(
        state: &mut D,
        _client: &Client,
        obj: &ZwlrGammaControlV1,
        request: zwlr_gamma_control_v1::Request,
        data: &GammaControlData,
        _dh: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            zwlr_gamma_control_v1::Request::SetGamma { fd } => {
                // A control that failed does not own the output; ignore its
                // requests instead of letting it overwrite the owner's ramp.
                if !state.gamma_control_state().controls.contains(obj) {
                    return;
                }
                let Some(output) = data.output.upgrade() else {
                    obj.failed();
                    state.gamma_control_state().controls.remove(obj);
                    return;
                };
                let Some(size) = state.gamma_table_size(&output) else {
                    obj.failed();
                    release_control(state, obj, data);
                    return;
                };

                match read_table(fd, size) {
                    Ok(table) => {
                        state
                            .gamma_control_state()
                            .store_table(output.clone(), table.clone());
                        state.set_output_gamma(&output, Some(table));
                    }
                    Err(err) => {
                        warn!(
                            ?err,
                            output = %output.name(),
                            size,
                            "Invalid gamma table from client",
                        );
                        obj.post_error(
                            zwlr_gamma_control_v1::Error::InvalidGamma,
                            format!("gamma table is not {} bytes: {err}", size as usize * 3 * 2),
                        );
                    }
                }
            }
            zwlr_gamma_control_v1::Request::Destroy => {
                release_control(state, obj, data);
            }
            _ => unreachable!(),
        }
    }

    fn destroyed(
        state: &mut D,
        _client: ClientId,
        obj: &ZwlrGammaControlV1,
        data: &GammaControlData,
    ) {
        // The client went away without destroying the control: restore the
        // ramp as the protocol requires. Failed controls are ignored.
        release_control(state, obj, data);
    }
}

/// Read `size * 3` native-endian u16s (red, green, blue ramp) from the fd the
/// client passed to `set_gamma`.
///
/// The fd comes from an untrusted client. Reject anything that is not a
/// regular file and use `pread` instead of `read`: a pipe or FIFO would block
/// the whole compositor in `read_exact`, and a regular file is the shape the
/// protocol asks for (a memfd in practice).
fn read_table(fd: OwnedFd, size: u32) -> io::Result<Vec<u16>> {
    let file = File::from(fd);
    let metadata = file.metadata()?;
    if !metadata.file_type().is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "gamma table fd is not a regular file",
        ));
    }

    let len = (size as usize)
        .checked_mul(3 * 2)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "gamma table size overflow"))?;
    if metadata.len() != len as u64 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "gamma table file length does not match the expected table",
        ));
    }

    let mut raw = vec![0u8; len];
    file.read_exact_at(&mut raw, 0)?;
    Ok(raw
        .chunks_exact(2)
        .map(|pair| u16::from_ne_bytes([pair[0], pair[1]]))
        .collect())
}

macro_rules! delegate_gamma_control {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        smithay::reexports::wayland_server::delegate_global_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols_wlr::gamma_control::v1::server::zwlr_gamma_control_manager_v1::ZwlrGammaControlManagerV1: $crate::wayland::protocols::gamma_control::GammaControlManagerGlobalData
        ] => $crate::wayland::protocols::gamma_control::GammaControlState);
        smithay::reexports::wayland_server::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols_wlr::gamma_control::v1::server::zwlr_gamma_control_manager_v1::ZwlrGammaControlManagerV1: ()
        ] => $crate::wayland::protocols::gamma_control::GammaControlState);
        smithay::reexports::wayland_server::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols_wlr::gamma_control::v1::server::zwlr_gamma_control_v1::ZwlrGammaControlV1: $crate::wayland::protocols::gamma_control::GammaControlData
        ] => $crate::wayland::protocols::gamma_control::GammaControlState);
    };
}
pub(crate) use delegate_gamma_control;
