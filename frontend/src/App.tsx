import React, { useState } from 'react';
import { WorkspaceProvider, useWorkspace } from './context/WorkspaceContext';
import { ToastProvider } from './context/ToastContext';
import { AuthProvider, useAuth } from './context/AuthContext';
import { ErrorBoundary } from './components/common/ErrorBoundary';
import { LoginView } from './components/auth/LoginView';
import { RESET_PASSWORD_PATH, ResetPasswordView } from './components/auth/ResetPasswordView';
import { CreateWorkspaceModal } from './components/workspace/CreateWorkspaceModal';

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
  const { isAuthenticated, isLoading: authLoading } = useAuth();
  const { needsWorkspaceCreation, isLoading: wsLoading } = useWorkspace();
  const [activeTab, setActiveTab] = useState<string>('scripts');
  const [isSearchOpen, setIsSearchOpen] = useState<boolean>(false);
  const [resetToken, setResetToken] = useState<string | null>(() =>
    window.location.pathname === RESET_PASSWORD_PATH
      ? new URLSearchParams(window.location.search).get('token') ?? ''
      : null,
  );

  if (resetToken !== null) {
    return (
      <ResetPasswordView
        token={resetToken}
        onDone={() => {
          // Drop the token from the address bar and history before leaving.
          window.history.replaceState(null, '', '/');
          setResetToken(null);
        }}
      />
    );
  }

  if (authLoading) {
    return (
      <div className="h-screen w-screen bg-slate-950 flex flex-col items-center justify-center text-slate-400 gap-3">
        <div className="w-8 h-8 rounded-full border-2 border-brand-500 border-t-transparent animate-spin" />
        <span className="text-xs font-mono">Connecting to Gantry...</span>
      </div>
    );
  }

  if (!isAuthenticated) {
    return <LoginView />;
  }

  if (needsWorkspaceCreation) {
    return <CreateWorkspaceModal isOpen={true} />;
  }

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
    <ErrorBoundary>
      <ToastProvider>
        <AuthProvider>
          <WorkspaceProvider>
            <AppContent />
          </WorkspaceProvider>
        </AuthProvider>
      </ToastProvider>
    </ErrorBoundary>
  );
}
