// Mirror of the Rust `model.rs` wire types (camelCase JSON).

export type DataSource = "api" | "jsonl" | "hybrid";

export type PlanTier = "pro" | "max5x" | "max20x" | "team" | "unknown";

export interface TokenBreakdown {
  input: number;
  output: number;
  cacheCreation: number;
  cacheRead: number;
  total: number;
}

export interface WindowStatus {
  percentUsed: number;
  remainingPercent: number;
  resetsAt: string | null; // ISO-8601
  remainingSecs: number;
  estimated: boolean;
  tokensUsed: number | null;
  tokenLimit: number | null;
}

export interface SessionStats {
  sessionId: string | null;
  startedAt: string | null;
  tokens: TokenBreakdown;
  model: string | null;
}

export interface ModelUsage {
  model: string;
  tokens: TokenBreakdown;
  costUsd: number;
}

export interface BurnStats {
  tokensPerMin: number;
  costPerHourUsd: number;
  fiveHourEta: string | null;
  sevenDayEta: string | null;
}

export interface CostStats {
  sessionUsd: number;
  fiveHourUsd: number;
  sevenDayUsd: number;
  totalUsd: number;
}

export interface UsageSnapshot {
  generatedAt: string;
  source: DataSource;
  plan: PlanTier;
  fiveHour: WindowStatus;
  sevenDay: WindowStatus;
  sevenDayOpus: WindowStatus | null;
  sevenDaySonnet: WindowStatus | null;
  currentSession: SessionStats;
  perModel: ModelUsage[];
  burn: BurnStats;
  cost: CostStats;
  warnings: string[];
}

export interface Settings {
  refreshIntervalSecs: number;
  useApi: boolean;
  planOverride: PlanTier | null;
  planLimitOverrides: { fiveHourTokens: number; sevenDayTokens: number } | null;
  writeBackTokens: boolean;
  alwaysOnTop: boolean;
  opacity: number;
  theme: string;
  currency: string;
}

export interface WireError {
  kind: string;
  message: string;
}
