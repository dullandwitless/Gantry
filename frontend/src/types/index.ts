export type ScriptLang = 'python3' | 'deno' | 'bun' | 'go' | 'bash' | 'sql' | 'rust' | 'php';

export interface ScriptArg {
  name: string;
  type: string;
  description?: string;
  default?: any;
  required?: boolean;
}

export interface ScriptArgs {
  args: ScriptArg[];
}

export interface Script {
  path: string;
  summary?: string;
  description?: string;
  content: string;
  language: ScriptLang;
  created_at?: string;
  created_by?: string;
  edited_at?: string;
  is_template?: boolean;
  is_locked?: boolean;
  args?: ScriptArg[];
  hash?: string;
  tag?: string;
}

export interface ScriptHistory {
  hash: string;
  created_at: string;
  created_by: string;
  summary?: string;
  content: string;
}

export interface Draft {
  path: string;
  kind: 'script' | 'flow' | 'app';
  value: string;
  updated_at: string;
}

// Flow Types based on openflow.openapi.yaml
export type TransformMode = 'static' | 'javascript' | 'ai';

export interface FlowTransform {
  type: TransformMode;
  value: any;
}

export interface FlowModule {
  id: string;
  summary?: string;
  value: {
    type: 'rawscript' | 'script' | 'flow' | 'branchone' | 'branchall' | 'forloopflow' | 'whileloopflow' | 'aiagent' | 'identity';
    path?: string;
    content?: string;
    language?: ScriptLang;
    branches?: FlowModule[][];
    modules?: FlowModule[];
    iterator?: string;
    predicate?: string;
  };
  input_transforms?: Record<string, FlowTransform>;
  retry?: {
    max_retries: number;
    delay_ms: number;
  };
  stop_after_if?: string;
}

export interface FlowValue {
  modules: FlowModule[];
  failure_module?: FlowModule;
}

export interface OpenFlow {
  path: string;
  summary?: string;
  description?: string;
  value: FlowValue;
  created_at?: string;
  created_by?: string;
  edited_at?: string;
}

export interface FlowStatusModule {
  id: string;
  status: 'queued' | 'running' | 'success' | 'failure';
  duration_ms?: number;
  error?: string;
}

export interface FlowStatus {
  step_statuses: Record<string, FlowStatusModule>;
}

// Job & Run Types
export type JobStatus = 'queued' | 'running' | 'completed' | 'failed' | 'canceled' | 'suspended';

export interface Job {
  id: string;
  workspace: string;
  path: string;
  runnable_kind: 'script' | 'flow' | 'raw_app';
  status: JobStatus;
  started_at?: string;
  completed_at?: string;
  duration_ms?: number;
  created_by?: string;
  trigger_kind?: string;
  args?: Record<string, any>;
  result?: any;
  error?: string;
  raw_logs?: string;
  flow_status?: FlowStatus;
}

// App Types
export interface ListableApp {
  path: string;
  summary?: string;
  created_at?: string;
  created_by?: string;
  edited_at?: string;
  is_raw?: boolean;
  custom_path?: string;
  admit_guests?: boolean;
}

// Resource Types
export interface ResourceType {
  name: string;
  description?: string;
  schema: Record<string, any>;
}

export interface Resource {
  path: string;
  resource_type: string;
  value: Record<string, any>;
  description?: string;
  edited_at?: string;
  edited_by?: string;
}

// Variable Types
export interface Variable {
  path: string;
  value?: string;
  is_secret: boolean;
  description?: string;
  edited_at?: string;
  expires_at?: string;
}

// Schedule & Trigger Types
export interface Schedule {
  path: string;
  schedule: string;
  timezone?: string;
  script_path: string;
  args?: Record<string, any>;
  enabled: boolean;
  edited_at?: string;
}

export interface HttpTrigger {
  path: string;
  route: string;
  http_method: 'GET' | 'POST' | 'PUT' | 'DELETE';
  script_path: string;
  is_public: boolean;
}

// Data Tables & Storage Types
export interface DataTable {
  name: string;
  table_count?: number;
  schema?: string;
}

export interface TableColumn {
  name: string;
  data_type: string;
  is_nullable: boolean;
  is_primary_key: boolean;
}

export interface Migration {
  timestamp: string;
  name: string;
  applied: boolean;
}

export interface S3FileItem {
  key: string;
  size_bytes: number;
  last_modified: string;
  is_directory: boolean;
}

// Workspace & Governance
export interface WorkspaceInfo {
  id: string;
  name: string;
  owner?: string;
}

export interface WorkspaceUser {
  email: string;
  role: 'admin' | 'member' | 'viewer' | 'operator';
  is_admin: boolean;
  created_at?: string;
}

export interface Folder {
  name: string;
  summary?: string;
  owners?: string[];
}

export interface WorkerNode {
  id: string;
  name: string;
  ip: string;
  status: 'active' | 'idle' | 'offline';
  tags: string[];
  active_jobs: number;
  cpu_percent?: number;
  memory_gb?: number;
}

export interface ConcurrencyGroup {
  name: string;
  concurrency_limit: number;
  current_running: number;
}
