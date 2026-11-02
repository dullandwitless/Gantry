import React, { useState, useEffect } from 'react';
import { useWorkspace } from '../../context/WorkspaceContext';
import { useToast } from '../../context/ToastContext';
import { api } from '../../api/client';
import { Script, ScriptLang, ScriptArg } from '../../types';
import { Play, Save, Clock, Lock, FileCode, CheckCircle2, AlertTriangle, Terminal } from 'lucide-react';
import { useRunnablePicker } from '../common/useRunnablePicker';
import { RunnablePathField } from '../common/RunnablePathField';

const listScripts = (workspace: string) => api.listScripts(workspace);

export const ScriptsStudio: React.FC = () => {
  const { currentWorkspace } = useWorkspace();
  const { addToast } = useToast();

  const picker = useRunnablePicker(currentWorkspace, listScripts, (e) =>
    addToast({ kind: 'error', title: 'Failed to list scripts', message: e.message }),
  );
  const { selection } = picker;

  const [newPath, setNewPath] = useState<string>('');
  const [script, setScript] = useState<Script | null>(null);
  const [code, setCode] = useState<string>('');
  const [language, setLanguage] = useState<ScriptLang>('python3');
  const [summary, setSummary] = useState<string>('');
  const [testArgs, setTestArgs] = useState<Record<string, any>>({});
  const [isRunning, setIsRunning] = useState<boolean>(false);
  const [runResult, setRunResult] = useState<any>(null);
  const [activeRightTab, setActiveRightTab] = useState<'arguments' | 'output' | 'settings'>('arguments');

  const scriptPath = selection?.kind === 'existing' ? selection.path : newPath.trim();

  useEffect(() => {
    setRunResult(null);
    if (selection?.kind !== 'existing') {
      setScript(null);
      setCode('');
      setLanguage('python3');
      setSummary('');
      setTestArgs({});
      setNewPath('');
      return;
    }
    let cancelled = false;
    api.getScript(currentWorkspace, selection.path).then(
      (data) => {
        if (cancelled) return;
        setScript(data);
        setCode(data.content);
        setLanguage(data.language);
        setSummary(data.summary || '');
        const initialArgs: Record<string, any> = {};
        data.args?.forEach((arg) => {
          initialArgs[arg.name] = arg.default !== undefined ? arg.default : '';
        });
        setTestArgs(initialArgs);
      },
      (e: Error) => {
        if (!cancelled) addToast({ kind: 'error', title: 'Failed to load script', message: e.message });
      },
    );
    return () => {
      cancelled = true;
    };
    // addToast is recreated on every render; reloading must depend only on what is open.
  }, [currentWorkspace, selection]);

  const handleSave = async () => {
    if (!scriptPath) {
      addToast({ kind: 'error', title: 'Path required', message: 'Name the script before deploying it.' });
      return;
    }
    try {
      await api.saveScript(currentWorkspace, {
        path: scriptPath,
        content: code,
        language,
        summary,
      });
      addToast({ kind: 'success', title: 'Script deployed', message: `Saved ${scriptPath}` });
      await picker.refresh(scriptPath);
    } catch (e: any) {
      addToast({ kind: 'error', title: 'Deploy failed', message: e.message });
    }
  };

  const handleRunTest = async () => {
    if (selection?.kind !== 'existing') return;
    setIsRunning(true);
    setActiveRightTab('output');
    try {
      const res = await api.runScriptTest(currentWorkspace, selection.path, testArgs);
      setRunResult(res);
      addToast({ kind: 'success', title: 'Execution completed', message: `Finished in ${res.duration_ms || 240}ms` });
    } catch (e: any) {
      setRunResult({ error: e.message, status: 'failed' });
      addToast({ kind: 'error', title: 'Execution failed', message: e.message });
    } finally {
      setIsRunning(false);
    }
  };

  return (
    <div className="flex flex-col h-[calc(100vh-3.5rem)] overflow-hidden">
      {/* Top Action Bar */}
      <div className="h-12 border-b border-slate-800 bg-slate-900/40 px-4 flex items-center justify-between">
        <div className="flex items-center gap-3">
          <FileCode className="w-4 h-4 text-brand-500" />
          <RunnablePathField
            noun="script"
            items={picker.items}
            selection={selection}
            onSelect={picker.select}
            newPath={newPath}
            onNewPathChange={setNewPath}
          />
          <select
            value={language}
            onChange={(e) => setLanguage(e.target.value as ScriptLang)}
            className="bg-slate-800 border border-slate-700 rounded px-2 py-1 text-xs text-slate-300 focus:outline-none"
          >
            <option value="python3">Python 3</option>
            <option value="deno">TypeScript (Deno)</option>
            <option value="bun">TypeScript (Bun)</option>
            <option value="go">Go</option>
            <option value="bash">Bash</option>
            <option value="sql">SQL</option>
            <option value="rust">Rust</option>
            <option value="php">PHP</option>
          </select>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={handleRunTest}
            disabled={isRunning || selection?.kind !== 'existing'}
            title={selection?.kind === 'existing' ? undefined : 'Deploy the script before running it'}
            className="flex items-center gap-1.5 bg-brand-600 hover:bg-brand-500 disabled:opacity-50 text-slate-950 font-semibold px-3 py-1.5 rounded-lg text-xs shadow transition-all"
          >
            <Play className="w-3.5 h-3.5 fill-current" />
            <span>{isRunning ? 'Running...' : 'Test Run'}</span>
          </button>
          <button
            onClick={handleSave}
            className="flex items-center gap-1.5 bg-slate-800 hover:bg-slate-700 text-slate-200 font-medium px-3 py-1.5 rounded-lg border border-slate-700 text-xs transition-colors"
          >
            <Save className="w-3.5 h-3.5" />
            <span>Deploy</span>
          </button>
        </div>
      </div>

      {/* Editor & Right Drawer Split */}
      <div className="flex flex-1 overflow-hidden">
        {/* Code Editor Pane */}
        <div className="flex-1 flex flex-col bg-slate-950 border-r border-slate-800">
          <div className="px-4 py-1.5 bg-slate-900/30 border-b border-slate-800/80 text-[11px] text-slate-400 font-mono flex justify-between items-center">
            <span>Editor ({language})</span>
            <span>UTF-8</span>
          </div>
          <textarea
            value={code}
            onChange={(e) => setCode(e.target.value)}
            spellCheck={false}
            className="flex-1 w-full bg-slate-950 text-slate-100 font-mono text-xs p-4 resize-none focus:outline-none selection:bg-brand-900 leading-relaxed"
          />
        </div>

        {/* Right Drawer (Parameters, Test Output, Settings) */}
        <div className="w-96 flex flex-col bg-slate-900/60 border-l border-slate-800">
          {/* Tab Selector */}
          <div className="flex border-b border-slate-800 text-xs font-medium">
            <button
              onClick={() => setActiveRightTab('arguments')}
              className={`flex-1 py-2.5 text-center transition-colors border-b-2 ${
                activeRightTab === 'arguments'
                  ? 'border-brand-500 text-brand-400 bg-slate-800/30'
                  : 'border-transparent text-slate-400 hover:text-slate-200'
              }`}
            >
              Test Arguments
            </button>
            <button
              onClick={() => setActiveRightTab('output')}
              className={`flex-1 py-2.5 text-center transition-colors border-b-2 ${
                activeRightTab === 'output'
                  ? 'border-brand-500 text-brand-400 bg-slate-800/30'
                  : 'border-transparent text-slate-400 hover:text-slate-200'
              }`}
            >
              Execution Output
            </button>
            <button
              onClick={() => setActiveRightTab('settings')}
              className={`flex-1 py-2.5 text-center transition-colors border-b-2 ${
                activeRightTab === 'settings'
                  ? 'border-brand-500 text-brand-400 bg-slate-800/30'
                  : 'border-transparent text-slate-400 hover:text-slate-200'
              }`}
            >
              Settings
            </button>
          </div>

          {/* Drawer Body */}
          <div className="flex-1 p-4 overflow-y-auto">
            {activeRightTab === 'arguments' && (
              <div className="space-y-4">
                <div className="text-xs text-slate-400 mb-2">
                  Parameters parsed dynamically from function signature:
                </div>
                {script?.args && script.args.length > 0 ? (
                  script.args.map((arg: ScriptArg) => (
                    <div key={arg.name} className="space-y-1">
                      <div className="flex items-center justify-between text-xs">
                        <label className="font-mono text-slate-300 font-medium">{arg.name}</label>
                        <span className="text-[10px] text-slate-500 font-mono">{arg.type}</span>
                      </div>
                      {arg.description && (
                        <div className="text-[11px] text-slate-400">{arg.description}</div>
                      )}
                      <input
                        type={arg.type === 'integer' || arg.type === 'number' ? 'number' : 'text'}
                        value={testArgs[arg.name] ?? ''}
                        onChange={(e) => setTestArgs({
                          ...testArgs,
                          [arg.name]: arg.type === 'integer' || arg.type === 'number' ? Number(e.target.value) : e.target.value
                        })}
                        className="w-full bg-slate-950 border border-slate-700/80 rounded px-2.5 py-1.5 text-xs text-slate-100 font-mono focus:outline-none focus:border-brand-500"
                      />
                    </div>
                  ))
                ) : (
                  <div className="text-xs text-slate-500 italic">No input parameters detected.</div>
                )}
              </div>
            )}

            {activeRightTab === 'output' && (
              <div className="space-y-4 text-xs font-mono">
                {runResult ? (
                  <>
                    <div className="flex items-center justify-between p-2.5 rounded bg-slate-950 border border-slate-800">
                      <div className="flex items-center gap-2">
                        {runResult.error ? (
                          <AlertTriangle className="w-4 h-4 text-rose-500" />
                        ) : (
                          <CheckCircle2 className="w-4 h-4 text-emerald-400" />
                        )}
                        <span className="capitalize font-bold text-slate-200">{runResult.status || 'Success'}</span>
                      </div>
                      <span className="text-slate-400 text-[11px]">{runResult.duration_ms || 240} ms</span>
                    </div>

                    {runResult.result && (
                      <div className="space-y-1">
                        <div className="text-slate-400 text-[11px] uppercase tracking-wider font-sans font-semibold">
                          Return Value
                        </div>
                        <pre className="p-3 rounded bg-slate-950 border border-slate-800 text-emerald-300 overflow-x-auto text-[11px]">
                          {JSON.stringify(runResult.result, null, 2)}
                        </pre>
                      </div>
                    )}

                    {runResult.logs && (
                      <div className="space-y-1">
                        <div className="text-slate-400 text-[11px] uppercase tracking-wider font-sans font-semibold flex items-center gap-1.5">
                          <Terminal className="w-3.5 h-3.5" />
                          <span>Standard Output / Error</span>
                        </div>
                        <pre className="p-3 rounded bg-slate-950 border border-slate-800 text-slate-300 overflow-x-auto text-[11px] leading-relaxed">
                          {runResult.logs}
                        </pre>
                      </div>
                    )}
                  </>
                ) : (
                  <div className="text-slate-500 italic text-center py-8">
                    Click "Test Run" to execute this script with sample arguments.
                  </div>
                )}
              </div>
            )}

            {activeRightTab === 'settings' && (
              <div className="space-y-4 text-xs">
                <div className="space-y-1">
                  <label className="text-slate-300 font-medium">Summary Description</label>
                  <input
                    type="text"
                    value={summary}
                    onChange={(e) => setSummary(e.target.value)}
                    placeholder="Short description of what this script does..."
                    className="w-full bg-slate-950 border border-slate-700/80 rounded px-2.5 py-1.5 text-xs text-slate-100 focus:outline-none focus:border-brand-500"
                  />
                </div>
                <div className="space-y-1">
                  <label className="text-slate-300 font-medium">Worker Group Tags</label>
                  <input
                    type="text"
                    placeholder="e.g. default, gpu, high-memory"
                    className="w-full bg-slate-950 border border-slate-700/80 rounded px-2.5 py-1.5 text-xs text-slate-100 focus:outline-none focus:border-brand-500"
                  />
                </div>
                <div className="space-y-1">
                  <label className="text-slate-300 font-medium">Execution Timeout (seconds)</label>
                  <input
                    type="number"
                    defaultValue={300}
                    className="w-full bg-slate-950 border border-slate-700/80 rounded px-2.5 py-1.5 text-xs text-slate-100 focus:outline-none focus:border-brand-500"
                  />
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
