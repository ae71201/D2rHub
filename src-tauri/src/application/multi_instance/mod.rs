mod account_creation;
mod account_deletion;
mod account_game_settings;
mod account_initialization;
mod account_mods;
mod account_naming;
mod account_ordering;
mod account_positions;
mod account_profile;
mod account_query;
mod account_settings;
mod facade;
mod frame_calibration;
mod instances;
mod launch;
mod launch_plan;
mod leases;
mod ports;

pub use account_creation::{AccountCreationService, CreateAccountRequest, TimestampProvider};
pub use account_deletion::AccountDeletionService;
pub use account_game_settings::AccountGameSettingsService;
pub use account_initialization::{AccountInitializationKind, AccountInitializationService};
pub use account_mods::AccountModService;
pub use account_naming::AccountNamingService;
pub use account_ordering::AccountOrderingService;
pub use account_positions::AccountPositionService;
pub use account_profile::{
    AccountProfilePatch, AccountProfilePolicy, AccountProfileService, ResolvedAccountProfile,
    TokenProtector,
};
pub use account_query::AccountQueryService;
pub use account_settings::AccountSettingsPreferenceService;
pub use facade::{MultiInstanceFacade, WindowMatch};
pub use frame_calibration::{
    FrameCalibrationClaim, FrameCalibrationKey, FrameCalibrationLease, FrameCalibrations,
};
pub use instances::{InstanceRegistry, InstanceRegistrySnapshot, RunningInstance};
pub use launch::{CancellationTicket, LaunchOrchestrator};
pub use launch_plan::{
    launch_queue_can_continue, LaunchAccountEntry, LaunchBatchPlan, LaunchGraphicsOverride,
};
pub use leases::{
    AccountCatalogLeaseManager, AccountLeaseManager, AccountOperationLease, AccountOperationLeases,
};
pub use ports::{
    AccountCatalog, AccountCreationRepository, AccountDeletionCleanupPort,
    AccountDeletionTransaction, AccountGameSettingsRepository, AccountInitializationTransaction,
    AccountModRepository, AccountNameRepository, AccountRenameTransaction, AccountRepository,
    AccountRuntimePort, AccountSettingsPreferenceRepository, GameSettings, GameWindowIdentity,
    GameWindowPort, InstanceStatusPort, WindowPosition,
};

#[derive(Default)]
pub struct MultiInstanceRuntime {
    instances: InstanceRegistry,
    launches: LaunchOrchestrator,
    account_leases: AccountLeaseManager,
    catalog_leases: AccountCatalogLeaseManager,
    frame_calibrations: std::sync::Arc<FrameCalibrations>,
}

impl MultiInstanceRuntime {
    pub fn frame_calibrations(&self) -> &std::sync::Arc<FrameCalibrations> {
        &self.frame_calibrations
    }
    pub fn instances(&self) -> &InstanceRegistry {
        &self.instances
    }

    pub fn facade(&self) -> MultiInstanceFacade<'_> {
        MultiInstanceFacade::new(&self.instances, &self.launches)
    }

    pub fn account_leases(&self) -> &AccountLeaseManager {
        &self.account_leases
    }

    pub fn catalog_leases(&self) -> &AccountCatalogLeaseManager {
        &self.catalog_leases
    }
}
