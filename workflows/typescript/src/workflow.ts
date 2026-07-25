import { randomUUID } from "crypto";

export interface WorkflowStep {
  id: string;
  name: string;
  type: string;
  config: Record<string, unknown>;
}

export interface Workflow {
  id: string;
  name: string;
  steps: WorkflowStep[];
  state: "created" | "running" | "completed" | "failed";
  createdAt: string;
}

export class WorkflowEngine {
  private workflows: Map<string, Workflow> = new Map();

  create(name: string, steps: Omit<WorkflowStep, "id">[]): Workflow {
    const id = randomUUID();
    const workflow: Workflow = {
      id,
      name,
      steps: steps.map((s) => ({ ...s, id: randomUUID() })),
      state: "created",
      createdAt: new Date().toISOString(),
    };
    this.workflows.set(id, workflow);
    return workflow;
  }

  execute(workflowId: string): Workflow | undefined {
    const workflow = this.workflows.get(workflowId);
    if (!workflow) return undefined;
    workflow.state = "completed";
    return workflow;
  }

  list(): Workflow[] {
    return Array.from(this.workflows.values());
  }

  get(workflowId: string): Workflow | undefined {
    return this.workflows.get(workflowId);
  }

  delete(workflowId: string): boolean {
    return this.workflows.delete(workflowId);
  }
}
