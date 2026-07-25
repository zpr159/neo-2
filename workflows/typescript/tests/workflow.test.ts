import { describe, it, expect } from "vitest";
import { WorkflowEngine } from "../src/workflow";
import { VERSION } from "../src/index";

describe("WorkflowEngine", () => {
  it("exports correct version", () => {
    expect(VERSION).toBe("0.1.0");
  });

  it("creates a workflow", () => {
    const engine = new WorkflowEngine();
    const wf = engine.create("test", [{ name: "step1", type: "http", config: {} }]);
    expect(wf.name).toBe("test");
    expect(wf.steps).toHaveLength(1);
    expect(wf.state).toBe("created");
  });

  it("lists workflows", () => {
    const engine = new WorkflowEngine();
    engine.create("a", []);
    engine.create("b", []);
    expect(engine.list()).toHaveLength(2);
  });

  it("executes a workflow", () => {
    const engine = new WorkflowEngine();
    const wf = engine.create("run", []);
    const result = engine.execute(wf.id);
    expect(result?.state).toBe("completed");
  });

  it("returns undefined for missing workflow", () => {
    const engine = new WorkflowEngine();
    expect(engine.execute("nonexistent")).toBeUndefined();
  });

  it("deletes a workflow", () => {
    const engine = new WorkflowEngine();
    const wf = engine.create("del", []);
    expect(engine.delete(wf.id)).toBe(true);
    expect(engine.list()).toHaveLength(0);
  });
});
