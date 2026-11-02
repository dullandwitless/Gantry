import React from 'react';
import { Settings } from 'lucide-react';
import { useWorkspace } from '../../context/WorkspaceContext';
import { AuditLogPanel } from './AuditLogPanel';

/** Settings of the current workspace. */
export const WorkspaceSettingsView: React.FC = () => {
  const { currentWorkspace } = useWorkspace();
  return (
    <div className="flex flex-col h-[calc(100vh-3.5rem)] overflow-y-auto p-6 space-y-6">
      <h1 className="text-xl font-bold text-slate-100 flex items-center gap-2">
        <Settings className="w-5 h-5 text-brand-500" />
        <span>Workspace settings</span>
        <span className="text-sm font-mono text-slate-400">{currentWorkspace}</span>
      </h1>
      <AuditLogPanel />
    </div>
  );
};
