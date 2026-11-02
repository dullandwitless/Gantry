import React, { useState } from 'react';
import { WorkspaceProvider } from './context/WorkspaceContext';
import { ToastProvider } from './context/ToastContext';
import { Header } from './components/layout/Header';
import { Sidebar } from './components/layout/Sidebar';
import { CommandPalette } from './components/layout/CommandPalette';

import { ScriptsStudio } from './components/scripts/ScriptsStudio';
import { FlowsStudio } from './components/flows/FlowsStudio';
import { AppsStudio } from './components/apps/AppsStudio';
import { RunsView } from './components/runs/RunsView';
import { SchedulesTriggersView } from './components/schedules/SchedulesTriggersView';
import { ResourcesVariablesView } from './components/resources/ResourcesVariablesView';
import { DataTablesView } from './components/datatables/DataTablesView';
import { AiToolsView } from './components/ai/AiToolsView';
import { GitSyncView } from './components/gitsync/GitSyncView';
import { AdminView } from './components/admin/AdminView';

export const AppContent: React.FC = () => {
  const [activeTab, setActiveTab] = useState<string>('scripts');
  const [isSearchOpen, setIsSearchOpen] = useState<boolean>(false);

  const renderActiveView = () => {
    switch (activeTab) {
      case 'scripts':
        return <ScriptsStudio />;
      case 'flows':
        return <FlowsStudio />;
      case 'apps':
        return <AppsStudio />;
      case 'runs':
        return <RunsView />;
      case 'schedules':
        return <SchedulesTriggersView />;
      case 'resources':
        return <ResourcesVariablesView />;
      case 'datatables':
        return <DataTablesView />;
      case 'ai':
        return <AiToolsView />;
      case 'gitsync':
        return <GitSyncView />;
      case 'workspace-settings':
      case 'admin':
        return <AdminView />;
      default:
        return <ScriptsStudio />;
    }
  };

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-slate-950 font-sans text-slate-100">
      <Sidebar activeTab={activeTab} setActiveTab={setActiveTab} />
      <div className="flex-1 flex flex-col min-w-0">
        <Header onOpenSearch={() => setIsSearchOpen(true)} activeTab={activeTab} />
        <main className="flex-1 overflow-hidden relative">
          {renderActiveView()}
        </main>
      </div>

      <CommandPalette
        isOpen={isSearchOpen}
        onClose={() => setIsSearchOpen(false)}
        onNavigate={(tab) => setActiveTab(tab)}
      />
    </div>
  );
};

export default function App() {
  return (
    <ToastProvider>
      <WorkspaceProvider>
        <AppContent />
      </WorkspaceProvider>
    </ToastProvider>
  );
}
