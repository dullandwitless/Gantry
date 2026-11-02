import React from 'react';
import { useWorkspace } from '../../context/WorkspaceContext';
import { Search, Bell, Settings, Layers, ShieldCheck } from 'lucide-react';

interface HeaderProps {
  onOpenSearch: () => void;
  activeTab: string;
}

export const Header: React.FC<HeaderProps> = ({ onOpenSearch, activeTab }) => {
  const { currentWorkspace, setWorkspace, workspaces } = useWorkspace();

  return (
    <header className="h-14 border-b border-slate-800 bg-slate-900/60 backdrop-blur px-4 flex items-center justify-between sticky top-0 z-20">
      <div className="flex items-center gap-4">
        {/* Workspace Dropdown */}
        <div className="flex items-center gap-2 bg-slate-800/80 hover:bg-slate-800 px-3 py-1.5 rounded-lg border border-slate-700/60 text-sm transition-colors">
          <Layers className="w-4 h-4 text-brand-500" />
          <select
            value={currentWorkspace}
            onChange={(e) => setWorkspace(e.target.value)}
            className="bg-transparent text-slate-200 font-medium focus:outline-none cursor-pointer"
          >
            {workspaces.map((w) => (
              <option key={w.id} value={w.id} className="bg-slate-900 text-slate-100">
                {w.name} ({w.id})
              </option>
            ))}
          </select>
        </div>

        {/* Breadcrumb Area */}
        <div className="hidden sm:flex items-center gap-2 text-sm text-slate-400">
          <span>/</span>
          <span className="capitalize text-slate-200 font-medium">{activeTab.replace('-', ' ')}</span>
        </div>
      </div>

      <div className="flex items-center gap-3">
        {/* Quick Search Button */}
        <button
          onClick={onOpenSearch}
          className="flex items-center gap-2 bg-slate-800/60 hover:bg-slate-800 text-slate-400 hover:text-slate-200 px-3 py-1.5 rounded-lg border border-slate-700/40 text-xs transition-colors"
        >
          <Search className="w-3.5 h-3.5" />
          <span>Search runnables...</span>
          <kbd className="bg-slate-900 px-1.5 py-0.5 rounded text-[10px] text-slate-400 font-mono border border-slate-700">⌘K</kbd>
        </button>

        {/* Clean-Room Badge */}
        <div className="hidden md:flex items-center gap-1.5 text-xs px-2.5 py-1 rounded-full bg-emerald-950/60 border border-emerald-800/60 text-emerald-400 font-medium">
          <ShieldCheck className="w-3.5 h-3.5 text-emerald-400" />
          <span>Clean-Room</span>
        </div>

        {/* User Avatar */}
        <div className="w-8 h-8 rounded-full bg-gradient-to-tr from-brand-600 to-emerald-400 flex items-center justify-center font-bold text-xs text-slate-950 shadow">
          A
        </div>
      </div>
    </header>
  );
};
