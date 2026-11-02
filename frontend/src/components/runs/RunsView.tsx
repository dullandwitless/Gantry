import React, { useState, useEffect } from 'react';
import { useWorkspace } from '../../context/WorkspaceContext';
import { useToast } from '../../context/ToastContext';
import { api } from '../../api/client';
import { Job, JobStatus } from '../../types';
import { 
  PlayCircle, CheckCircle2, AlertTriangle, Clock, 
  Ban, RefreshCw, Terminal, Search, User, Filter 
} from 'lucide-react';

export const RunsView: React.FC = () => {
  const { currentWorkspace } = useWorkspace();
  const { addToast } = useToast();

  const [jobs, setJobs] = useState<Job[]>([]);
  const [selectedJobId, setSelectedJobId] = useState<string | null>(null);
  const [statusFilter, setStatusFilter] = useState<string>('all');
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [isLoading, setIsLoading] = useState<boolean>(true);

  useEffect(() => {
    loadJobs();
  }, [currentWorkspace, statusFilter]);

  const loadJobs = async () => {
    setIsLoading(true);
    try {
      const data = await api.listJobs(currentWorkspace, statusFilter === 'all' ? undefined : statusFilter);
      setJobs(data);
      if (data.length > 0 && !selectedJobId) {
        setSelectedJobId(data[0].id);
      }
    } catch (e: any) {
      addToast({ kind: 'error', title: 'Failed to fetch runs', message: e.message });
    } finally {
      setIsLoading(false);
    }
  };

  const handleCancel = async (id: string) => {
    try {
      await api.cancelJob(currentWorkspace, id);
      addToast({ kind: 'info', title: 'Cancellation requested', message: `Job ${id}` });
      loadJobs();
    } catch (e: any) {
      addToast({ kind: 'error', title: 'Cancel failed', message: e.message });
    }
  };

  const selectedJob = jobs.find(j => j.id === selectedJobId);

  const getStatusBadge = (status: JobStatus) => {
    switch (status) {
      case 'completed':
        return (
          <span className="flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-full bg-emerald-950/80 border border-emerald-800 text-emerald-400 font-medium">
            <CheckCircle2 className="w-3 h-3" />
            <span>Success</span>
          </span>
        );
      case 'running':
        return (
          <span className="flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-full bg-blue-950/80 border border-blue-800 text-blue-400 font-medium animate-pulse">
            <Clock className="w-3 h-3" />
            <span>Running</span>
          </span>
        );
      case 'failed':
        return (
          <span className="flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-full bg-rose-950/80 border border-rose-800 text-rose-400 font-medium">
            <AlertTriangle className="w-3 h-3" />
            <span>Failed</span>
          </span>
        );
      case 'canceled':
        return (
          <span className="flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-full bg-slate-800 border border-slate-700 text-slate-400 font-medium">
            <Ban className="w-3 h-3" />
            <span>Canceled</span>
          </span>
        );
      default:
        return (
          <span className="flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-full bg-amber-950/80 border border-amber-800 text-amber-400 font-medium">
            <Clock className="w-3 h-3" />
            <span>Queued</span>
          </span>
        );
    }
  };

  const filteredJobs = jobs.filter(j => 
    j.path.toLowerCase().includes(searchQuery.toLowerCase()) || 
    j.id.toLowerCase().includes(searchQuery.toLowerCase())
  );

  return (
    <div className="flex flex-col h-[calc(100vh-3.5rem)] overflow-hidden">
      {/* Top Filter Bar */}
      <div className="h-12 border-b border-slate-800 bg-slate-900/40 px-4 flex items-center justify-between">
        <div className="flex items-center gap-3">
          <PlayCircle className="w-4 h-4 text-brand-500" />
          <div className="flex items-center gap-1 text-xs">
            {['all', 'running', 'completed', 'failed', 'queued'].map(status => (
              <button
                key={status}
                onClick={() => setStatusFilter(status)}
                className={`px-3 py-1 rounded-lg capitalize transition-colors font-medium ${
                  statusFilter === status
                    ? 'bg-slate-800 text-slate-100 border border-slate-700'
                    : 'text-slate-400 hover:text-slate-200'
                }`}
              >
                {status}
              </button>
            ))}
          </div>
        </div>

        <div className="flex items-center gap-3">
          <div className="relative">
            <Search className="w-3.5 h-3.5 text-slate-500 absolute left-2.5 top-2" />
            <input
              type="text"
              placeholder="Filter by path or job ID..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="bg-slate-800 border border-slate-700 rounded-lg pl-8 pr-3 py-1 text-xs text-slate-200 placeholder-slate-500 focus:outline-none w-56"
            />
          </div>
          <button
            onClick={loadJobs}
            className="p-1.5 rounded-lg border border-slate-700 bg-slate-800 hover:bg-slate-700 text-slate-300 transition-colors"
          >
            <RefreshCw className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>

      {/* Main Split: Jobs Table on Left, Details on Right */}
      <div className="flex flex-1 overflow-hidden">
        {/* Jobs List */}
        <div className="w-1/2 border-r border-slate-800 overflow-y-auto">
          {filteredJobs.length > 0 ? (
            <div className="divide-y divide-slate-800/60">
              {filteredJobs.map((job) => {
                const isSelected = job.id === selectedJobId;
                return (
                  <div
                    key={job.id}
                    onClick={() => setSelectedJobId(job.id)}
                    className={`p-3.5 cursor-pointer transition-colors ${
                      isSelected ? 'bg-slate-800/60 border-l-2 border-brand-500' : 'hover:bg-slate-900/50'
                    }`}
                  >
                    <div className="flex items-center justify-between mb-1.5">
                      <span className="font-mono text-xs font-semibold text-slate-200 truncate max-w-[280px]">
                        {job.path}
                      </span>
                      {getStatusBadge(job.status)}
                    </div>
                    <div className="flex items-center justify-between text-[11px] text-slate-400 font-mono">
                      <span>ID: {job.id}</span>
                      <span>{job.duration_ms ? `${job.duration_ms}ms` : 'In progress'}</span>
                    </div>
                  </div>
                );
              })}
            </div>
          ) : (
            <div className="text-center py-12 text-slate-500 text-xs">No execution runs found.</div>
          )}
        </div>

        {/* Selected Job Details Panel */}
        <div className="flex-1 bg-slate-950 flex flex-col overflow-hidden">
          {selectedJob ? (
            <div className="flex-1 flex flex-col overflow-hidden">
              {/* Job Header */}
              <div className="p-4 border-b border-slate-800 bg-slate-900/30 flex items-center justify-between">
                <div>
                  <div className="font-mono text-xs font-bold text-slate-100 flex items-center gap-2">
                    <span>{selectedJob.path}</span>
                    {getStatusBadge(selectedJob.status)}
                  </div>
                  <div className="text-[11px] text-slate-400 mt-1 flex items-center gap-3">
                    <span>Job ID: {selectedJob.id}</span>
                    <span>•</span>
                    <span>Trigger: {selectedJob.trigger_kind || 'manual'}</span>
                  </div>
                </div>

                {selectedJob.status === 'running' && (
                  <button
                    onClick={() => handleCancel(selectedJob.id)}
                    className="flex items-center gap-1.5 bg-rose-950 hover:bg-rose-900 text-rose-300 border border-rose-800 px-3 py-1.5 rounded-lg text-xs font-medium transition-colors"
                  >
                    <Ban className="w-3.5 h-3.5" />
                    <span>Cancel Run</span>
                  </button>
                )}
              </div>

              {/* Logs & Output Pane */}
              <div className="flex-1 p-4 overflow-y-auto space-y-4 font-mono text-xs">
                {selectedJob.result && (
                  <div className="space-y-1">
                    <div className="text-[11px] text-slate-400 uppercase tracking-wider font-sans font-semibold">
                      Return Value
                    </div>
                    <pre className="p-3 rounded-lg bg-slate-900 border border-slate-800 text-emerald-300 overflow-x-auto text-[11px]">
                      {JSON.stringify(selectedJob.result, null, 2)}
                    </pre>
                  </div>
                )}

                <div className="space-y-1">
                  <div className="text-[11px] text-slate-400 uppercase tracking-wider font-sans font-semibold flex items-center gap-1.5">
                    <Terminal className="w-3.5 h-3.5" />
                    <span>Standard Output / Execution Logs</span>
                  </div>
                  <pre className="p-3.5 rounded-lg bg-slate-900 border border-slate-800 text-slate-300 overflow-x-auto text-[11px] leading-relaxed">
                    {selectedJob.raw_logs || 'No log output emitted for this job.'}
                  </pre>
                </div>
              </div>
            </div>
          ) : (
            <div className="flex-1 flex items-center justify-center text-slate-500 text-xs italic">
              Select an execution run from the list to view its logs and output.
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
