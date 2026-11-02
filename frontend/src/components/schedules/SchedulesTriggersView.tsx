import React, { useState, useEffect } from 'react';
import { useWorkspace } from '../../context/WorkspaceContext';
import { useToast } from '../../context/ToastContext';
import { api } from '../../api/client';
import { Schedule } from '../../types';
import { Clock, Plus, Globe, CheckCircle2, AlertCircle, ToggleLeft, ToggleRight, Trash2 } from 'lucide-react';
import { CleanRoomStubNotice } from '../common/CleanRoomStubNotice';

export const SchedulesTriggersView: React.FC = () => {
  const { currentWorkspace } = useWorkspace();
  const { addToast } = useToast();

  const [schedules, setSchedules] = useState<Schedule[]>([]);

  useEffect(() => {
    loadSchedules();
  }, [currentWorkspace]);

  const loadSchedules = async () => {
    try {
      const list = await api.listSchedules(currentWorkspace);
      setSchedules(list);
    } catch (e: any) {
      addToast({ kind: 'error', title: 'Failed to load schedules', message: e.message });
    }
  };

  const handleToggle = (path: string, currentEnabled: boolean) => {
    setSchedules(schedules.map(s => s.path === path ? { ...s, enabled: !currentEnabled } : s));
    addToast({ kind: 'info', title: 'Schedule updated', message: `${path} ${!currentEnabled ? 'enabled' : 'disabled'}` });
  };

  return (
    <div className="flex flex-col h-[calc(100vh-3.5rem)] overflow-y-auto p-6">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-xl font-bold text-slate-100 flex items-center gap-2">
            <Clock className="w-5 h-5 text-brand-500" />
            <span>Schedules & Triggers</span>
          </h1>
          <p className="text-xs text-slate-400 mt-1">
            Automate script and flow execution with recurring cron schedules, webhooks, and event listeners.
          </p>
        </div>

        <button className="flex items-center gap-1.5 bg-brand-600 hover:bg-brand-500 text-slate-950 font-semibold px-3 py-1.5 rounded-lg text-xs shadow transition-all">
          <Plus className="w-3.5 h-3.5" />
          <span>New Schedule</span>
        </button>
      </div>

      <CleanRoomStubNotice
        featureName="Proprietary Message Queue Triggers & Inbound Email"
        notes="Kafka, AWS SQS, NATS, GCP Pub/Sub, Azure Service Bus, and inbound email triggers are proprietary upstream modules. In Gantry, time-based cron schedules, HTTP webhooks, and PostgreSQL LISTEN/NOTIFY hooks are fully supported."
      />

      <div className="space-y-4">
        <h2 className="text-sm font-semibold text-slate-200">Active Cron Schedules</h2>
        <div className="border border-slate-800 rounded-xl overflow-hidden bg-slate-900/40">
          <table className="w-full text-left text-xs">
            <thead className="bg-slate-900/80 border-b border-slate-800 text-slate-400 font-mono">
              <tr>
                <th className="p-3">Schedule Name</th>
                <th className="p-3">Cron Expression</th>
                <th className="p-3">Target Runnable</th>
                <th className="p-3">Timezone</th>
                <th className="p-3">Status</th>
                <th className="p-3 text-right">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-800/60 font-mono text-slate-300">
              {schedules.map((s) => (
                <tr key={s.path} className="hover:bg-slate-900/50">
                  <td className="p-3 font-semibold text-slate-200">{s.path}</td>
                  <td className="p-3">
                    <span className="px-2 py-0.5 rounded bg-slate-800 text-brand-400 border border-slate-700 font-bold">
                      {s.schedule}
                    </span>
                  </td>
                  <td className="p-3 text-slate-300">{s.script_path}</td>
                  <td className="p-3 text-slate-400">{s.timezone || 'UTC'}</td>
                  <td className="p-3">
                    <button
                      onClick={() => handleToggle(s.path, s.enabled)}
                      className={`flex items-center gap-1.5 text-xs font-semibold ${
                        s.enabled ? 'text-emerald-400' : 'text-slate-500'
                      }`}
                    >
                      {s.enabled ? <ToggleRight className="w-5 h-5 text-emerald-400" /> : <ToggleLeft className="w-5 h-5 text-slate-600" />}
                      <span>{s.enabled ? 'Active' : 'Paused'}</span>
                    </button>
                  </td>
                  <td className="p-3 text-right">
                    <button className="text-slate-500 hover:text-rose-400 p-1 rounded">
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
};
