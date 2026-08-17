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
            state = obs_open_04a_g1::kernel::step(&state, &observation, &context)?.state;
            let ancestor = obs_open_04a_g1::kernel::project_state_to_04a(&state, &context)?;
            records.push(ReplayRecord {
                session_id: session.spec.session_id.clone(),
                prefix_ordinal: index as u32,
                reduced: reduced_key(&ReducedState::from(&ancestor)),
                state: state.clone(),
                context: context.clone(),
            });
        }
    }
    records.sort_unstable_by(|a, b| {
        a.session_id
            .cmp(&b.session_id)
            .then(a.prefix_ordinal.cmp(&b.prefix_ordinal))
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
            .map(|v| v.map(location_code))
            .collect(),
        upper_extensions_bits: value
            .upper_extensions
            .iter()
            .map(|v| v.map(f64::to_bits))
            .collect(),
        lower_extensions_bits: value
            .lower_extensions
            .iter()
            .map(|v| v.map(f64::to_bits))
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
    fn location_codes_are_frozen() {
        assert_eq!(location_code(AncestorLocation::InZone), 0);
        assert_eq!(location_code(AncestorLocation::Above), 1);
        assert_eq!(location_code(AncestorLocation::Below), 2);
    }
}
