export interface AgentHandle {
  id: string;
  name: string;
  state: string;
  createdAt: string;
}

export interface TaskHandle {
  id: string;
  agentId: string;
  status: string;
  result: Record<string, unknown> | null;
  createdAt: string;
}
