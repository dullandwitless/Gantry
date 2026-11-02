import React, { createContext, useContext, useState, useEffect } from 'react';
import { WorkspaceInfo, Folder } from '../types';
import { api } from '../api/client';

interface WorkspaceContextType {
  currentWorkspace: string;
  setWorkspace: (ws: string) => void;
  workspaces: WorkspaceInfo[];
  folders: Folder[];
  isLoading: boolean;
  refreshWorkspaces: () => Promise<void>;
}

const WorkspaceContext = createContext<WorkspaceContextType | undefined>(undefined);

export const WorkspaceProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [currentWorkspace, setCurrentWorkspace] = useState<string>('demo');
  const [workspaces, setWorkspaces] = useState<WorkspaceInfo[]>([
    { id: 'demo', name: 'Demo Workspace', owner: 'admin@windmill.dev' }
  ]);
  const [folders, setFolders] = useState<Folder[]>([]);
  const [isLoading, setIsLoading] = useState<boolean>(true);

  const refreshWorkspaces = async () => {
    try {
      const list = await api.listWorkspaces();
      if (Array.isArray(list) && list.length > 0) {
        setWorkspaces(list);
        if (!list.some(w => w.id === currentWorkspace)) {
          setCurrentWorkspace(list[0].id);
        }
      } else {
        setWorkspaces([{ id: 'demo', name: 'Demo Workspace', owner: 'admin@windmill.dev' }]);
      }
    } catch (e) {
      console.warn('[WorkspaceContext] Failed to load workspaces, falling back:', e);
      setWorkspaces([{ id: 'demo', name: 'Demo Workspace', owner: 'admin@windmill.dev' }]);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    refreshWorkspaces();
  }, []);

  useEffect(() => {
    if (currentWorkspace) {
      api.listFolders(currentWorkspace)
        .then((f) => {
          if (Array.isArray(f)) setFolders(f);
        })
        .catch(console.warn);
    }
  }, [currentWorkspace]);

  return (
    <WorkspaceContext.Provider
      value={{
        currentWorkspace,
        setWorkspace: setCurrentWorkspace,
        workspaces,
        folders,
        isLoading,
        refreshWorkspaces,
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
