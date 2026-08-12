use super::{LevelId, LevelState, StructuralMutation, StructuralNodeId, StructuralObject};
use hashbrown::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// Single-writer store for structural objects. Objects are never physically
/// removed, which preserves stable IDs and makes mutation replay auditable.
pub struct LevelBook {
    objects: Vec<StructuralObject>,
    indices: HashMap<LevelId, usize>,
    validation_states: HashMap<LevelId, LevelState>,
    generation: u64,
    active_cache: Arc<[StructuralObject]>,
    cache_generation: u64,
}

impl Default for LevelBook {
    fn default() -> Self {
        Self::new()
    }
}

impl LevelBook {
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
            indices: HashMap::new(),
            validation_states: HashMap::new(),
            generation: 0,
            active_cache: Arc::from([]),
            cache_generation: 0,
        }
    }

    #[inline]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    #[inline]
    pub fn get(&self, id: LevelId) -> Option<&StructuralObject> {
        self.indices
            .get(&id)
            .and_then(|index| self.objects.get(*index))
    }

    #[inline]
    pub fn all(&self) -> &[StructuralObject] {
        &self.objects
    }

    /// Applies one deterministic generation atomically. Validation simulates
    /// every transition before storage is changed.
    pub fn apply_batch(
        &mut self,
        generation: u64,
        mutations: &[StructuralMutation],
    ) -> Result<bool, LevelBookError> {
        if mutations.is_empty() {
            return Ok(false);
        }
        if generation != self.generation.saturating_add(1) {
            return Err(LevelBookError::NonSequentialGeneration {
                expected: self.generation.saturating_add(1),
                actual: generation,
            });
        }

        self.validation_states.clear();
        self.validation_states.reserve(self.objects.len());
        self.validation_states.extend(
            self.objects
                .iter()
                .map(|object| (object.id(), object.state())),
        );
        for mutation in mutations {
            validate_transition(
                mutation,
                &mut self.validation_states,
                &self.objects,
                &self.indices,
            )?;
        }

        for mutation in mutations {
            self.apply_validated(mutation.clone());
        }
        self.generation = generation;
        Ok(true)
    }

    pub fn active_snapshot(&mut self) -> Arc<[StructuralObject]> {
        if self.cache_generation == self.generation {
            return Arc::clone(&self.active_cache);
        }
        let mut active: Vec<_> = self
            .objects
            .iter()
            .filter(|object| object.is_active())
            .cloned()
            .collect();
        active.sort_unstable_by_key(|object| {
            (
                object.instrument_id(),
                object.price_space(),
                object.reference_price(),
                object.id(),
            )
        });
        self.active_cache = active.into();
        self.cache_generation = self.generation;
        Arc::clone(&self.active_cache)
    }

    fn apply_validated(&mut self, mutation: StructuralMutation) {
        match mutation {
            StructuralMutation::Created(object) => self.insert(object),
            StructuralMutation::Updated(object) => {
                let index = self.indices[&object.id()];
                self.objects[index] = object;
            }
            StructuralMutation::Frozen { id, at_ns } => {
                let index = self.indices[&id];
                self.objects[index].set_state(LevelState::Frozen, at_ns);
            }
            StructuralMutation::Superseded { old, new, at_ns } => {
                let index = self.indices[&old];
                self.objects[index].set_state(LevelState::Superseded, at_ns);
                self.insert(new);
            }
            StructuralMutation::Expired { id, at_ns } => {
                let index = self.indices[&id];
                self.objects[index].set_state(LevelState::Expired, at_ns);
            }
        }
    }

    fn insert(&mut self, object: StructuralObject) {
        let id = object.id();
        let index = self.objects.len();
        self.objects.push(object);
        self.indices.insert(id, index);
    }
}

