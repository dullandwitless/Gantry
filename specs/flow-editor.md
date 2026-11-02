# Gantry Clean-Room Specification: Flow Canvas & Orchestration Editor

Author: cleanroom agent
Date: 2026-11-02
Sources:
- `openflow.openapi.yaml` (Complete Flow Specification: `OpenFlow`, `FlowValue`, `FlowModule`, `FlowModuleValue`, `BranchOne`, `BranchAll`, `ForloopFlow`, `WhileloopFlow`, `AiAgent`, `AiDecision`, `Identity`, `InputTransform`, `Retry`, `StopAfterIf`)
- `backend/windmill-api/openapi.yaml` (tags: `flow`, `flow_conversations`, `input`, `job`)
- Windmill Public Documentation (`https://windmill.dev/docs/flows`): Visual flow builder, branching, loops, error handlers, subflows, approvals and resume tokens
Attestation: I have not read Windmill's frontend source or proprietary code.

---

## 1. Scope & Purpose

Flows Studio is the visual workflow orchestration interface for composing multi-step automated workflows. It allows combining scripts, subflows, conditional branches, loops, human approval checkpoints, and AI agents into directed execution graphs.

---

## 2. Core User Capabilities

### 2.1 Workflow Visual Canvas
- Interactive canvas rendering steps in topological execution order:
  - Add steps at any position in the pipeline (before, after, or nested within containers).
  - Drag-and-drop or click-to-insert step creation.
  - Zoom, pan, auto-layout, and minimap controls for complex flows.

### 2.2 Step Module Types (`FlowModuleValue`)
The editor supports all modules defined in `openflow.openapi.yaml`:
1. **Script Step (`PathScript`, `RawScript`)**:
   - Reference an existing workspace script path (`PathScript`) or write inline code directly (`RawScript`).
2. **Subflow (`PathFlow`)**:
   - Nest existing flows with parameterized inputs and return value forwarding.
3. **Branching**:
   - `BranchOne`: Conditional routing (If / Else If / Else). Evaluates JavaScript expressions or static conditions to choose one branch.
   - `BranchAll`: Parallel execution branches. Executes multiple branches simultaneously and merges outputs.
4. **Loops**:
   - `ForloopFlow`: Iterate over array outputs from previous steps, with configurable parallel iteration count.
   - `WhileloopFlow`: Repeat steps until a predicate expression evaluates to false.
5. **AI Steps**:
   - `AiAgent`: Autonomous task execution given goals and available workspace tools (`AgentTool`).
   - `AiDecision`: Route flow paths using LLM natural language evaluation.
6. **Identity / Pass-Through**:
   - Format, merge, or re-structure step outputs without running an external container.
7. **Approval & Human Input Step**:
   - Suspend flow execution pending review by an operator or user group. Prompts for required form inputs before proceeding.

### 2.3 Input Transformations (`InputTransform`)
Each step module declares required inputs based on its script/flow signature. The editor provides three transformation modes for mapping inputs:
- **Static Transform (`StaticTransform`)**: Constant strings, numbers, booleans, or JSON literals.
- **JavaScript Transform (`JavascriptTransform`)**: Expression evaluated against previous step outputs (e.g., `results.step_1.customer_id`).
- **AI Transform (`AiTransform`)**: Dynamic natural language prompt that queries an LLM to generate the step input based on previous results.

### 2.4 Error Handling & Retries
- Step-level retry policies (`Retry`): Max retries, initial delay, backoff factor, and retry predicate (`RetryIf`).
- Early exit conditions (`StopAfterIf`): Terminate flow early based on expression outcome.
- Global failure handler (`failure_module`): Dedicated fallback branch executed if any step fails uncaught.

### 2.5 Testing, Simulation & Live Debugger
- "Run Flow" simulation with test input arguments.
- Real-time step progress visualization on canvas:
  - Pending (Grey) -> Running (Spinning Blue) -> Succeeded (Green) / Failed (Red) / Suspended for Approval (Amber).
- Clicking any step node inspects its exact input payload, output data, execution duration, and error trace.

---

## 3. UI Views & Component Architecture

1. **Toolbar**:
   - Flow path and summary description
   - Add Step (+) quick picker
   - Save / Publish button
   - "Test Flow" execution trigger
   - Layout toggle (Vertical Linear vs Node Graph Canvas)

2. **Main Canvas**:
   - Linear / hierarchical card layout displaying step sequence.
   - Connecting lines indicating data dependencies and branch splits.
   - Expandable / collapsible branch and loop containers.

3. **Step Inspector Drawer (Right Panel)**:
   - **Step Header**: Step ID, Title, Type indicator.
   - **Runnable Binding**: Target script or flow picker with version selector.
   - **Input Transform Matrix**: Dynamically generated input list with mode toggles (Static, Variable/Secret, Previous Step Expression).
   - **Advanced Settings**: Retries, timeout, cache TTL, worker group tags, mock results for testing.

---

## 4. API Endpoint Integration

| Action | HTTP Method | Endpoint Path | Notes |
| :--- | :--- | :--- | :--- |
| Get flow | `GET` | `/w/{workspace}/flows/get/{path}` | Retrieves `OpenFlow` JSON structure |
| Create flow | `POST` | `/w/{workspace}/flows/create` | `{ path, value: FlowValue, summary }` |
| Update flow | `POST` | `/w/{workspace}/flows/update/{path}` | Updates flow definition |
| Delete flow | `DELETE` | `/w/{workspace}/flows/delete/{path}` | Soft-deletes flow to trash |
| Execute flow | `POST` | `/w/{workspace}/jobs/run/f/{path}` | Starts async flow execution |
| List approvals | `GET` | `/w/{workspace}/flow_conversations/list` | Returns pending approval requests |
| Resume approval | `POST` | `/w/{workspace}/flow_conversations/resume` | Resumes suspended flow step with payload |
