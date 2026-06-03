export const queryKeys = {
  config: () => ["config"] as const,
  status: () => ["status"] as const,
  diff: () => ["diff"] as const,
  appSettings: () => ["app-settings"] as const,
} as const;
