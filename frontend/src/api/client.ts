import { 
  Script, ScriptHistory, Draft, OpenFlow, Job, ListableApp, AppWithLastVersion, 
  Resource, ResourceType, Variable, Schedule, HttpTrigger, 
  WorkspaceInfo, WorkspaceUser, Folder, WorkerPing, ConcurrencyGroup, ListedRunnable,
  DataTable, TableColumn, Migration, S3FileItem, GlobalUser, NewUser, AuditLogEntry, AuditLogFilters
} from '../types';

export interface SignInOptions {
  iap: boolean;
  mock_users: string[];
}

export interface UserProfile {
  email: string;
  username: string;
  name?: string;
  super_admin: boolean;
  first_time_user?: boolean;
}

class GantryApiClient {
  private baseUrl: string = '/api';
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

    const url = `${this.baseUrl}${path}`;

    try {
      const response = await fetch(url, {
        ...options,
        credentials: 'include',
        headers,
      });

      if (!response.ok) {
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

      const contentType = response.headers.get('content-type') || '';
      if (contentType.includes('text/html')) {
        throw new Error(`Endpoint ${path} returned HTML instead of expected JSON.`);
      }

      if (contentType.includes('application/json')) {
        return (await response.json()) as T;
      }
      return (await response.text()) as unknown as T;
    } catch (err: any) {
      console.warn(`[GantryAPI] Request to ${url} failed:`, err.message);
      throw err;
    }
  }

  // Authentication
  async login(email: string, password: string): Promise<string> {
    const token = await this.request<string>('/auth/login', {
      method: 'POST',
      body: JSON.stringify({ email, password }),
    });
    this.setToken(token);
    return token;
  }

  // Sign-in through Identity-Aware Proxy, or as a mock user in local development
  // (specs/identity-aware-proxy.md).
  async getSignInOptions(): Promise<SignInOptions> {
    try {
      const options = await this.request<SignInOptions>('/auth/sign_in_options');
      return { iap: options.iap === true, mock_users: Array.isArray(options.mock_users) ? options.mock_users : [] };
    } catch {
      return { iap: false, mock_users: [] };
    }
  }

  /** Exchanges the IAP assertion the proxy adds to this request for a session. */
  async iapLogin(): Promise<void> {
    this.setToken(await this.request<string>('/auth/iap', { method: 'POST' }));
  }

  async mockLogin(email: string): Promise<void> {
    this.setToken(
      await this.request<string>('/auth/mock_login', { method: 'POST', body: JSON.stringify({ email }) }),
    );
  }

  async logout(): Promise<void> {
    try {
      await this.request('/auth/logout', { method: 'POST' });
    } catch {
      // Ignore error on logout
    }
    this.setToken(null);
  }

  async whoami(): Promise<UserProfile | null> {
    try {
      return await this.request<UserProfile>('/users/whoami');
    } catch {
      return null;
    }
  }

  async isFirstTimeSetup(): Promise<boolean> {
    try {
      return await this.request<boolean>('/auth/is_first_time_setup');
    } catch {
      return false;
    }
  }

  // Passwords (specs/users-and-email.md, section 6)
  async setOwnPassword(password: string): Promise<void> {
    await this.request('/users/setpassword', { method: 'POST', body: JSON.stringify({ password }) });
  }

  async isSmtpConfigured(): Promise<boolean> {
    return (await this.request<boolean>('/auth/is_smtp_configured')) === true;
  }

  /** Returns the server's message, which is the same whether or not the account exists. */
  async requestPasswordReset(email: string): Promise<string> {
    const res = await this.request<{ message: string }>('/auth/request_password_reset', {
      method: 'POST',
      body: JSON.stringify({ email }),
    });
    return res.message;
  }

  async resetPassword(token: string, newPassword: string): Promise<void> {
    await this.request('/auth/reset_password', {
      method: 'POST',
      body: JSON.stringify({ token, new_password: newPassword }),
    });
  }

  // Audit log (specs/audit-logs.md)
  async listAuditLogs(
    workspace: string,
    filters: AuditLogFilters,
    perPage: number,
    beforeId?: number,
  ): Promise<AuditLogEntry[]> {
    const query = new URLSearchParams({ per_page: String(perPage) });
    for (const [key, value] of Object.entries(filters)) {
      if (value) query.set(key, value);
    }
    if (beforeId !== undefined) query.set('before_id', String(beforeId));
    const list = await this.request<AuditLogEntry[]>(`/w/${workspace}/audit/list?${query}`);
    return Array.isArray(list) ? list : [];
  }

  // Instance users (superadmin)
  async listGlobalUsers(): Promise<GlobalUser[]> {
    const list = await this.request<GlobalUser[]>('/users/list_as_super_admin?per_page=1000');
    return Array.isArray(list) ? list : [];
  }

  async createUser(user: NewUser): Promise<void> {
    await this.request('/users/create', { method: 'POST', body: JSON.stringify(user) });
  }

  async setPasswordOf(email: string, password: string): Promise<void> {
    await this.request(`/users/set_password_of/${encodeURIComponent(email)}`, {
      method: 'POST',
      body: JSON.stringify({ password }),
    });
  }

