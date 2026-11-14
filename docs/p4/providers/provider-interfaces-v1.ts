/**
 * LIA provider-neutral interfaces, contract revision 1.
 *
 * Interfaces only: this file intentionally contains no vendor SDK,
 * credential handling, model routing policy, network implementation,
 * persistence code, or tool execution authority.
 */

export type Sensitivity = "S0" | "S1" | "S2" | "S3";
export type DataLocality = "local_only" | "approved_region" | "any_approved_region";
export type ProviderModality = "text" | "image" | "audio_input" | "audio_output" | "embedding";
export type JsonPrimitive = string | number | boolean | null;
export type JsonValue = JsonPrimitive | JsonValue[] | { [key: string]: JsonValue };

export interface ScopedReference {
  referenceId: string;
  ownerDomain: string;
  scope: "account" | "project" | "task" | "device" | "session" | "external_provider";
  sensitivity: Sensitivity;
  revision: string;
  digestSha256: string;
}

export interface ContextPackageReference {
  contextId: string;
  revision: string;
  digestSha256: string;
  principalId: string;
  purpose: string;
  locality: DataLocality;
  sensitivity: Sensitivity;
  sourceReferences: ScopedReference[];
  egressDecisionId?: string;
}

export interface ProviderPolicy {
  allowedProviderIds: string[];
  allowedRegions: string[];
  locality: DataLocality;
  maximumSensitivity: Sensitivity;
  retentionClass: "transient" | "approved_task_retention";
  trainingUsePermitted: false;
  requireNoProviderTraining: boolean;
  maximumWallTimeMs: number;
  maximumInputTokens?: number;
  maximumOutputTokens?: number;
  maximumCostMicros?: number;
}

export interface ModelRequirement {
  taskClass: string;
  requiredModalities: ProviderModality[];
  requiredStructuredSchemaId?: string;
  minimumQualityProfile: string;
  maximumLatencyMs?: number;
}

export interface ModelInvocationRequest {
  invocationId: string;
  conversationId?: string;
  turnId?: string;
  objectiveId?: string;
  runId?: string;
  stepId?: string;
  principalId: string;
  principalType: "user" | "service" | "delegated_task";
  purpose: string;
  context: ContextPackageReference;
  instructionBundleId: string;
  instructionBundleRevision: string;
  requirement: ModelRequirement;
  policy: ProviderPolicy;
  input: {
    messages: Array<{
      role: "system" | "developer" | "user" | "assistant";
      content: string;
      trust: "trusted_instruction" | "user_content" | "untrusted_data";
      sourceReferenceIds?: string[];
    }>;
    responseSchemaId?: string;
    responseSchemaVersion?: number;
  };
  budget: {
    deadlineAt: string;
    maximumTurns: number;
    maximumToolCycles: number;
    maximumInputTokens?: number;
    maximumOutputTokens?: number;
    maximumCostMicros?: number;
  };
  correlationId: string;
  causationId?: string;
}

export interface ToolCallProposal {
  toolCallId: string;
  invocationId: string;
  toolId: string;
  toolSchemaVersion: number;
  argumentsJson: string;
  resourceReferenceId?: string;
  parentPlanStepId?: string;
  rationale?: string;
}

export interface NormalizedModelResponse {
  invocationId: string;
  providerId: string;
  providerModelId: string;
  providerModelVersion?: string;
  status: "completed" | "refused" | "incomplete";
  text?: string;
  structuredOutput?: JsonValue;
  toolProposals: ToolCallProposal[];
  finishReason: string;
  usage: {
    inputTokens?: number;
    outputTokens?: number;
    audioInputUnits?: number;
    audioOutputUnits?: number;
    costMicros?: number;
  };
  timing: {
    startedAt: string;
    completedAt: string;
    firstTokenLatencyMs?: number;
    totalLatencyMs: number;
  };
  providerDiagnosticId?: string;
  contextDigestSha256: string;
  instructionBundleRevision: string;
}

