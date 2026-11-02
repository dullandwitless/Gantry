import React, { useState, useEffect } from 'react';
import { Search, Code2, GitFork, AppWindow, KeyRound, Clock, X } from 'lucide-react';
import { useWorkspace } from '../../context/WorkspaceContext';

interface CommandPaletteProps {
  isOpen: boolean;
  onClose: () => void;
  onNavigate: (tab: string) => void;
}

export const CommandPalette: React.FC<CommandPaletteProps> = ({ isOpen, onClose, onNavigate }) => {
  const { currentWorkspace } = useWorkspace();
  const [query, setQuery] = useState('');

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        if (isOpen) onClose();
        else {
          // Open
        }
      }
      if (e.key === 'Escape' && isOpen) {
        onClose();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  const quickNav = [
    { label: 'Scripts Studio', tab: 'scripts', icon: Code2, desc: 'Write and test standalone code functions' },
    { label: 'Flows Canvas', tab: 'flows', icon: GitFork, desc: 'Visual workflow orchestration editor' },
    { label: 'Runs & Queue', tab: 'runs', icon: Clock, desc: 'Real-time job queue and execution logs' },
    { label: 'Apps Studio', tab: 'apps', icon: AppWindow, desc: 'Full-code custom application builder' },
    { label: 'Resources & Variables', tab: 'resources', icon: KeyRound, desc: 'Credentials, databases, and secrets' },
  ];

  const filtered = quickNav.filter(item => 
    item.label.toLowerCase().includes(query.toLowerCase()) || 
    item.desc.toLowerCase().includes(query.toLowerCase())
  );

  return (
    <div className="fixed inset-0 z-50 bg-slate-950/80 backdrop-blur-sm flex items-start justify-center pt-24 p-4">
      <div className="w-full max-w-lg bg-slate-900 border border-slate-700/80 rounded-2xl shadow-2xl overflow-hidden flex flex-col">
        {/* Search Input Bar */}
        <div className="p-3 border-b border-slate-800 flex items-center gap-3">
          <Search className="w-4 h-4 text-slate-400 shrink-0 ml-1" />
          <input
            type="text"
            autoFocus
            placeholder="Search runnables, flows, resources or jump to tab..."
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            className="flex-1 bg-transparent text-slate-100 placeholder-slate-500 text-xs focus:outline-none"
          />
          <button onClick={onClose} className="p-1 rounded text-slate-400 hover:text-slate-200">
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Results List */}
        <div className="max-h-80 overflow-y-auto p-2 space-y-1">
          <div className="text-[10px] font-semibold uppercase tracking-wider text-slate-500 px-3 py-1">
            Navigation
          </div>
          {filtered.map((item) => {
            const Icon = item.icon;
            return (
              <button
                key={item.tab}
                onClick={() => {
                  onNavigate(item.tab);
                  onClose();
                }}
                className="w-full flex items-center gap-3 px-3 py-2.5 rounded-xl hover:bg-slate-800/80 text-left transition-colors group"
              >
                <div className="w-7 h-7 rounded-lg bg-slate-800 border border-slate-700 flex items-center justify-center group-hover:border-brand-500/50">
                  <Icon className="w-3.5 h-3.5 text-brand-400" />
                </div>
                <div>
                  <div className="text-xs font-semibold text-slate-200">{item.label}</div>
                  <div className="text-[11px] text-slate-400">{item.desc}</div>
                </div>
              </button>
            );
          })}
        </div>

        {/* Footer */}
        <div className="p-3 bg-slate-950/60 border-t border-slate-800/80 text-[10px] text-slate-500 flex justify-between items-center">
          <span>Current workspace: <span className="font-mono text-slate-300 font-semibold">{currentWorkspace}</span></span>
          <span>ESC to close</span>
        </div>
      </div>
    </div>
  );
};
