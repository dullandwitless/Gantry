import React, { useState, useEffect } from 'react';
import { useWorkspace } from '../../context/WorkspaceContext';
import { useToast } from '../../context/ToastContext';
import { api } from '../../api/client';
import { Resource, Variable } from '../../types';
import { KeyRound, Shield, Plus, Lock, Globe, Database, Trash2 } from 'lucide-react';
import { CleanRoomStubNotice } from '../common/CleanRoomStubNotice';

export const ResourcesVariablesView: React.FC = () => {
  const { currentWorkspace } = useWorkspace();
  const { addToast } = useToast();

  const [activeTab, setActiveTab] = useState<'resources' | 'variables'>('resources');
  const [resources, setResources] = useState<Resource[]>([]);
  const [variables, setVariables] = useState<Variable[]>([]);

  useEffect(() => {
    loadData();
  }, [currentWorkspace]);

  const loadData = async () => {
    try {
      const [resList, varList] = await Promise.all([
        api.listResources(currentWorkspace),
        api.listVariables(currentWorkspace)
      ]);
      setResources(resList);
      setVariables(varList);
    } catch (e: any) {
      addToast({ kind: 'error', title: 'Failed to load configuration', message: e.message });
    }
  };

  return (
    <div className="flex flex-col h-[calc(100vh-3.5rem)] overflow-y-auto p-6">
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-xl font-bold text-slate-100 flex items-center gap-2">
            <KeyRound className="w-5 h-5 text-brand-500" />
            <span>Resources & Variables</span>
          </h1>
          <p className="text-xs text-slate-400 mt-1">
            Manage credentials, database connections, environment variables, and encrypted secrets.
          </p>
        </div>

        <div className="flex items-center gap-2 bg-slate-900 border border-slate-800 p-1 rounded-lg">
          <button
            onClick={() => setActiveTab('resources')}
            className={`px-3 py-1.5 rounded text-xs font-medium transition-colors ${
              activeTab === 'resources' ? 'bg-slate-800 text-slate-100' : 'text-slate-400 hover:text-slate-200'
            }`}
          >
            Resources
          </button>
          <button
            onClick={() => setActiveTab('variables')}
            className={`px-3 py-1.5 rounded text-xs font-medium transition-colors ${
              activeTab === 'variables' ? 'bg-slate-800 text-slate-100' : 'text-slate-400 hover:text-slate-200'
            }`}
          >
            Variables & Secrets
          </button>
        </div>
      </div>

      <CleanRoomStubNotice
        featureName="External Secret Vaults"
        notes="External secret backends (HashiCorp Vault, AWS Secrets Manager, Azure Key Vault) are proprietary upstream modules. In Gantry, sensitive variables and credentials are encrypted directly at rest using database encryption keys."
      />

      {activeTab === 'resources' && (
        <div className="space-y-4">
          <div className="flex justify-between items-center">
            <h2 className="text-sm font-semibold text-slate-200">Configured Resources</h2>
            <button className="flex items-center gap-1.5 bg-brand-600 hover:bg-brand-500 text-slate-950 font-semibold px-3 py-1.5 rounded-lg text-xs shadow transition-all">
              <Plus className="w-3.5 h-3.5" />
              <span>New Resource</span>
            </button>
          </div>

          <div className="border border-slate-800 rounded-xl overflow-hidden bg-slate-900/40">
            <table className="w-full text-left text-xs">
              <thead className="bg-slate-900/80 border-b border-slate-800 text-slate-400 font-mono">
                <tr>
                  <th className="p-3">Path</th>
                  <th className="p-3">Resource Type</th>
                  <th className="p-3">Connection Summary</th>
                  <th className="p-3 text-right">Actions</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-800/60 font-mono text-slate-300">
                {resources.map((res) => (
                  <tr key={res.path} className="hover:bg-slate-900/50">
                    <td className="p-3 font-semibold text-slate-200">{res.path}</td>
                    <td className="p-3">
                      <span className="px-2 py-0.5 rounded bg-slate-800 text-brand-400 border border-slate-700">
                        {res.resource_type}
                      </span>
                    </td>
                    <td className="p-3 text-slate-400 text-[11px] truncate max-w-xs">
                      {JSON.stringify(res.value)}
                    </td>
                    <td className="p-3 text-right">
                      <button className="text-slate-500 hover:text-rose-400 p-1 rounded">
                        <Trash2 className="w-3.5 h-3.5" />
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {activeTab === 'variables' && (
        <div className="space-y-4">
          <div className="flex justify-between items-center">
            <h2 className="text-sm font-semibold text-slate-200">Workspace Variables</h2>
            <button className="flex items-center gap-1.5 bg-brand-600 hover:bg-brand-500 text-slate-950 font-semibold px-3 py-1.5 rounded-lg text-xs shadow transition-all">
              <Plus className="w-3.5 h-3.5" />
              <span>New Variable</span>
            </button>
          </div>

          <div className="border border-slate-800 rounded-xl overflow-hidden bg-slate-900/40">
            <table className="w-full text-left text-xs">
              <thead className="bg-slate-900/80 border-b border-slate-800 text-slate-400 font-mono">
                <tr>
                  <th className="p-3">Variable Name</th>
                  <th className="p-3">Type</th>
                  <th className="p-3">Value</th>
                  <th className="p-3">Description</th>
                  <th className="p-3 text-right">Actions</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-800/60 font-mono text-slate-300">
                {variables.map((v) => (
                  <tr key={v.path} className="hover:bg-slate-900/50">
                    <td className="p-3 font-semibold text-slate-200">{v.path}</td>
                    <td className="p-3">
                      {v.is_secret ? (
                        <span className="flex items-center gap-1 text-amber-400">
                          <Lock className="w-3 h-3" />
                          <span>Secret</span>
                        </span>
                      ) : (
                        <span className="flex items-center gap-1 text-slate-400">
                          <Globe className="w-3 h-3" />
                          <span>Plaintext</span>
                        </span>
                      )}
                    </td>
                    <td className="p-3 text-slate-300 text-[11px]">{v.value}</td>
                    <td className="p-3 text-slate-400 font-sans text-xs">{v.description || '—'}</td>
                    <td className="p-3 text-right">
                      <button className="text-slate-500 hover:text-rose-400 p-1 rounded">
                        <Trash2 className="w-3.5 h-3.5" />
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
};
