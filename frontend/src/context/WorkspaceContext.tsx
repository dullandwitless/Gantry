import React, { createContext, useContext, useState, useEffect } from 'react';
import { WorkspaceInfo, Folder } from '../types';
import { api } from '../api/client';
import { useAuth } from './AuthContext';

/** The instance's one workspace (specs/single-organisation.md §2), beside `admins`. */
const ADMINS_WORKSPACE = 'admins';

interface WorkspaceContextType {
  currentWorkspace: string;
  /** The workspace's display name. */
  workspaceName: string;
  folders: Folder[];
  isLoading: boolean;
  needsWorkspaceCreation: boolean;
  refreshWorkspaces: () => Promise<void>;
  createNewWorkspace: (id: string, name: string) => Promise<void>;
}

const WorkspaceContext = createContext<WorkspaceContextType | undefined>(undefined);

export const WorkspaceProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const { isAuthenticated } = useAuth();
  const [workspace, setWorkspace] = useState<WorkspaceInfo | null>(null);
  const [folders, setFolders] = useState<Folder[]>([]);
  const [isLoading, setIsLoading] = useState<boolean>(true);

  const refreshWorkspaces = async () => {
    if (!isAuthenticated) return;
    setIsLoading(true);
    try {
      const list = await api.listWorkspaces();
      setWorkspace((Array.isArray(list) ? list : []).find((w) => w.id !== ADMINS_WORKSPACE) ?? null);
    } catch (e) {
      console.warn('[WorkspaceContext] Failed to load workspaces:', e);
      setWorkspace(null);
    } finally {
      setIsLoading(false);
    }
  };

  const createNewWorkspace = async (id: string, name: string) => {
    await api.createWorkspace(id, name);
    await refreshWorkspaces();
  };

  const currentWorkspace = workspace?.id ?? '';

  useEffect(() => {
    if (isAuthenticated) {
      refreshWorkspaces();
    }
  }, [isAuthenticated]);

  useEffect(() => {
    if (currentWorkspace && isAuthenticated) {
      api.listFolders(currentWorkspace)
        .then((f) => {
          if (Array.isArray(f)) setFolders(f);
        })
        .catch(console.warn);
    } else {
      setFolders([]);
    }
  }, [currentWorkspace, isAuthenticated]);

  return (
    <WorkspaceContext.Provider
      value={{
        currentWorkspace,
        workspaceName: workspace?.name ?? '',
        folders,
        isLoading,
        needsWorkspaceCreation: !isLoading && workspace === null,
        refreshWorkspaces,
        createNewWorkspace,
      }}
    >
      {children}
    </WorkspaceContext.Provider>
  );
};

export const useWorkspace = () => {
  const context = useContext(WorkspaceContext);
  if (!context) throw new Error('useWorkspace must be used within WorkspaceProvider');
  return context;
};
