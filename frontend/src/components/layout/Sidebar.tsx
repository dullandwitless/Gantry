import React from 'react';
import { 
  Code2, GitFork, PlayCircle, AppWindow, Clock, 
  KeyRound, Database, Bot, GitBranch, Settings, 
  FolderTree, HardDrive, Shield
} from 'lucide-react';

interface SidebarProps {
  activeTab: string;
  setActiveTab: (tab: string) => void;
}

export const Sidebar: React.FC<SidebarProps> = ({ activeTab, setActiveTab }) => {
  const navSections = [
    {
      title: 'Studio',
      items: [
        { id: 'scripts', label: 'Scripts Studio', icon: Code2 },
        { id: 'flows', label: 'Flows Canvas', icon: GitFork },
        { id: 'apps', label: 'Apps Studio', icon: AppWindow },
      ],
    },
    {
      title: 'Operations',
      items: [
        { id: 'runs', label: 'Runs & Queue', icon: PlayCircle },
        { id: 'schedules', label: 'Schedules & Triggers', icon: Clock },
      ],
    },
    {
      title: 'Data & Config',
      items: [
        { id: 'resources', label: 'Resources & Vars', icon: KeyRound },
        { id: 'datatables', label: 'Data Tables & S3', icon: Database },
        { id: 'ai', label: 'AI & MCP Tools', icon: Bot },
      ],
    },
    {
      title: 'Management',
      items: [
        { id: 'gitsync', label: 'Git Sync', icon: GitBranch },
        { id: 'workspace-settings', label: 'Workspace Settings', icon: Settings },
        { id: 'admin', label: 'Instance Admin', icon: Shield },
      ],
    },
  ];

  return (
    <aside className="w-64 border-r border-slate-800 bg-slate-900/90 flex flex-col justify-between select-none">
      <div>
        {/* Brand Header */}
        <div className="h-14 px-4 flex items-center gap-3 border-b border-slate-800">
          <div className="w-8 h-8 rounded-lg bg-emerald-500/10 border border-emerald-500/30 flex items-center justify-center">
            <span className="text-brand-500 font-extrabold text-lg">G</span>
          </div>
          <div>
            <div className="font-bold text-sm tracking-wide text-slate-100">GANTRY</div>
            <div className="text-[10px] text-slate-400 uppercase tracking-wider font-semibold">Workflow Engine</div>
          </div>
        </div>

        {/* Navigation Groups */}
        <nav className="p-3 space-y-5">
          {navSections.map((sec) => (
            <div key={sec.title}>
              <div className="px-3 text-[10px] uppercase tracking-wider font-semibold text-slate-500 mb-1.5">
                {sec.title}
              </div>
              <div className="space-y-0.5">
                {sec.items.map((item) => {
                  const Icon = item.icon;
                  const isActive = activeTab === item.id;
                  return (
                    <button
                      key={item.id}
                      onClick={() => setActiveTab(item.id)}
                      className={`w-full flex items-center gap-3 px-3 py-2 rounded-lg text-xs font-medium transition-colors ${
                        isActive
                          ? 'bg-brand-600/15 text-brand-400 border border-brand-500/30'
                          : 'text-slate-300 hover:bg-slate-800/60 hover:text-slate-100'
                      }`}
                    >
                      <Icon className={`w-4 h-4 ${isActive ? 'text-brand-400' : 'text-slate-400'}`} />
                      <span>{item.label}</span>
                    </button>
                  );
                })}
              </div>
            </div>
          ))}
        </nav>
      </div>

      {/* Footer Info */}
      <div className="p-4 border-t border-slate-800/80 text-xs text-slate-500">
        <div className="flex items-center justify-between">
          <span>v1.855.1 (Clean-Room)</span>
          <span className="w-2 h-2 rounded-full bg-emerald-500 inline-block animate-pulse"></span>
        </div>
      </div>
    </aside>
  );
};
