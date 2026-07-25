import { describe, it, expect } from "vitest";
import { NeoClient } from "../src/client";

describe("NeoClient", () => {
  it("creates a client with defaults", () => {
    const client = new NeoClient();
    expect(client.isConnected()).toBe(false);
  });

  it("connects and disconnects", () => {
    const client = new NeoClient();
    client.connect();
    expect(client.isConnected()).toBe(true);
    client.disconnect();
    expect(client.isConnected()).toBe(false);
  });

  it("creates an agent", () => {
    const client = new NeoClient();
    const agent = client.createAgent("TestBot");
    expect(agent.name).toBe("TestBot");
    expect(agent.state).toBe("running");
    expect(agent.id).toBeTruthy();
  });

  it("submits a task", () => {
    const client = new NeoClient();
    const task = client.submitTask("agent-1", { type: "query" });
    expect(task.agentId).toBe("agent-1");
    expect(task.status).toBe("submitted");
  });

  it("returns empty agent list", () => {
    const client = new NeoClient();
    expect(client.listAgents()).toEqual([]);
  });

  it("returns health", () => {
    const client = new NeoClient();
    expect(client.health().status).toBe("ok");
  });
});
