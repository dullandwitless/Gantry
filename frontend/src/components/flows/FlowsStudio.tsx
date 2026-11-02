import React, { useState, useEffect } from 'react';
import { useWorkspace } from '../../context/WorkspaceContext';
import { useToast } from '../../context/ToastContext';
import { api } from '../../api/client';
import { OpenFlow, FlowModule, FlowTransform } from '../../types';
import { 
  GitFork, Play, Save, Plus, Trash2, ArrowDown, 
  Code, GitBranch, Repeat, Bot, CheckSquare, Layers 
} from 'lucide-react';

export const FlowsStudio: React.FC = () => {
  const { currentWorkspace } = useWorkspace();
  const { addToast } = useToast();

  const [flowPath, setFlowPath] = useState<string>('f/etl/customer_sync');
  const [flow, setFlow] = useState<OpenFlow | null>(null);
  const [modules, setModules] = useState<FlowModule[]>([]);
  const [selectedModuleId, setSelectedModuleId] = useState<string | null>(null);
  const [isSimulating, setIsSimulating] = useState<boolean>(false);

  useEffect(() => {
    loadFlow(flowPath);
  }, [currentWorkspace, flowPath]);

  const loadFlow = async (path: string) => {
    try {
      const data = await api.getFlow(currentWorkspace, path);
      setFlow(data);
      setModules(data.value?.modules || []);
      if (data.value?.modules?.length > 0) {
        setSelectedModuleId(data.value.modules[0].id);
      }
    } catch (e: any) {
      addToast({ kind: 'error', title: 'Failed to load flow', message: e.message });
    }
  };

  const handleAddModule = (type: FlowModule['value']['type']) => {
    const newId = `step_${modules.length + 1}`;
    const newModule: FlowModule = {
      id: newId,
      summary: `New ${type} step`,
      value: {
        type,
        path: type === 'script' ? 'u/admin/sample_script' : undefined,
        language: 'python3',
      },
    };
    setModules([...modules, newModule]);
    setSelectedModuleId(newId);
    addToast({ kind: 'info', title: 'Step added', message: `Added ${newId} (${type})` });
  };

  const handleDeleteModule = (id: string) => {
    setModules(modules.filter(m => m.id !== id));
    if (selectedModuleId === id) {
      setSelectedModuleId(modules[0]?.id || null);
    }
  };

  const handleSave = async () => {
    try {
      await api.saveFlow(currentWorkspace, {
        path: flowPath,
        value: { modules },
      });
      addToast({ kind: 'success', title: 'Flow saved', message: `Saved ${flowPath}` });
    } catch (e: any) {
      addToast({ kind: 'error', title: 'Save failed', message: e.message });
    }
  };

  const selectedModule = modules.find(m => m.id === selectedModuleId);

  const getModuleIcon = (type: FlowModule['value']['type']) => {
    switch (type) {
      case 'script':
      case 'rawscript':
        return Code;
      case 'flow':
        return Layers;
      case 'branchone':
      case 'branchall':
        return GitBranch;
      case 'forloopflow':
      case 'whileloopflow':
        return Repeat;
      case 'aiagent':
        return Bot;
      default:
        return CheckSquare;
    }
  };

  return (
    <div className="flex flex-col h-[calc(100vh-3.5rem)] overflow-hidden">
      {/* Top Action Bar */}
      <div className="h-12 border-b border-slate-800 bg-slate-900/40 px-4 flex items-center justify-between">
        <div className="flex items-center gap-3">
          <GitFork className="w-4 h-4 text-brand-500" />
          <input
            type="text"
            value={flowPath}
            onChange={(e) => setFlowPath(e.target.value)}
            className="bg-slate-800/80 border border-slate-700/60 rounded px-2.5 py-1 text-xs font-mono text-slate-200 focus:outline-none focus:border-brand-500 w-64"
          />
        </div>

        <div className="flex items-center gap-2">
          {/* Quick Add Dropdown */}
          <div className="flex items-center gap-1 bg-slate-800 border border-slate-700 rounded-lg p-0.5">
            <button
              onClick={() => handleAddModule('script')}
              className="flex items-center gap-1 hover:bg-slate-700 text-slate-200 px-2.5 py-1 rounded text-xs transition-colors"
            >
              <Code className="w-3.5 h-3.5 text-brand-400" />
              <span>+ Script</span>
            </button>
            <button
              onClick={() => handleAddModule('branchone')}
              className="flex items-center gap-1 hover:bg-slate-700 text-slate-200 px-2.5 py-1 rounded text-xs transition-colors"
            >
              <GitBranch className="w-3.5 h-3.5 text-amber-400" />
              <span>+ Branch</span>
            </button>
            <button
              onClick={() => handleAddModule('forloopflow')}
              className="flex items-center gap-1 hover:bg-slate-700 text-slate-200 px-2.5 py-1 rounded text-xs transition-colors"
            >
              <Repeat className="w-3.5 h-3.5 text-blue-400" />
              <span>+ Loop</span>
            </button>
            <button
              onClick={() => handleAddModule('aiagent')}
              className="flex items-center gap-1 hover:bg-slate-700 text-slate-200 px-2.5 py-1 rounded text-xs transition-colors"
            >
              <Bot className="w-3.5 h-3.5 text-purple-400" />
              <span>+ AI Agent</span>
            </button>
          </div>

          <button
            onClick={handleSave}
            className="flex items-center gap-1.5 bg-brand-600 hover:bg-brand-500 text-slate-950 font-semibold px-3 py-1.5 rounded-lg text-xs shadow transition-all"
          >
            <Save className="w-3.5 h-3.5" />
            <span>Publish Flow</span>
          </button>
        </div>
      </div>

      {/* Main Flow Canvas & Inspector */}
      <div className="flex flex-1 overflow-hidden">
        {/* Flow Canvas */}
        <div className="flex-1 bg-slate-950 p-8 overflow-y-auto flex flex-col items-center">
          <div className="w-full max-w-xl space-y-4">
            {modules.map((m, index) => {
              const Icon = getModuleIcon(m.value.type);
              const isSelected = m.id === selectedModuleId;
              return (
                <React.Fragment key={m.id}>
                  <div
                    onClick={() => setSelectedModuleId(m.id)}
                    className={`p-4 rounded-xl border transition-all cursor-pointer relative ${
                      isSelected
                        ? 'bg-slate-900 border-brand-500 shadow-lg shadow-brand-500/10'
                        : 'bg-slate-900/60 border-slate-800 hover:border-slate-700'
                    }`}
                  >
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-3">
                        <div className="w-8 h-8 rounded-lg bg-slate-800 flex items-center justify-center border border-slate-700">
                          <Icon className="w-4 h-4 text-brand-400" />
                        </div>
                        <div>
                          <div className="font-mono text-xs font-semibold text-slate-200 flex items-center gap-2">
                            <span>{m.id}</span>
                            <span className="text-[10px] px-1.5 py-0.5 rounded bg-slate-800 text-slate-400 font-sans uppercase">
                              {m.value.type}
                            </span>
                          </div>
                          <div className="text-xs text-slate-400 mt-0.5">{m.summary || 'No description'}</div>
                        </div>
                      </div>

                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          handleDeleteModule(m.id);
                        }}
                        className="text-slate-500 hover:text-rose-400 p-1.5 rounded transition-colors"
                      >
                        <Trash2 className="w-4 h-4" />
                      </button>
                    </div>

                    {m.value.path && (
                      <div className="mt-3 text-[11px] font-mono text-slate-400 bg-slate-950/60 px-2.5 py-1 rounded border border-slate-800/80">
                        Runnable: {m.value.path}
                      </div>
                    )}
                  </div>

                  {index < modules.length - 1 && (
                    <div className="flex justify-center py-1">
                      <ArrowDown className="w-4 h-4 text-slate-600" />
                    </div>
                  )}
                </React.Fragment>
              );
            })}

            {modules.length === 0 && (
              <div className="text-center py-12 border-2 border-dashed border-slate-800 rounded-xl text-slate-500 text-xs">
                No steps in flow. Use the "+ Script", "+ Branch", or "+ Loop" buttons above to build your pipeline.
              </div>
            )}
          </div>
        </div>

        {/* Step Inspector Panel */}
        <div className="w-96 bg-slate-900/70 border-l border-slate-800 flex flex-col">
          <div className="p-4 border-b border-slate-800 text-xs font-semibold uppercase tracking-wider text-slate-400">
            Step Inspector
          </div>
          {selectedModule ? (
            <div className="p-4 space-y-5 overflow-y-auto flex-1 text-xs">
              <div className="space-y-1">
                <label className="text-slate-300 font-medium">Step Identifier</label>
                <input
                  type="text"
                  value={selectedModule.id}
                  disabled
                  className="w-full bg-slate-950 border border-slate-800 rounded px-2.5 py-1.5 text-xs text-slate-400 font-mono"
                />
              </div>

              <div className="space-y-1">
                <label className="text-slate-300 font-medium">Step Summary</label>
                <input
                  type="text"
                  value={selectedModule.summary || ''}
                  onChange={(e) => {
                    const updated = modules.map(m => m.id === selectedModule.id ? { ...m, summary: e.target.value } : m);
                    setModules(updated);
                  }}
                  className="w-full bg-slate-950 border border-slate-700/80 rounded px-2.5 py-1.5 text-xs text-slate-100 focus:outline-none focus:border-brand-500"
                />
              </div>

              {selectedModule.value.type === 'script' && (
                <div className="space-y-1">
                  <label className="text-slate-300 font-medium">Target Script Path</label>
                  <input
                    type="text"
                    value={selectedModule.value.path || ''}
                    onChange={(e) => {
                      const updated = modules.map(m => m.id === selectedModule.id ? {
                        ...m,
                        value: { ...m.value, path: e.target.value }
                      } : m);
                      setModules(updated);
                    }}
                    className="w-full bg-slate-950 border border-slate-700/80 rounded px-2.5 py-1.5 text-xs text-slate-100 font-mono focus:outline-none focus:border-brand-500"
                  />
                </div>
              )}

              <div className="pt-2 border-t border-slate-800 space-y-3">
                <div className="font-semibold text-slate-200">Input Transformations</div>
                <div className="p-3 rounded bg-slate-950/60 border border-slate-800 text-[11px] text-slate-400 space-y-2">
                  <div className="flex items-center justify-between">
                    <span className="font-mono text-slate-300">args.data</span>
                    <span className="text-[10px] px-1.5 py-0.5 rounded bg-brand-950 text-brand-400 border border-brand-800">
                      JavaScript
                    </span>
                  </div>
                  <input
                    type="text"
                    defaultValue="results.step_1.output"
                    className="w-full bg-slate-900 border border-slate-700 rounded px-2 py-1 font-mono text-[11px] text-slate-200"
                  />
                </div>
              </div>
            </div>
          ) : (
            <div className="p-8 text-center text-slate-500 text-xs italic">
              Select a step to inspect and configure its inputs and retry policy.
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
