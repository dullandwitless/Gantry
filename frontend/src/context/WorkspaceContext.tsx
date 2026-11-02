import React, { createContext, useContext, useState, useEffect } from 'react';
import { WorkspaceInfo, Folder } from '../types';
import { api } from '../api/client';
import { useAuth } from './AuthContext';

interface WorkspaceContextType {
  currentWorkspace: string;
  setWorkspace: (ws: string) => void;
  workspaces: WorkspaceInfo[];
  folders: Folder[];
  isLoading: boolean;
  needsWorkspaceCreation: boolean;
  refreshWorkspaces: () => Promise<void>;
  createNewWorkspace: (id: string, name: string) => Promise<void>;
}

const WorkspaceContext = createContext<WorkspaceContextType | undefined>(undefined);

export const WorkspaceProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const { isAuthenticated } = useAuth();
  const [currentWorkspace, setCurrentWorkspace] = useState<string>('');
  const [workspaces, setWorkspaces] = useState<WorkspaceInfo[]>([]);
  const [folders, setFolders] = useState<Folder[]>([]);
  const [isLoading, setIsLoading] = useState<boolean>(true);
  const [needsWorkspaceCreation, setNeedsWorkspaceCreation] = useState<boolean>(false);

  const refreshWorkspaces = async () => {
    if (!isAuthenticated) return;
    setIsLoading(true);
    try {
      const list = await api.listWorkspaces();
      if (Array.isArray(list) && list.length > 0) {
        setWorkspaces(list);
        setNeedsWorkspaceCreation(false);
        if (!currentWorkspace || !list.some(w => w.id === currentWorkspace)) {
          setCurrentWorkspace(list[0].id);
        }
      } else {
        setWorkspaces([]);
        setNeedsWorkspaceCreation(true);
        setCurrentWorkspace('');
      }
    } catch (e) {
      console.warn('[WorkspaceContext] Failed to load workspaces:', e);
      setWorkspaces([]);
      setNeedsWorkspaceCreation(true);
    } finally {
      setIsLoading(false);
    }
  };

  const createNewWorkspace = async (id: string, name: string) => {
    await api.createWorkspace(id, name);
    setCurrentWorkspace(id);
    await refreshWorkspaces();
  };

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
        setWorkspace: setCurrentWorkspace,
        workspaces,
        folders,
        isLoading,
        needsWorkspaceCreation,
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
