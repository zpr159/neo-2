import { describe, it, expect } from "vitest";
import { AutomationEngine } from "../src/automation";

describe("AutomationEngine", () => {
  it("adds and lists rules", () => {
    const engine = new AutomationEngine();
    const rule = engine.addRule("notify", "email_received", "is_urgent", "send_alert");
    expect(rule.name).toBe("notify");
    expect(engine.listRules()).toHaveLength(1);
  });

  it("removes a rule", () => {
    const engine = new AutomationEngine();
    const rule = engine.addRule("r", "t", "c", "a");
    expect(engine.removeRule(rule.id)).toBe(true);
    expect(engine.listRules()).toHaveLength(0);
  });

  it("enables and disables rules", () => {
    const engine = new AutomationEngine();
    const rule = engine.addRule("r", "t", "c", "a");
    engine.disableRule(rule.id);
    expect(engine.getRule(rule.id)?.enabled).toBe(false);
    engine.enableRule(rule.id);
    expect(engine.getRule(rule.id)?.enabled).toBe(true);
  });

  it("evaluates trigger matches", () => {
    const engine = new AutomationEngine();
    engine.addRule("a", "click", "true", "log");
    engine.addRule("b", "submit", "true", "send");
    engine.addRule("c", "click", "false", "noop");
    engine.disableRule(engine.listRules()[2].id);
    const matched = engine.evaluate("click");
    expect(matched).toHaveLength(1);
    expect(matched[0].name).toBe("a");
  });

  it("returns undefined for missing rule", () => {
    const engine = new AutomationEngine();
    expect(engine.getRule("nope")).toBeUndefined();
  });
});
