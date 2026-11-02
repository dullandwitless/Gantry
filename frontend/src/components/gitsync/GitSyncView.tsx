import React from 'react';
import { useWorkspace } from '../../context/WorkspaceContext';
import { useToast } from '../../context/ToastContext';
import { GitBranch, Download, Upload, CheckCircle2, RefreshCw } from 'lucide-react';
import { CleanRoomStubNotice } from '../common/CleanRoomStubNotice';

export const GitSyncView: React.FC = () => {
  const { currentWorkspace } = useWorkspace();
  const { addToast } = useToast();

  const handleExport = () => {
    addToast({ kind: 'info', title: 'Export triggered', message: `Exporting ${currentWorkspace} to Git...` });
  };

  const handleImport = () => {
    addToast({ kind: 'info', title: 'Import triggered', message: `Importing changes from Git...` });
  };

  return (
    <div className="flex flex-col h-[calc(100vh-3.5rem)] overflow-y-auto p-6">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-xl font-bold text-slate-100 flex items-center gap-2">
            <GitBranch className="w-5 h-5 text-brand-500" />
            <span>Git Sync & Pipelines</span>
          </h1>
          <p className="text-xs text-slate-400 mt-1">
            Synchronize scripts, flows, and apps with your Git repositories for code reviews and versioning.
          </p>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={handleImport}
            className="flex items-center gap-1.5 bg-slate-800 hover:bg-slate-700 text-slate-200 border border-slate-700 px-3 py-1.5 rounded-lg text-xs font-medium transition-colors"
          >
            <Download className="w-3.5 h-3.5" />
            <span>Pull from Git</span>
          </button>
          <button
            onClick={handleExport}
            className="flex items-center gap-1.5 bg-brand-600 hover:bg-brand-500 text-slate-950 font-semibold px-3 py-1.5 rounded-lg text-xs shadow transition-all"
          >
            <Upload className="w-3.5 h-3.5" />
            <span>Push to Git</span>
          </button>
        </div>
      </div>

      <CleanRoomStubNotice
        featureName="Multi-Repository Sync & Promotion Requests"
        notes="Automated multi-repository git sync and formal enterprise deployment promotion requests are proprietary upstream modules. Single repository bidirectional sync via GitHub App / git integration is supported in Gantry."
      />

      <div className="p-5 rounded-xl border border-slate-800 bg-slate-900/40 space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-sm font-bold text-slate-100">Linked Repository</h2>
            <div className="font-mono text-xs text-slate-400 mt-0.5">https://github.com/my-org/gantry-workflows (branch: main)</div>
          </div>
          <span className="flex items-center gap-1 text-[11px] px-2.5 py-1 rounded-full bg-emerald-950 text-emerald-400 border border-emerald-800 font-semibold">
            <CheckCircle2 className="w-3.5 h-3.5" />
            <span>Synchronized</span>
          </span>
        </div>

        <div className="text-xs text-slate-400">
          Last synchronization commit: <span className="font-mono text-slate-200">c0511d3 (2026-11-01 22:38 UTC)</span>
        </div>
      </div>
    </div>
  );
};
