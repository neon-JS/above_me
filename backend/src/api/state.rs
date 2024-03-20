use crate::{
    aprs::{Position, Status},
    haversine::calculate_distance,
    time::get_current_timestamp,
};

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
};

const MAX_AGE_DIFF: u64 = 60 * 5; /* 5 minutes */

/// Our shared application state for the API
#[derive(Clone)]
pub struct AppState {
    /// Reference to all currently stored states
    states: Arc<Mutex<HashMap<String, Status>>>,
}

impl AppState {
    /// Creates a new `AppState`
    ///
    /// # Examples
    ///
    /// ```
    /// use api::AppState;
    ///
    /// let app_state = AppState::create();
    /// ```
    pub fn create() -> AppState {
        AppState {
            states: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Returns the states in the `AppState` that match given filters
    ///
    /// # Arguments
    /// * `position` - The position that should be searched for
    /// * `range` - Range around given `latitude` and `longitude` that should be searched for.
    ///
    /// # Examples
    /// ```
    /// use api::AppState;
    /// use aprs::Position;
    ///
    /// let app_state = AppState::create();
    ///
    /// // Push some states
    /// app_state.push_status(...);
    /// app_state.push_status(...);
    /// app_state.push_status(...);
    ///
    /// let position = Position {
    ///    latitude: 48.858222,
    ///    longitude: 2.2945,
    /// };
    ///
    /// var filtered_states = app_state.get_filtered_states(&position, 15);
    ///
    /// // filtered_states contains clones of all pushed states that match the given filter
    /// ```
    pub fn get_filtered_states(&self, position: &Position, range: f32) -> Vec<Status> {
        let mut states = self.states.lock().expect("Mutex was poisoned");

        // TODO: Check if this is the correct place to call this function as this isn't really correct.
        AppState::remove_outdated_states(&mut states);

        states
            .values()
            .filter(|&status| calculate_distance(position, &status.position) <= range)
            .cloned()
            .collect::<Vec<Status>>()
    }

    /// Stores / updates a new status in the `AppState`
    ///
    /// # Arguments
    ///
    /// * `status` - The status to store / update
    ///
    /// # Examples
    /// ```
    /// use api::AppState;
    /// use aprs::Status;
    ///
    /// let app_state = AppState::create();
    ///
    /// let status = Status { ... };
    ///
    /// app_state.push_status(status);
    /// ```
    pub async fn push_status(&self, status: Status) {
        let mut states = self.states.lock().expect("Mutex was poisoned");

        // TODO: Check if this is the correct place to call this function as `push_async` may be called often.
        AppState::remove_outdated_states(&mut states);
        states.insert(status.aircraft.call_sign.clone(), status);
    }

    /// Removes outdated states (by max age)
    ///
    /// # Arguments
    ///
    /// * `states` - `MutexGuard` of states map
    fn remove_outdated_states(states: &mut MutexGuard<HashMap<String, Status>>) {
        let current_timestamp = get_current_timestamp();

        let outdated_keys = states
            .values()
            .filter(|e| current_timestamp - e.time_stamp > MAX_AGE_DIFF)
            .map(|e| e.aircraft.call_sign.clone())
            .collect::<Vec<String>>();

        for key in outdated_keys {
            states.remove(&key);
        }
    }
}
