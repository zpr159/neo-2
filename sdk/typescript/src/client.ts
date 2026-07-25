import { randomUUID } from "crypto";
import type { AgentHandle, TaskHandle } from "./types";

export class NeoClient {
  private connected: boolean = false;
  private readonly host: string;
  private readonly port: number;
  private readonly apiKey: string | null;

  constructor(host: string = "localhost", port: number = 8080, apiKey: string | null = null) {
    this.host = host;
    this.port = port;
    this.apiKey = apiKey;
  }

  connect(): void {
    this.connected = true;
  }

  disconnect(): void {
    this.connected = false;
  }

  isConnected(): boolean {
    return this.connected;
  }

  createAgent(name: string, config?: Record<string, unknown>): AgentHandle {
    return {
      id: randomUUID(),
      name,
      state: "running",
      createdAt: new Date().toISOString(),
    };
  }

  submitTask(agentId: string, task: Record<string, unknown>): TaskHandle {
    return {
      id: randomUUID(),
      agentId,
      status: "submitted",
      result: null,
      createdAt: new Date().toISOString(),
    };
  }

  listAgents(): AgentHandle[] {
    return [];
  }

  health(): { status: string; connected: boolean } {
    return { status: "ok", connected: this.connected };
  }
}
