// Generated from Rust types. Run npm run contracts; do not edit.

export type AppError = { code: string, message: string, recovery: string, };

export type DeploymentStage = "validating" | "provisioning" | "containerStarting" | "downloading" | "loading" | "verifying" | "ready" | "draining" | "terminating" | "terminated" | "failed" | "cleanupPending" | "reconciliationRequired";

export type CliKind = "codex" | "claude";

export type SessionStatus = "starting" | "running" | "disconnected" | "ended";

export type Policy = { paidProvisioningEnabled: boolean, maxHourlyUsd: number, totalBudgetUsd: number | null, maxLifetimeMinutes: number | null, acknowledgeOfflineRisk: boolean, autoTerminateDelaySeconds: number | null, claudeEnabled: boolean, };

export type AppSettings = { policy: Policy, huggingFaceSecret: string | null, codexPath: string | null, claudePath: string | null, };

export type Project = { id: string, name: string, path: string, };

export type Deployment = { id: string, podName: string, podIds: Array<string>, profile: ResolvedProfile, stage: DeploymentStage, intendedAction: string, createdAt: number, updatedAt: number, terminatedAt: number | null, deadlineAt: number, verifiedAt: number | null, 
/**
 * None means a legacy record whose startup history is unknown.
 */
startupCompleted: boolean | null, hourlyUsd: number, budgetUsd: number, gpuType: string, gpuCount: number, dataCenterId: string, networkVolumeId: string | null, retainedStorageMonthlyUsd: number | null, credentialRef: string, failureReason: string | null, message: string, cleanupDueAt: number | null, };

export type Session = { id: string, name: string, projectId: string, projectPath: string, cli: CliKind, cliVersion: string, deploymentId: string, profileId: string, profileVersion: number, model: string, status: SessionStatus, createdAt: number, endedAt: number | null, conversationId: string | null, configDirectory: string, unverifiedCombination: boolean, exitCode: number | null, };

export type LocalState = { schemaVersion: number, installationId: string, settings: AppSettings, projects: Array<Project>, deployments: Array<Deployment>, sessions: Array<Session>, enabledDeploymentId: string | null, importedModels: Array<ImportedModel>, };

export type AppSnapshot = { state: LocalState, catalog: Array<ResolvedProfile>, runpodKeyConfigured: boolean | null, catalogError: AppError | null, credentialStoreError: AppError | null, hourlyCeilingUsd: number, offlineExpiryVerified: boolean, appVersion: string, };

export type ProvisionRequest = { profileId: string, gpuType: string, dataCenterId: string, networkVolumeId: string | null, acknowledgePaidCreation: boolean, };

export type LaunchRequest = { projectId: string, cli: CliKind, name: string, allowUnverified: boolean, };

export type CliInstallation = { cli: CliKind, path: string | null, version: string | null, message: string, };

export type TerminalChunk = { sessionId: string, sequence: number, text: string, };

export type TerminalReplay = { chunks: Array<TerminalChunk>, running: boolean, };

export type RuntimeProfile = { id: string, engine: string, version: string, image: string, inferencePort: number, statusPort: number, downloadTimeoutSeconds: number, loadTimeoutSeconds: number, };

export type Sampling = { temperature: number, topP: number, };

export type ModelAccess = "public" | "gated" | "private";

export type CompatibilityEvidence = { cli: string, cliVersion: string, gpuType: string, fingerprint: string, testedAt: string, checks: Array<string>, report: string, };

export type ModelProfile = { id: string, displayName: string, schemaVersion: number, version: number, repository: string, revision: string, tokenizerRevision: string | null, servedModel: string, runtimeId: string, access: ModelAccess, gpuTypes: Array<string>, gpuCount: number, minGpuMemoryGb: number, minCpuRamGb: number, diskGb: number, 
/**
 * Zero is accepted only when deserializing legacy deployment snapshots.
 */
minCacheGb: number, quantization: string | null, contextTokens: number, maxSessions: number, sampling: Sampling, toolParser: string | null, reasoningParser: string | null, chatTemplate: string | null, modelCachePath: string, compileCachePath: string, supportsResponses: boolean, supportsMessages: boolean, evidence: Array<CompatibilityEvidence>, limitations: Array<string>, };

export type CatalogDocument = { schemaVersion: number, runtimes: Array<RuntimeProfile>, profiles: Array<ModelProfile>, };

export type ResolvedProfile = { model: ModelProfile, runtime: RuntimeProfile, fingerprint: string, };

export type GpuType = { id: string, displayName: string, memoryInGb: number, secureCloud: boolean, securePrice: number | null, };

export type GpuAvailability = { gpuTypeId: string, stockStatus: string | null, };

export type DataCenter = { id: string, name: string, location: string, gpuAvailability: Array<GpuAvailability>, };

export type NetworkVolume = { id: string, name: string, size: number, dataCenterId: string, };

export type Hardware = { gpus: Array<GpuType>, dataCenters: Array<DataCenter>, networkVolumes: Array<NetworkVolume>, };

export type HubModel = { repository: string, revision: string, access: ModelAccess, pipelineTag: string | null, libraryName: string | null, parameterCount: number | null, license: string | null, tags: Array<string>, weightSizeGb: number | null, weightFormat: string | null, isAdapter: boolean, configJson: string, cardExcerpt: string, notes: Array<string>, };

export type ModelAnalysis = { summary: string, runtimeCompatible: boolean, compatibilityReason: string, minimumTotalVramGb: number, gpuCount: number, compatibleGpuTypes: Array<string>, minCpuRamGb: number, minCacheGb: number, diskGb: number, contextMinTokens: number, contextMaxTokens: number, recommendedContextTokens: number, toolParser: string | null, reasoningParser: string | null, quantization: string | null, assumptions: Array<string>, warnings: Array<string>, };

export type GpuQuote = { gpuType: string, gpuCount: number, memoryPerGpuGb: number, hourlyUsd: number | null, availableRegions: Array<string>, };

export type ImportedModel = { id: string, sourceUrl: string, metadata: HubModel, analysis: ModelAnalysis, analyzerVersion: string, analyzedAt: number, pricesCheckedAt: number, quotes: Array<GpuQuote>, runtime: RuntimeProfile, profileVersion: number, profile: ResolvedProfile | null, };

export type ModelImportResult = { id: string, reused: boolean, };

