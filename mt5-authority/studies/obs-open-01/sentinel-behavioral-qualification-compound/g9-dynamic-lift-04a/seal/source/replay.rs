use crate::model::{ReducedKey, ReplayRecord};
use obs_open_03a::AtlasSession;
use obs_open_04a::model::{Location as AncestorLocation, ReducedState};
use obs_open_04a_g1::model::InputAuthority;

pub fn replay_sessions(
    sessions: &[AtlasSession],
) -> Result<Vec<ReplayRecord>, Box<dyn std::error::Error>> {
    let capacity = sessions.iter().map(|session| session.bars.len()).sum();
    let mut records = Vec::with_capacity(capacity);
    for session in sessions {
        let context = obs_open_04a_g1::audit::context_from_session(session)?;
        let mut state = obs_open_04a_g1::kernel::init(&context)?;
        for (index, bar) in session.bars.iter().enumerate() {
            let observation = obs_open_04a_g1::audit::observation_from_bar(
                bar,
                InputAuthority::CanonicalIntegerM1BridgeV1,
            )?;
            let result = obs_open_04a_g1::kernel::step(&state, &observation, &context)?;
            state = result.state;
            let ancestor = obs_open_04a_g1::kernel::project_state_to_04a(&state, &context)?;
            let reduced = ReducedState::from(&ancestor);
            records.push(ReplayRecord {
                session_id: session.spec.session_id.clone(),
                prefix_ordinal: index as u32,
                state: state.clone(),
                context: context.clone(),
                reduced: reduced_key(&reduced),
            });
        }
    }
    records.sort_unstable_by(|left, right| {
        left.session_id
            .cmp(&right.session_id)
            .then(left.prefix_ordinal.cmp(&right.prefix_ordinal))
    });
    Ok(records)
}

fn reduced_key(value: &ReducedState) -> ReducedKey {
    ReducedKey {
        upper_value_bits: value.upper_value.to_bits(),
        lower_value_bits: value.lower_value.to_bits(),
        upper_giveback_bits: value.upper_giveback.to_bits(),
        lower_giveback_bits: value.lower_giveback.to_bits(),
        range_locations: value
            .range_locations
            .iter()
            .map(|item| item.map(location_code))
            .collect(),
        upper_extensions_bits: value
            .upper_extensions
            .iter()
            .map(|item| item.map(f64::to_bits))
            .collect(),
        lower_extensions_bits: value
            .lower_extensions
            .iter()
            .map(|item| item.map(f64::to_bits))
            .collect(),
        window_active: value.window_active,
        coverage_complete: value.coverage_complete,
    }
}

fn location_code(value: AncestorLocation) -> u8 {
    match value {
        AncestorLocation::InZone => 0,
        AncestorLocation::Above => 1,
        AncestorLocation::Below => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reduced_key_preserves_float_bits() {
        let state = ReducedState {
            upper_value: 1.0,
            lower_value: -0.0,
            upper_giveback: 2.0,
            lower_giveback: 3.0,
            range_locations: vec![Some(AncestorLocation::Above), None],
            upper_extensions: vec![Some(0.0)],
            lower_extensions: vec![Some(-0.0)],
            window_active: true,
            coverage_complete: false,
        };
        let key = reduced_key(&state);
        assert_eq!(key.lower_value_bits, (-0.0_f64).to_bits());
        assert_eq!(key.range_locations, vec![Some(1), None]);
    }
}
