export const DEFAULT_PROJECT_KEY = "JET";
export const DEFAULT_PROJECT_NAME = "Jet Black";

export const DEFAULT_STATES = [
  {
    color: "#71717a",
    isDefault: false,
    name: "Backlog",
    position: 1000,
    type: "backlog",
  },
  {
    color: "#3f3f46",
    isDefault: true,
    name: "Todo",
    position: 2000,
    type: "unstarted",
  },
  {
    color: "#f59e0b",
    isDefault: false,
    name: "In progress",
    position: 3000,
    type: "started",
  },
  {
    color: "#22c55e",
    isDefault: false,
    name: "Done",
    position: 4000,
    type: "completed",
  },
] as const;
