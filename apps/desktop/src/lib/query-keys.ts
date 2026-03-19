export const queryKeys = {
  config: () => ["config"] as const,
  status: () => ["status"] as const,
  diff:   () => ["diff"]   as const,
} as const;
