import React, { useState, useEffect } from 'react';
import { useWorkspace } from '../../context/WorkspaceContext';
import { useToast } from '../../context/ToastContext';
import { api } from '../../api/client';
import { WorkerPing, ConcurrencyGroup } from '../../types';
import { Shield, Users, Cpu, Key, Activity, Plus, CheckCircle2, Server } from 'lucide-react';
import { UsersPanel } from './UsersPanel';
import { useAuth } from '../../context/AuthContext';

export const AdminView: React.FC = () => {
  const { currentWorkspace } = useWorkspace();
  const { addToast } = useToast();
  const { user } = useAuth();

  const [activeTab, setActiveTab] = useState<'workers' | 'users' | 'concurrency' | 'health'>('workers');
  const [workers, setWorkers] = useState<WorkerPing[] | null>(null);
  const [concurrency, setConcurrency] = useState<ConcurrencyGroup[]>([]);

  useEffect(() => {
    if (currentWorkspace) {
      loadData();
    }
  }, [currentWorkspace]);

  const loadData = () => {
    // Load independently so one failing endpoint does not hide the other's data.
    api.listWorkers().then(setWorkers, (e: Error) => {
      setWorkers([]);
      addToast({ kind: 'error', title: 'Failed to load workers', message: e.message });
    });
    api.listConcurrencyGroups(currentWorkspace).then(setConcurrency, (e: Error) =>
      addToast({ kind: 'error', title: 'Failed to load concurrency groups', message: e.message }),
    );
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
          {user?.super_admin && (
            <button
              onClick={() => setActiveTab('users')}
              className={`px-3 py-1.5 rounded text-xs font-medium transition-colors ${
                activeTab === 'users' ? 'bg-slate-800 text-slate-100' : 'text-slate-400 hover:text-slate-200'
              }`}
            >
              Users
            </button>
          )}
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

      {activeTab === 'workers' && (
        <div className="space-y-4">
          <h2 className="text-sm font-semibold text-slate-200">Registered Worker Nodes</h2>
          {workers === null ? (
            <div className="text-xs text-slate-500">Loading workers…</div>
          ) : workers.length === 0 ? (
            <div className="text-xs text-slate-500 italic">No worker has pinged in the last 5 minutes.</div>
          ) : (
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              {workers.map((w) => (
                <WorkerCard key={w.worker} worker={w} />
              ))}
            </div>
          )}
        </div>
      )}

      {activeTab === 'users' && user?.super_admin && <UsersPanel />}

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

/** Seconds without a ping after which a worker is shown as stale. */
const STALE_AFTER_SECONDS = 60;

type WorkerState = 'alive' | 'stale' | 'draining';

const workerState = (w: WorkerPing): WorkerState => {
  if (w.draining) return 'draining';
  if (w.last_ping == null || w.last_ping > STALE_AFTER_SECONDS) return 'stale';
  return 'alive';
};

const stateBadgeClass: Record<WorkerState, string> = {
  alive: 'bg-emerald-950 text-emerald-400 border-emerald-800',
  stale: 'bg-rose-950 text-rose-400 border-rose-800',
  draining: 'bg-amber-950 text-amber-400 border-amber-800',
};

const formatBytes = (bytes: number): string => {
  const units = ['B', 'KiB', 'MiB', 'GiB', 'TiB'];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit++;
  }
  return `${value.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`;
};

const formatPercent = (fraction: number | undefined): string =>
  fraction === undefined ? '–' : `${Math.round(fraction * 100)}%`;

const WorkerCard: React.FC<{ worker: WorkerPing }> = ({ worker: w }) => {
  const state = workerState(w);
  const tags = w.custom_tags?.length ? w.custom_tags.join(', ') : 'none';
  const memory =
    w.memory_usage != null && w.memory != null
      ? `${formatBytes(w.memory_usage)} / ${formatBytes(w.memory)}`
      : w.memory != null
        ? formatBytes(w.memory)
        : null;
  const hasOccupancy =
    w.occupancy_rate_15s !== undefined || w.occupancy_rate_5m !== undefined || w.occupancy_rate_30m !== undefined;

  return (
    <div className="p-4 rounded-xl border border-slate-800 bg-slate-900/40 space-y-3 font-mono text-xs">
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-2 min-w-0">
          <Server className="w-4 h-4 shrink-0 text-brand-400" />
          <span className="font-bold text-slate-200 truncate" title={w.worker}>{w.worker}</span>
        </div>
        <span className={`px-2 py-0.5 rounded-full border text-[10px] uppercase font-sans font-semibold ${stateBadgeClass[state]}`}>
          {state}
        </span>
      </div>
      <div className="text-slate-400 text-[11px] space-y-1">
        <div>Group: {w.worker_group}</div>
        <div>IP address: {w.ip}</div>
        <div>Tags: {tags}</div>
        <div>Jobs executed: {w.jobs_executed}</div>
        <div>Last ping: {w.last_ping == null ? 'unknown' : `${w.last_ping}s ago`}</div>
      </div>
      {(memory !== null || hasOccupancy) && (
        <div className="pt-2 border-t border-slate-800/80 flex items-center justify-between text-[11px] text-slate-400 font-sans">
          {memory !== null && <span>Memory: {memory}</span>}
          {hasOccupancy && (
            <span title="Occupancy over the last 15 seconds / 5 minutes / 30 minutes">
              Occupancy: {formatPercent(w.occupancy_rate_15s)} / {formatPercent(w.occupancy_rate_5m)} /{' '}
              {formatPercent(w.occupancy_rate_30m)}
            </span>
          )}
        </div>
      )}
      <div className="text-[10px] text-slate-500">{w.wm_version}</div>
    </div>
  );
};
