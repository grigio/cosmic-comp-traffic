// SPDX-License-Identifier: GPL-3.0-only

use smithay::utils::{Rectangle, user_data::UserDataMap};

use crate::{
    shell::CosmicSurface,
    state::State,
    utils::prelude::Global,
    wayland::protocols::{
        toplevel_info::{ToplevelInfoHandler, ToplevelInfoState, Window, delegate_toplevel_info},
        wlr_foreign_toplevel::{
            WlrForeignToplevelHandler, WlrForeignToplevelState, delegate_wlr_foreign_toplevel,
        },
    },
};

impl ToplevelInfoHandler for State {
    type Window = CosmicSurface;

    fn toplevel_info_state(&self) -> &ToplevelInfoState<State, Self::Window> {
        &self.common.toplevel_info_state
    }
    fn toplevel_info_state_mut(&mut self) -> &mut ToplevelInfoState<State, Self::Window> {
        &mut self.common.toplevel_info_state
    }
}

impl Window for CosmicSurface {
    fn title(&self) -> String {
        CosmicSurface::title(self)
    }

    fn app_id(&self) -> String {
        CosmicSurface::app_id(self)
    }

    fn is_activated(&self) -> bool {
        !self.is_minimized() && CosmicSurface::is_activated(self, true)
    }

    fn is_maximized(&self) -> bool {
        !self.is_minimized() && CosmicSurface::is_maximized(self, false)
    }

    fn is_fullscreen(&self) -> bool {
        !self.is_minimized() && CosmicSurface::is_fullscreen(self, false)
    }

    fn is_minimized(&self) -> bool {
        CosmicSurface::is_minimized(self)
    }

    fn is_sticky(&self) -> bool {
        CosmicSurface::is_sticky(self)
    }

    fn is_resizing(&self) -> bool {
        CosmicSurface::is_resizing(self, true).unwrap_or(false)
    }

    fn global_geometry(&self) -> Option<Rectangle<i32, Global>> {
        CosmicSurface::global_geometry(self)
    }

    fn user_data(&self) -> &UserDataMap {
        CosmicSurface::user_data(self)
    }
}

impl WlrForeignToplevelHandler for State {
    fn wlr_foreign_toplevel_state(&mut self) -> &mut WlrForeignToplevelState<State, CosmicSurface> {
        &mut self.common.wlr_foreign_toplevel_state
    }
}

delegate_toplevel_info!(State, CosmicSurface);
delegate_wlr_foreign_toplevel!(State, CosmicSurface);