fn validate_transition(
    mutation: &StructuralMutation,
    simulated: &mut HashMap<LevelId, LevelState>,
    objects: &[StructuralObject],
    indices: &HashMap<LevelId, usize>,
) -> Result<(), LevelBookError> {
    match mutation {
        StructuralMutation::Created(object) => {
            validate_object(object)?;
            if simulated.insert(object.id(), object.state()).is_some() {
                return Err(LevelBookError::DuplicateIdentity(object.id()));
            }
        }
        StructuralMutation::Updated(object) => {
            validate_object(object)?;
            let state = simulated
                .get(&object.id())
                .copied()
                .ok_or(LevelBookError::UnknownIdentity(object.id()))?;
            if state != LevelState::Developing || object.state() != LevelState::Developing {
                return Err(LevelBookError::ImmutableIdentity(object.id()));
            }
            let previous = indices
                .get(&object.id())
                .and_then(|index| objects.get(*index))
                .ok_or(LevelBookError::UnknownIdentity(object.id()))?;
            if previous.instrument_id() != object.instrument_id()
                || previous.price_space() != object.price_space()
                || previous.kind() != object.kind()
                || previous.family() != object.family()
                || previous.provenance().producer_id != object.provenance().producer_id
                || previous.provenance().epoch_ns != object.provenance().epoch_ns
            {
                return Err(LevelBookError::IdentityDrift(object.id()));
            }
        }
        StructuralMutation::Frozen { id, .. } => {
            transition_existing(simulated, *id, LevelState::Developing, LevelState::Frozen)?;
        }
        StructuralMutation::Superseded { old, new, .. } => {
            validate_object(new)?;
            let old_state = simulated
                .get_mut(old)
                .ok_or(LevelBookError::UnknownIdentity(*old))?;
            if !matches!(*old_state, LevelState::Developing | LevelState::Frozen) {
                return Err(LevelBookError::ImmutableIdentity(*old));
            }
            *old_state = LevelState::Superseded;
            if simulated.insert(new.id(), new.state()).is_some() {
                return Err(LevelBookError::DuplicateIdentity(new.id()));
            }
        }
        StructuralMutation::Expired { id, .. } => {
            let state = simulated
                .get_mut(id)
                .ok_or(LevelBookError::UnknownIdentity(*id))?;
            if !matches!(*state, LevelState::Developing | LevelState::Frozen) {
                return Err(LevelBookError::ImmutableIdentity(*id));
            }
            *state = LevelState::Expired;
        }
    }
    Ok(())
}

fn transition_existing(
    simulated: &mut HashMap<LevelId, LevelState>,
    id: LevelId,
    expected: LevelState,
    next: LevelState,
) -> Result<(), LevelBookError> {
    let state = simulated
        .get_mut(&id)
        .ok_or(LevelBookError::UnknownIdentity(id))?;
    if *state != expected {
        return Err(LevelBookError::ImmutableIdentity(id));
    }
    *state = next;
    Ok(())
}

fn validate_object(object: &StructuralObject) -> Result<(), LevelBookError> {
    if object.id() == LevelId::UNKNOWN || object.instrument_id().get() == 0 {
        return Err(LevelBookError::MissingIdentity);
    }
    if object.lower() > object.upper()
        || object.reference_price() < object.lower()
        || object.reference_price() > object.upper()
    {
        return Err(LevelBookError::InvalidGeometry(object.id()));
    }
    if object.provenance().producer_id.get() == 0
        || object.provenance().derivation_version.get() == 0
        || object.provenance().first_sequence.get() == 0
        || object.provenance().last_sequence < object.provenance().first_sequence
    {
        return Err(LevelBookError::InvalidProvenance(object.id()));
    }
    Ok(())
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum LevelBookError {
    #[error("level and instrument identities must be non-zero")]
    MissingIdentity,
    #[error("level {0:?} already exists")]
    DuplicateIdentity(LevelId),
    #[error("level {0:?} does not exist")]
    UnknownIdentity(LevelId),
    #[error("level {0:?} is frozen, superseded, or expired")]
    ImmutableIdentity(LevelId),
    #[error("level {0:?} changed its semantic identity")]
    IdentityDrift(LevelId),
    #[error("level {0:?} has invalid price geometry")]
    InvalidGeometry(LevelId),
    #[error("level {0:?} has invalid causal provenance")]
    InvalidProvenance(LevelId),
    #[error("structural generation must be {expected}, got {actual}")]
    NonSequentialGeneration { expected: u64, actual: u64 },
    #[error("node {0:?} is not represented in this level book")]
    UnknownNode(StructuralNodeId),
}
