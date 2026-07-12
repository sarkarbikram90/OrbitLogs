use crate::SatelliteId;
use std::marker::PhantomData;
use uuid::Uuid;

// =========================================================================
// Typestate Command State Machine (P1 #3)
// =========================================================================
//
// The command lifecycle (Created → Approved → Queued → Transmitted → Executed)
// is enforced at COMPILE TIME. You literally cannot call `.queue()` on a
// Command<Created> — it won't compile. No runtime `if status != Approved` checks.
//
// Key Rust features:
// - PhantomData<S> for zero-sized type state markers
// - Consuming `self` to prevent reuse after state transitions
// - Each state only exposes valid transition methods

/// Marker trait for command states. Sealed — cannot be implemented outside this module.
pub trait CommandState: private::Sealed {}

mod private {
    pub trait Sealed {}
    impl Sealed for super::Created {}
    impl Sealed for super::Approved {}
    impl Sealed for super::Queued {}
    impl Sealed for super::Transmitted {}
    impl Sealed for super::Executed {}
}

// --- State Marker Types (Zero-Sized Types) ---

/// Command has been created but not yet approved.
pub struct Created;
impl CommandState for Created {}

/// Command has been approved by a flight director.
pub struct Approved;
impl CommandState for Approved {}

/// Command has been added to the uplink queue.
pub struct Queued;
impl CommandState for Queued {}

/// Command has been transmitted to the spacecraft.
pub struct Transmitted;
impl CommandState for Transmitted {}

/// Command has been executed by the spacecraft.
pub struct Executed;
impl CommandState for Executed {}

// --- The Typed Command ---

/// A command with compile-time state enforcement.
///
/// The type parameter `S` encodes the current state. Only methods valid
/// for that state are available. State transitions consume `self` and
/// return a new `TypedCommand` in the next state — preventing reuse.
///
/// # Examples
/// ```
/// use orbitlogs_models::command_state::TypedCommand;
/// use orbitlogs_models::SatelliteId;
///
/// let sat = SatelliteId::parse("SAT-0001").unwrap();
/// let cmd = TypedCommand::new(sat, "REBOOT_OBC".to_string(), "operator_1".to_string());
///
/// // Valid transition chain:
/// let approved = cmd.approve("flight_director_1".to_string());
/// let queued = approved.queue();
/// let transmitted = queued.transmit();
/// let executed = transmitted.execute();
///
/// // These would NOT compile:
/// // cmd.queue();        // Can't queue a Created command
/// // approved.approve(); // Can't approve twice
/// // queued.approve();   // Can't go backwards
/// ```
pub struct TypedCommand<S: CommandState> {
    pub id: Uuid,
    pub satellite_id: SatelliteId,
    pub command_type: String,
    pub created_by: String,
    pub approved_by: Option<String>,
    _state: PhantomData<S>,
}

impl TypedCommand<Created> {
    /// Create a new command in the `Created` state.
    pub fn new(satellite_id: SatelliteId, command_type: String, created_by: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            satellite_id,
            command_type,
            created_by,
            approved_by: None,
            _state: PhantomData,
        }
    }

    /// Approve the command. Only a `Created` command can be approved.
    /// Consumes `self` — the `Created` command no longer exists.
    pub fn approve(self, approver: String) -> TypedCommand<Approved> {
        TypedCommand {
            id: self.id,
            satellite_id: self.satellite_id,
            command_type: self.command_type,
            created_by: self.created_by,
            approved_by: Some(approver),
            _state: PhantomData,
        }
    }
}

impl TypedCommand<Approved> {
    /// Queue the command for uplink. Only an `Approved` command can be queued.
    pub fn queue(self) -> TypedCommand<Queued> {
        TypedCommand {
            id: self.id,
            satellite_id: self.satellite_id,
            command_type: self.command_type,
            created_by: self.created_by,
            approved_by: self.approved_by,
            _state: PhantomData,
        }
    }
}

impl TypedCommand<Queued> {
    /// Transmit the command to the spacecraft.
    pub fn transmit(self) -> TypedCommand<Transmitted> {
        TypedCommand {
            id: self.id,
            satellite_id: self.satellite_id,
            command_type: self.command_type,
            created_by: self.created_by,
            approved_by: self.approved_by,
            _state: PhantomData,
        }
    }
}

impl TypedCommand<Transmitted> {
    /// Mark the command as executed by the spacecraft.
    pub fn execute(self) -> TypedCommand<Executed> {
        TypedCommand {
            id: self.id,
            satellite_id: self.satellite_id,
            command_type: self.command_type,
            created_by: self.created_by,
            approved_by: self.approved_by,
            _state: PhantomData,
        }
    }
}

impl<S: CommandState> TypedCommand<S> {
    /// Get the command ID (available in any state).
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get the satellite ID (available in any state).
    pub fn satellite_id(&self) -> &SatelliteId {
        &self.satellite_id
    }
}

impl<S: CommandState> std::fmt::Debug for TypedCommand<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TypedCommand")
            .field("id", &self.id)
            .field("satellite_id", &self.satellite_id)
            .field("command_type", &self.command_type)
            .field("state", &std::any::type_name::<S>())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_lifecycle() {
        let sat = SatelliteId::from_trusted("SAT-0001".to_string());
        let cmd = TypedCommand::new(sat, "REBOOT_OBC".to_string(), "op1".to_string());
        let id = cmd.id();

        let approved = cmd.approve("fd1".to_string());
        assert_eq!(approved.id(), id);
        assert_eq!(approved.approved_by.as_deref(), Some("fd1"));

        let queued = approved.queue();
        assert_eq!(queued.id(), id);

        let transmitted = queued.transmit();
        assert_eq!(transmitted.id(), id);

        let executed = transmitted.execute();
        assert_eq!(executed.id(), id);
    }

    #[test]
    fn command_debug_shows_state() {
        let sat = SatelliteId::from_trusted("SAT-0001".to_string());
        let cmd = TypedCommand::new(sat, "PING".to_string(), "op1".to_string());
        let debug = format!("{:?}", cmd);
        assert!(debug.contains("Created"));
    }

    // The following tests are compile-time checks. If they compile,
    // the typestate pattern is working. Uncomment to verify they
    // DON'T compile:
    //
    // #[test]
    // fn cannot_queue_created_command() {
    //     let sat = SatelliteId::from_trusted("SAT-0001".to_string());
    //     let cmd = TypedCommand::new(sat, "X".to_string(), "op1".to_string());
    //     cmd.queue(); // ERROR: no method named `queue` found for TypedCommand<Created>
    // }
    //
    // #[test]
    // fn cannot_approve_twice() {
    //     let sat = SatelliteId::from_trusted("SAT-0001".to_string());
    //     let cmd = TypedCommand::new(sat, "X".to_string(), "op1".to_string());
    //     let approved = cmd.approve("fd1".to_string());
    //     approved.approve("fd2".to_string()); // ERROR: no method named `approve`
    // }
}
