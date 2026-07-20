export type EstimatePreset = "linear" | "fibonacci" | "squares" | "custom";

export type EstimateSystem = {
  enabled: boolean;
  preset: EstimatePreset;
  values: number[];
};

export const ESTIMATE_PRESETS: {
  label: string;
  value: EstimatePreset;
  values: number[];
}[] = [
  {
    label: "Linear (1, 2, 3, 4, 5, 6)",
    value: "linear",
    values: [1, 2, 3, 4, 5, 6],
  },
  {
    label: "Fibonacci (1, 2, 3, 5, 8, 13, 21)",
    value: "fibonacci",
    values: [1, 2, 3, 5, 8, 13, 21],
  },
  {
    label: "Squares (1, 4, 9, 16, 25, 36)",
    value: "squares",
    values: [1, 4, 9, 16, 25, 36],
  },
  { label: "Custom", value: "custom", values: [] },
];

export function presetValues(preset: EstimatePreset): number[] {
  return ESTIMATE_PRESETS.find((entry) => entry.value === preset)?.values ?? [];
}
