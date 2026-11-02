import { 
  Script, ScriptHistory, Draft, OpenFlow, Job, ListableApp, 
  Resource, ResourceType, Variable, Schedule, HttpTrigger, 
  WorkspaceInfo, WorkspaceUser, Folder, WorkerNode, ConcurrencyGroup,
  DataTable, TableColumn, Migration, S3FileItem
} from '../types';

class GantryApiClient {
  private baseUrl: string = '';
  private token: string | null = localStorage.getItem('gantry_token');

  setToken(token: string | null) {
    this.token = token;
    if (token) {
      localStorage.setItem('gantry_token', token);
    } else {
      localStorage.removeItem('gantry_token');
    }
  }

  getToken(): string | null {
    return this.token;
  }

  private async request<T>(path: string, options: RequestInit = {}): Promise<T> {
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      ...(options.headers as Record<string, string> || {}),
    };

    if (this.token) {
      headers['Authorization'] = `Bearer ${this.token}`;
    }

    try {
      const response = await fetch(`${this.baseUrl}${path}`, {
        ...options,
        headers,
      });

      if (!response.ok) {
        // Handle Gantry clean-room refusal or error
        const text = await response.text();
        let message = `Request failed: ${response.status} ${response.statusText}`;
        try {
          const json = JSON.parse(text);
          if (json.message) message = json.message;
        } catch {
          if (text) message = text;
        }
        throw new Error(message);
      }

      const contentType = response.headers.get('content-type');
      if (contentType && contentType.includes('application/json')) {
        return (await response.json()) as T;
      }
      return (await response.text()) as unknown as T;
    } catch (err: any) {
      // In standalone development mode, log and bubble error
      console.warn(`[GantryAPI] API call to ${path} failed:`, err.message);
      throw err;
    }
  }

  // Workspaces
  async listWorkspaces(): Promise<WorkspaceInfo[]> {
    try {
      return await this.request<WorkspaceInfo[]>('/workspaces/list');
    } catch {
      return [
        { id: 'demo', name: 'Demo Workspace', owner: 'admin@gantry.dev' },
        { id: 'prod', name: 'Production', owner: 'admin@gantry.dev' }
      ];
    }
  }

  // Folders
  async listFolders(workspace: string): Promise<Folder[]> {
    try {
      return await this.request<Folder[]>(`/w/${workspace}/folders/list`);
    } catch {
      return [
        { name: 'u', summary: 'Personal user scripts' },
        { name: 'f', summary: 'Folder shared workflows' },
        { name: 'integrations', summary: 'Third-party connectors' }
      ];
    }
  }

  // Scripts
  async getScript(workspace: string, path: string): Promise<Script> {
    try {
      return await this.request<Script>(`/w/${workspace}/scripts/get/p/${path}`);
    } catch {
      return {
        path,
        summary: 'Demo Python Script',
        language: 'python3',
        content: `def main(name: str = "World", count: int = 3):\n    """Sample greeting script for Gantry\n    :param name: Name to greet\n    :param count: Number of repetitions\n    """\n    return {"message": f"Hello {name}!", "count": count}`,
        args: [
          { name: 'name', type: 'string', default: 'World', description: 'Name to greet', required: true },
          { name: 'count', type: 'integer', default: 3, description: 'Number of repetitions', required: false }
        ],
        created_by: 'admin@gantry.dev',
        edited_at: new Date().toISOString()
      };
    }
  }

  async saveScript(workspace: string, script: Partial<Script>): Promise<void> {
    await this.request(`/w/${workspace}/scripts/create`, {
      method: 'POST',
      body: JSON.stringify(script)
    });
  }

  async runScriptTest(workspace: string, path: string, args: Record<string, any>): Promise<any> {
    try {
      return await this.request(`/w/${workspace}/jobs/run_wait_result/p/${path}`, {
        method: 'POST',
        body: JSON.stringify({ args })
      });
    } catch {
      // Mock execution result for standalone preview
      return {
        status: 'completed',
        duration_ms: 245,
        result: { message: `Hello ${args.name || 'World'}!`, count: args.count || 3 },
        logs: `[${new Date().toLocaleTimeString()}] Starting script execution...\n[${new Date().toLocaleTimeString()}] Evaluated arguments.\n[${new Date().toLocaleTimeString()}] Succeeded in 245ms.`
      };
    }
  }

  // Flows
  async getFlow(workspace: string, path: string): Promise<OpenFlow> {
    try {
      return await this.request<OpenFlow>(`/w/${workspace}/flows/get/p/${path}`);
    } catch {
      return {
        path,
        summary: 'Demo Ingestion Pipeline',
        value: {
          modules: [
            {
              id: 'fetch_data',
              summary: 'Fetch records from source',
              value: { type: 'rawscript', content: 'def main():\n    return [{"id": 1, "val": "A"}, {"id": 2, "val": "B"}]', language: 'python3' }
            },
            {
              id: 'process_items',
              summary: 'Process items in loop',
              value: { type: 'forloopflow', iterator: 'results.fetch_data' },
              input_transforms: {
                iterator: { type: 'javascript', value: 'results.fetch_data' }
              }
            }
          ]
        }
      };
    }
  }

  async saveFlow(workspace: string, flow: Partial<OpenFlow>): Promise<void> {
    await this.request(`/w/${workspace}/flows/create`, {
      method: 'POST',
      body: JSON.stringify(flow)
    });
  }

  // Jobs & Runs
  async listJobs(workspace: string, status?: string): Promise<Job[]> {
    try {
      return await this.request<Job[]>(`/w/${workspace}/jobs/list_search`, {
        method: 'POST',
        body: JSON.stringify({ status })
      });
    } catch {
      return [
        {
          id: 'job_01j7abc123',
          workspace,
          path: 'f/etl/sync_customers',
          runnable_kind: 'flow',
          status: 'completed',
          started_at: new Date(Date.now() - 120000).toISOString(),
          completed_at: new Date(Date.now() - 45000).toISOString(),
          duration_ms: 75000,
          created_by: 'admin@gantry.dev',
          trigger_kind: 'schedule',
          result: { imported: 1240, status: 'ok' },
          raw_logs: 'Sync started at 00:00:00\nConnected to PostgreSQL\n1240 records processed.\nComplete.'
        },
        {
          id: 'job_01j7abc456',
          workspace,
          path: 'u/alice/generate_pdf',
          runnable_kind: 'script',
          status: 'running',
          started_at: new Date(Date.now() - 15000).toISOString(),
          created_by: 'alice@gantry.dev',
          trigger_kind: 'webhook',
          raw_logs: 'Rendering HTML template...\nApplying CSS stylesheet...'
        }
      ];
    }
  }

  async cancelJob(workspace: string, id: string): Promise<void> {
    await this.request(`/w/${workspace}/jobs/cancel/${id}`, { method: 'POST' });
  }

  // Apps
  async listApps(workspace: string): Promise<ListableApp[]> {
    try {
      return await this.request<ListableApp[]>(`/w/${workspace}/apps/list`);
    } catch {
      return [
        { path: 'f/ops/system_monitor', summary: 'System resource monitor', custom_path: 'monitor', is_raw: true, admit_guests: false }
      ];
    }
  }

  // Resources & Variables
  async listResources(workspace: string): Promise<Resource[]> {
    try {
      return await this.request<Resource[]>(`/w/${workspace}/resources/list`);
    } catch {
      return [
        { path: 'u/admin/main_pg', resource_type: 'postgresql', value: { host: 'localhost', port: 5432, dbname: 'windmill' } },
        { path: 'f/shared/s3_storage', resource_type: 's3', value: { bucket: 'gantry-data', region: 'us-east-1' } }
      ];
    }
  }

  async listVariables(workspace: string): Promise<Variable[]> {
    try {
      return await this.request<Variable[]>(`/w/${workspace}/variables/list`);
    } catch {
      return [
        { path: 'ENVIRONMENT', value: 'production', is_secret: false, description: 'Deployment stage' },
        { path: 'STRIPE_API_KEY', value: '••••••••••••••••', is_secret: true, description: 'Payment gateway API key' }
      ];
    }
  }

  // Schedules
  async listSchedules(workspace: string): Promise<Schedule[]> {
    try {
      return await this.request<Schedule[]>(`/w/${workspace}/schedules/list`);
    } catch {
      return [
        { path: 'nightly_backup', schedule: '0 2 * * *', timezone: 'UTC', script_path: 'f/ops/backup_db', enabled: true },
        { path: 'hourly_healthcheck', schedule: '0 * * * *', timezone: 'UTC', script_path: 'f/ops/ping_services', enabled: true }
      ];
    }
  }

  // Workers & Admin
  async listWorkers(): Promise<WorkerNode[]> {
    try {
      return await this.request<WorkerNode[]>('/workers/custom_tags');
    } catch {
      return [
        { id: 'worker-01', name: 'gantry-worker-prod-1', ip: '10.0.1.24', status: 'active', tags: ['default', 'gpu'], active_jobs: 2, cpu_percent: 24, memory_gb: 16 },
        { id: 'worker-02', name: 'gantry-worker-prod-2', ip: '10.0.1.25', status: 'idle', tags: ['default'], active_jobs: 0, cpu_percent: 4, memory_gb: 16 }
      ];
    }
  }

  async listConcurrencyGroups(workspace: string): Promise<ConcurrencyGroup[]> {
    try {
      return await this.request<ConcurrencyGroup[]>(`/w/${workspace}/concurrencyGroups/list`);
    } catch {
      return [
        { name: 'external_rate_limit', concurrency_limit: 5, current_running: 1 },
        { name: 'heavy_batch_jobs', concurrency_limit: 2, current_running: 0 }
      ];
    }
  }
}

export const api = new GantryApiClient();
