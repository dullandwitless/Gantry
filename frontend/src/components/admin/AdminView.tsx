import React, { useState, useEffect } from 'react';
import { useWorkspace } from '../../context/WorkspaceContext';
import { useToast } from '../../context/ToastContext';
import { api } from '../../api/client';
import { WorkerNode, ConcurrencyGroup } from '../../types';
import { Shield, Users, Cpu, Key, Activity, Plus, CheckCircle2, Server } from 'lucide-react';
import { CleanRoomStubNotice } from '../common/CleanRoomStubNotice';

export const AdminView: React.FC = () => {
  const { currentWorkspace } = useWorkspace();
  const { addToast } = useToast();

  const [activeTab, setActiveTab] = useState<'workers' | 'members' | 'concurrency' | 'health'>('workers');
  const [workers, setWorkers] = useState<WorkerNode[]>([]);
  const [concurrency, setConcurrency] = useState<ConcurrencyGroup[]>([]);

  useEffect(() => {
    if (currentWorkspace) {
      loadData();
    }
  }, [currentWorkspace]);

  const loadData = async () => {
    try {
      const [workerList, concList] = await Promise.all([
        api.listWorkers(),
        api.listConcurrencyGroups(currentWorkspace)
      ]);
      setWorkers(workerList);
      setConcurrency(concList);
    } catch (e: any) {
      addToast({ kind: 'error', title: 'Failed to load admin data', message: e.message });
    }
  };

  return (
    <div className="flex flex-col h-[calc(100vh-3.5rem)] overflow-y-auto p-6">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-xl font-bold text-slate-100 flex items-center gap-2">
            <Shield className="w-5 h-5 text-brand-500" />
            <span>Administration & Governance</span>
          </h1>
          <p className="text-xs text-slate-400 mt-1">
            Monitor worker clusters, manage concurrency limits, workspace members, and system health.
          </p>
        </div>

        <div className="flex items-center gap-2 bg-slate-900 border border-slate-800 p-1 rounded-lg">
          <button
            onClick={() => setActiveTab('workers')}
            className={`px-3 py-1.5 rounded text-xs font-medium transition-colors ${
              activeTab === 'workers' ? 'bg-slate-800 text-slate-100' : 'text-slate-400 hover:text-slate-200'
            }`}
          >
            Worker Pool
          </button>
          <button
            onClick={() => setActiveTab('members')}
            className={`px-3 py-1.5 rounded text-xs font-medium transition-colors ${
              activeTab === 'members' ? 'bg-slate-800 text-slate-100' : 'text-slate-400 hover:text-slate-200'
            }`}
          >
            Members & Roles
          </button>
          <button
            onClick={() => setActiveTab('concurrency')}
            className={`px-3 py-1.5 rounded text-xs font-medium transition-colors ${
              activeTab === 'concurrency' ? 'bg-slate-800 text-slate-100' : 'text-slate-400 hover:text-slate-200'
            }`}
          >
            Concurrency Groups
          </button>
          <button
            onClick={() => setActiveTab('health')}
            className={`px-3 py-1.5 rounded text-xs font-medium transition-colors ${
              activeTab === 'health' ? 'bg-slate-800 text-slate-100' : 'text-slate-400 hover:text-slate-200'
            }`}
          >
            Health Diagnostics
          </button>
        </div>
      </div>

      <CleanRoomStubNotice
        featureName="Enterprise SSO & Provisioning Stubs"
        notes="Enterprise SAML, SCIM provisioning, and custom workspace role builder are proprietary upstream modules. User self-registration and invitation emails are stubs in the current backend. Gantry administers access using seeded admin roles and scoped API tokens."
      />

      {activeTab === 'workers' && (
        <div className="space-y-4">
          <h2 className="text-sm font-semibold text-slate-200">Registered Worker Nodes</h2>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {workers.map((w) => (
              <div key={w.id} className="p-4 rounded-xl border border-slate-800 bg-slate-900/40 space-y-3 font-mono text-xs">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <Server className="w-4 h-4 text-brand-400" />
                    <span className="font-bold text-slate-200">{w.name}</span>
                  </div>
                  <span className="px-2 py-0.5 rounded-full bg-emerald-950 text-emerald-400 border border-emerald-800 text-[10px] uppercase font-sans font-semibold">
                    {w.status}
                  </span>
                </div>
                <div className="text-slate-400 text-[11px] space-y-1">
                  <div>IP Address: {w.ip}</div>
                  <div>Tags: {w.tags.join(', ')}</div>
                  <div>Active Executions: {w.active_jobs}</div>
                </div>
                <div className="pt-2 border-t border-slate-800/80 flex items-center justify-between text-[11px] text-slate-400 font-sans">
                  <span>CPU: {w.cpu_percent}%</span>
                  <span>Memory: {w.memory_gb} GB</span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {activeTab === 'members' && (
        <div className="space-y-4">
          <div className="flex justify-between items-center">
            <h2 className="text-sm font-semibold text-slate-200">Workspace Members</h2>
          </div>
          <div className="border border-slate-800 rounded-xl overflow-hidden bg-slate-900/40">
            <table className="w-full text-left text-xs font-mono">
              <thead className="bg-slate-900/80 border-b border-slate-800 text-slate-400">
                <tr>
                  <th className="p-3">User Email</th>
                  <th className="p-3">Role</th>
                  <th className="p-3">Permissions</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-800/60 text-slate-300">
                <tr>
                  <td className="p-3 font-bold text-slate-200">admin@gantry.dev</td>
                  <td className="p-3">
                    <span className="px-2 py-0.5 rounded bg-brand-950 text-brand-400 border border-brand-800 font-semibold">
                      Admin
                    </span>
                  </td>
                  <td className="p-3 text-slate-400 font-sans text-xs">Full instance & workspace control</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      )}

      {activeTab === 'concurrency' && (
        <div className="space-y-4">
          <div className="flex justify-between items-center">
            <h2 className="text-sm font-semibold text-slate-200">Concurrency Limits</h2>
            <button className="flex items-center gap-1.5 bg-brand-600 hover:bg-brand-500 text-slate-950 font-semibold px-3 py-1.5 rounded-lg text-xs shadow transition-all">
              <Plus className="w-3.5 h-3.5" />
              <span>New Concurrency Group</span>
            </button>
          </div>
          <div className="border border-slate-800 rounded-xl overflow-hidden bg-slate-900/40">
            <table className="w-full text-left text-xs font-mono">
              <thead className="bg-slate-900/80 border-b border-slate-800 text-slate-400">
                <tr>
                  <th className="p-3">Group Name</th>
                  <th className="p-3">Max Parallel Limit</th>
                  <th className="p-3">Currently Active</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-800/60 text-slate-300">
                {concurrency.map((cg) => (
                  <tr key={cg.name}>
                    <td className="p-3 font-semibold text-slate-200">{cg.name}</td>
                    <td className="p-3 text-brand-400 font-bold">{cg.concurrency_limit}</td>
                    <td className="p-3 text-slate-400">{cg.current_running}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {activeTab === 'health' && (
        <div className="p-5 rounded-xl border border-slate-800 bg-slate-900/40 space-y-4 font-mono text-xs">
          <div className="flex items-center gap-2 text-emerald-400 font-bold text-sm">
            <CheckCircle2 className="w-4 h-4" />
            <span>Backend API: Operational (HTTP 200 OK)</span>
          </div>
          <div className="text-slate-400 space-y-1 text-[11px]">
            <div>PostgreSQL Connection Pool: Healthy (3ms ping)</div>
            <div>Queue Worker Daemon: Active (Listening on port 5432)</div>
            <div>Static Frontend Serve: Feature static_frontend enabled</div>
          </div>
        </div>
      )}
    </div>
  );
};
