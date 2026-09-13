//! NEXUS — TWARDY.exe / TW4RDYDEV
//! Canonical persisted vocabulary; labels may evolve without changing identifiers.
macro_rules! vocabulary {
    ($name:ident { $($variant:ident => ($canonical:literal, $label:literal)),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum $name { $($variant),+ }
        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];
            pub const fn as_str(self) -> &'static str { match self { $(Self::$variant => $canonical),+ } }
            pub const fn label(self) -> &'static str { match self { $(Self::$variant => $label),+ } }
            pub fn parse(value: &str) -> Option<Self> { Self::ALL.iter().copied().find(|v| v.as_str() == value) }
            pub fn registry() -> serde_json::Value {
                serde_json::json!(Self::ALL.iter().map(|v| serde_json::json!({"id":v.as_str(),"label":v.label()})).collect::<Vec<_>>())
            }
        }
    }
}

vocabulary!(DomainEvent {
    AssetDiscovered => ("nx.asset.discovered", "Asset discovered"),
    AssetEnriched => ("nx.asset.enriched", "Asset enriched"),
    ServiceObserved => ("nx.service.observed", "Service observed"),
    CredentialRecorded => ("nx.credential.recorded", "Credential recorded"),
    AuthenticationConfirmed => ("nx.authentication.confirmed", "Authentication confirmed"),
    AuthenticationRecorded => ("nx.authentication.recorded", "Authentication recorded"),
    SessionOpened => ("nx.session.opened", "Session opened"),
    PivotCreated => ("nx.pivot.created", "Pivot created"),
    NetworkReachable => ("nx.network.reachable", "Network reachable"),
    FindingConfirmed => ("nx.finding.confirmed", "Finding confirmed"),
    FindingRecorded => ("nx.finding.recorded", "Finding recorded"),
    SnapshotCreated => ("nx.snapshot.created", "Snapshot created"),
    ImportCompleted => ("nx.import.completed", "Import completed"),
    EngagementCreated => ("nx.engagement.created", "Engagement created"),
    DemoReady => ("nx.demo.ready", "Demo ready"),
    EngagementRecorded => ("nx.engagement.recorded", "Engagement recorded"),
    WorkspaceRecovered => ("nx.workspace.recovered", "Workspace recovered"),
    AssetAliasRecorded => ("nx.asset.alias.recorded", "Alias recorded"),
    ScopeChanged => ("nx.scope.changed", "Scope changed"),
    PathPinned => ("nx.graph.path.pinned", "Path pinned"),
    VaultUnlocked => ("nx.vault.unlocked", "Vault unlocked"),
    VaultLocked => ("nx.vault.locked", "Vault locked"),
    VaultStateRecorded => ("nx.vault.state.recorded", "Vault state recorded"),
    EvidenceAttached => ("nx.evidence.attached", "Evidence attached"),
    ScanBlocked => ("nx.scope.scan.blocked", "Scan blocked"),
    ScanStarted => ("nx.scan.started", "Scan started"),
    ScanFinished => ("nx.scan.finished", "Scan finished"),
    ScanRecorded => ("nx.scan.recorded", "Scan recorded"),
    ReportExported => ("nx.report.exported", "Report exported"),
    EntityEdited => ("nx.entity.edited", "Entity edited"),
    BackupCreated => ("nx.backup.created", "Backup created"),
    EditUndone => ("nx.entity.edit.undone", "Edit undone"),
    ObservationRecorded => ("nx.observation.recorded", "Observation recorded"),
    PivotRecorded => ("nx.pivot.recorded", "Pivot recorded")
});

vocabulary!(RelationshipKind {
    Exposes => ("NX_EXPOSES", "Exposes"),
    ResolvesTo => ("NX_RESOLVES_TO", "Resolves To"),
    AuthenticatesTo => ("NX_AUTHENTICATES_TO", "Authenticates To"),
    DiscoveredFrom => ("NX_DISCOVERED_FROM", "Discovered From"),
    HasSession => ("NX_HAS_SESSION", "Has Session"),
    ReachableVia => ("NX_REACHABLE_VIA", "Reachable Via"),
    AffectedBy => ("NX_AFFECTED_BY", "Affected By"),
    EvidencedBy => ("NX_EVIDENCED_BY", "Evidenced By"),
    ConnectsTo => ("NX_CONNECTS_TO", "Connects To"),
    MemberOf => ("NX_MEMBER_OF", "Member Of")
});

impl DomainEvent {
    /// Legacy categories lacked enough detail to infer a successful authentication,
    /// active session or confirmed finding. Preserve that uncertainty on migration.
    pub fn legacy(value: &str) -> Option<Self> {
        Some(match value {
            "Discovery" => Self::ObservationRecorded,
            "Authentication" => Self::AuthenticationRecorded,
            "Finding" => Self::FindingRecorded,
            "Engagement" => Self::EngagementRecorded,
            "Vault" => Self::VaultStateRecorded,
            "Tool execution" => Self::ScanRecorded,
            "Pivot" => Self::PivotRecorded,
            "Manual edit" => Self::EntityEdited,
            "Import" => Self::ImportCompleted,
            "Snapshot" => Self::SnapshotCreated,
            "Recovery" => Self::WorkspaceRecovered,
            "Alias" => Self::AssetAliasRecorded,
            "Scope" => Self::ScopeChanged,
            "Attack path" => Self::PathPinned,
            "Credential" => Self::CredentialRecorded,
            "Evidence" => Self::EvidenceAttached,
            "Scope Guard" => Self::ScanBlocked,
            "Export" => Self::ReportExported,
            "Backup" => Self::BackupCreated,
            "Undo" => Self::EditUndone,
            _ => return None,
        })
    }
}
