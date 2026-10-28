// SPDX-License-Identifier: GPL-3.0-only

use smithay::output::Output;
use tracing::warn;

use crate::{
    state::{BackendData, State},
    wayland::protocols::gamma_control::{
        GammaControlHandler, GammaControlState, delegate_gamma_control,
    },
};

impl GammaControlHandler for State {
    fn gamma_control_state(&mut self) -> &mut GammaControlState {
        &mut self.common.gamma_control_state
    }

    fn gamma_table_size(&mut self, output: &Output) -> Option<u32> {
        match &self.backend {
            BackendData::Kms(kms) => kms.gamma_table_size(output),
            _ => None,
        }
    }

    fn set_output_gamma(&mut self, output: &Output, table: Option<Vec<u16>>) {
        let BackendData::Kms(kms) = &self.backend else {
            return;
        };

        if let Err(err) = kms.set_output_gamma(output, table.as_deref()) {
            warn!(?err, output = %output.name(), "Failed to set the gamma table");
        }
    }
}

delegate_gamma_control!(State);