  // Workspaces
  async listWorkspaces(): Promise<WorkspaceInfo[]> {
    const list = await this.request<WorkspaceInfo[]>('/workspaces/list');
    return Array.isArray(list) ? list : [];
  }

  async createWorkspace(id: string, name: string): Promise<string> {
    return await this.request<string>('/workspaces/create', {
      method: 'POST',
      body: JSON.stringify({ id, name }),
    });
  }

  // Folders
  async listFolders(workspace: string): Promise<Folder[]> {
    const list = await this.request<Folder[]>(`/w/${workspace}/folders/list`);
    return Array.isArray(list) ? list : [];
  }

  // Scripts
  async listScripts(workspace: string): Promise<ListedRunnable[]> {
    const list = await this.request<ListedRunnable[]>(`/w/${workspace}/scripts/list`);
    return Array.isArray(list) ? list : [];
  }

  async getScript(workspace: string, path: string): Promise<Script> {
    return await this.request<Script>(`/w/${workspace}/scripts/get/p/${path}`);
  }

  async saveScript(workspace: string, script: Partial<Script>): Promise<void> {
    await this.request(`/w/${workspace}/scripts/create`, {
      method: 'POST',
      body: JSON.stringify(script)
    });
  }

  async runScriptTest(workspace: string, path: string, args: Record<string, any>): Promise<any> {
    return await this.request(`/w/${workspace}/jobs/run_wait_result/p/${path}`, {
      method: 'POST',
      body: JSON.stringify({ args })
    });
  }

  async listFlows(workspace: string): Promise<ListedRunnable[]> {
    const list = await this.request<ListedRunnable[]>(`/w/${workspace}/flows/list`);
    return Array.isArray(list) ? list : [];
  }

  // Flows: path in OpenAPI is /w/{workspace}/flows/get/{path} (no p/)
  async getFlow(workspace: string, path: string): Promise<OpenFlow> {
    return await this.request<OpenFlow>(`/w/${workspace}/flows/get/${path}`);
  }

  async saveFlow(workspace: string, flow: Partial<OpenFlow>): Promise<void> {
    await this.request(`/w/${workspace}/flows/create`, {
      method: 'POST',
      body: JSON.stringify(flow)
    });
  }

  // Jobs & Runs: OpenAPI endpoint is GET /w/{workspace}/jobs/list
  async listJobs(workspace: string, status?: string): Promise<Job[]> {
    const query = new URLSearchParams();
    if (status && status !== 'all') {
      if (status === 'running') {
        query.set('running', 'true');
      } else if (status === 'completed') {
        query.set('status', 'success');
      } else if (status === 'failed') {
        query.set('status', 'failure');
      }
    }
    const qStr = query.toString();
    const list = await this.request<Job[]>(`/w/${workspace}/jobs/list${qStr ? `?${qStr}` : ''}`);
    return Array.isArray(list) ? list : [];
  }

  // Job Cancel: OpenAPI endpoint is POST /w/{workspace}/jobs_u/queue/cancel/{id}
  async cancelJob(workspace: string, id: string): Promise<void> {
    await this.request(`/w/${workspace}/jobs_u/queue/cancel/${id}`, {
      method: 'POST',
      body: JSON.stringify({ reason: 'Cancelled from UI' })
    });
  }

  // Apps
  async listApps(workspace: string): Promise<ListableApp[]> {
    const list = await this.request<ListableApp[]>(`/w/${workspace}/apps/list`);
    return Array.isArray(list) ? list : [];
  }

  async getApp(workspace: string, path: string): Promise<AppWithLastVersion> {
    return await this.request<AppWithLastVersion>(`/w/${workspace}/apps/get/p/${path}`);
  }

  // Resources & Variables
  async listResources(workspace: string): Promise<Resource[]> {
    const list = await this.request<Resource[]>(`/w/${workspace}/resources/list`);
    return Array.isArray(list) ? list : [];
  }

  async listVariables(workspace: string): Promise<Variable[]> {
    const list = await this.request<Variable[]>(`/w/${workspace}/variables/list`);
    return Array.isArray(list) ? list : [];
  }

  // Schedules
  async listSchedules(workspace: string): Promise<Schedule[]> {
    const list = await this.request<Schedule[]>(`/w/${workspace}/schedules/list`);
    return Array.isArray(list) ? list : [];
  }

  // Workers: GET /workers/list returns WorkerPing[]; /workers/custom_tags is tag names only.
  async listWorkers(): Promise<WorkerPing[]> {
    const list = await this.request<WorkerPing[]>('/workers/list');
    return Array.isArray(list) ? list : [];
  }

  // Concurrency groups: OpenAPI endpoint is GET /concurrency_groups/list
  async listConcurrencyGroups(workspace: string): Promise<ConcurrencyGroup[]> {
    const list = await this.request<ConcurrencyGroup[]>('/concurrency_groups/list');
    return Array.isArray(list) ? list : [];
  }
}

export const api = new GantryApiClient();