export type ModelStreamEvent =
  | { type: "text_delta"; invocationId: string; sequence: number; text: string }
  | { type: "completed"; invocationId: string; response: NormalizedModelResponse }
  | { type: "refused"; invocationId: string; reasonCode: string }
  | { type: "failed"; invocationId: string; failure: ProviderFailure };

export interface ProviderFailure {
  category:
    | "invalid_request"
    | "authentication"
    | "rate_limited"
    | "quota_exceeded"
    | "unavailable"
    | "timeout"
    | "cancelled"
    | "content_policy"
    | "unsupported_capability"
    | "unknown";
  retryable: boolean;
  retryAfterMs?: number;
  safeMessage: string;
  diagnosticId?: string;
}

export interface ProviderCapabilities {
  providerId: string;
  adapterVersion: string;
  available: boolean;
  regions: string[];
  modalities: ProviderModality[];
  structuredOutput: boolean;
  streamingText: boolean;
  cancellation: boolean;
  contextWindowTokens?: number;
  maxOutputTokens?: number;
  policyDescriptor: {
    retentionDisclosure: string;
    trainingUse: "never" | "contractually_disabled" | "unknown";
    processingRegions: string[];
    localProcessing: boolean;
    dataEgressDescription: string;
  };
  observedAt: string;
  validUntil: string;
}

export interface ModelProvider {
  readonly providerId: string;
  getCapabilities(signal?: AbortSignal): Promise<ProviderCapabilities>;
  invoke(
    request: ModelInvocationRequest,
    signal: AbortSignal
  ): AsyncIterable<ModelStreamEvent>;
  cancel(invocationId: string, reason: string): Promise<void>;
  health(signal?: AbortSignal): Promise<{
    available: boolean;
    reasonCode?: string;
    observedAt: string;
  }>;
}

/**
 * Speech remains an independent, optional modality pipeline.
 * Raw audio is transient by default; transcript confidence and retention
 * decisions are returned explicitly.
 */
export interface SpeechRecognitionRequest {
  recognitionId: string;
  audioReference: ScopedReference;
  languageHints: string[];
  purpose: string;
  locality: DataLocality;
  deadlineAt: string;
}

export interface SpeechRecognitionResult {
  recognitionId: string;
  transcript: string;
  confidence?: number;
  language?: string;
  segments?: Array<{ startMs: number; endMs: number; text: string; confidence?: number }>;
  providerId: string;
  providerVersion: string;
  rawAudioRetained: false;
}

export interface SpeechRecognitionProvider {
  readonly providerId: string;
  transcribe(request: SpeechRecognitionRequest, signal: AbortSignal): Promise<SpeechRecognitionResult>;
}

export interface SpeechSynthesisRequest {
  synthesisId: string;
  text: string;
  voiceProfileId: string;
  language: string;
  locality: DataLocality;
  deadlineAt: string;
}

export interface SpeechSynthesisResult {
  synthesisId: string;
  artifactReference: string;
  digestSha256: string;
  durationMs: number;
  providerId: string;
  providerVersion: string;
  retentionClass: "transient";
}

export interface SpeechSynthesisProvider {
  readonly providerId: string;
  synthesize(request: SpeechSynthesisRequest, signal: AbortSignal): Promise<SpeechSynthesisResult>;
}

export interface EmbeddingRequest {
  embeddingRequestId: string;
  texts: string[];
  modelRequirement: string;
  locality: DataLocality;
  sensitivity: Sensitivity;
}

export interface EmbeddingResult {
  embeddingRequestId: string;
  modelId: string;
  dimensions: number;
  vectors: number[][];
  providerId: string;
  providerVersion: string;
}

export interface EmbeddingProvider {
  readonly providerId: string;
  embed(request: EmbeddingRequest, signal: AbortSignal): Promise<EmbeddingResult>;
}

/**
 * Tool proposal validation/dispatch remains in the LIA-owned broker.
 * A ModelProvider must never receive an execution-capable callback.
 */
export interface ProviderNeutralModelGateway {
  invoke(
    request: ModelInvocationRequest,
    signal: AbortSignal
  ): AsyncIterable<ModelStreamEvent>;
}
