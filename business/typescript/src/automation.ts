import { randomUUID } from "crypto";

export interface AutomationRule {
  id: string;
  name: string;
  trigger: string;
  condition: string;
  action: string;
  enabled: boolean;
  createdAt: string;
}

export class AutomationEngine {
  private rules: Map<string, AutomationRule> = new Map();

  addRule(name: string, trigger: string, condition: string, action: string): AutomationRule {
    const rule: AutomationRule = {
      id: randomUUID(),
      name,
      trigger,
      condition,
      action,
      enabled: true,
      createdAt: new Date().toISOString(),
    };
    this.rules.set(rule.id, rule);
    return rule;
  }

  removeRule(ruleId: string): boolean {
    return this.rules.delete(ruleId);
  }

  enableRule(ruleId: string): boolean {
    const rule = this.rules.get(ruleId);
    if (!rule) return false;
    rule.enabled = true;
    return true;
  }

  disableRule(ruleId: string): boolean {
    const rule = this.rules.get(ruleId);
    if (!rule) return false;
    rule.enabled = false;
    return true;
  }

  getRule(ruleId: string): AutomationRule | undefined {
    return this.rules.get(ruleId);
  }

  listRules(): AutomationRule[] {
    return Array.from(this.rules.values());
  }

  evaluate(trigger: string): AutomationRule[] {
    return this.listRules().filter((r) => r.enabled && r.trigger === trigger);
  }
}
