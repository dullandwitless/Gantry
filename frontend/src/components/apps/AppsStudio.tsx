import React, { useState, useEffect } from 'react';
import { useWorkspace } from '../../context/WorkspaceContext';
import { useToast } from '../../context/ToastContext';
import { api } from '../../api/client';
import { ListableApp } from '../../types';
import { AppWindow, ExternalLink, Code2, Database, Play, Plus, Eye } from 'lucide-react';

export const AppsStudio: React.FC = () => {
  const { currentWorkspace } = useWorkspace();
  const { addToast } = useToast();

  const [apps, setApps] = useState<ListableApp[]>([]);
  const [selectedApp, setSelectedApp] = useState<ListableApp | null>(null);
  const [activeTab, setActiveTab] = useState<'preview' | 'source' | 'data'>('preview');
  const [appSource, setAppSource] = useState<string>(`import React, { useState } from 'react';

export default function App() {
  const [count, setCount] = useState(0);

  return (
    <div style={{ padding: '2rem', fontFamily: 'sans-serif' }}>
      <h1>Gantry Custom Application</h1>
      <p>Compiled on worker, executing runnables via Gantry SDK.</p>
      <button 
        onClick={() => setCount(c => c + 1)}
        style={{ padding: '0.5rem 1rem', background: '#22c55e', color: '#000', border: 'none', borderRadius: '4px', cursor: 'pointer', fontWeight: 'bold' }}
      >
        Clicked {count} times
      </button>
    </div>
  );
}`);

  useEffect(() => {
    if (currentWorkspace) {
      loadApps();
    }
  }, [currentWorkspace]);

  const loadApps = async () => {
    try {
      const list = await api.listApps(currentWorkspace);
      setApps(list);
      if (list.length > 0) {
        setSelectedApp(list[0]);
      }
    } catch (e: any) {
      addToast({ kind: 'error', title: 'Failed to load apps', message: e.message });
    }
  };

  return (
    <div className="flex flex-col h-[calc(100vh-3.5rem)] overflow-hidden">
      {/* Top Action Bar */}
      <div className="h-12 border-b border-slate-800 bg-slate-900/40 px-4 flex items-center justify-between">
        <div className="flex items-center gap-3">
          <AppWindow className="w-4 h-4 text-brand-500" />
          <span className="font-semibold text-xs text-slate-200">Custom Applications Studio</span>
          {selectedApp && (
            <span className="text-xs font-mono text-slate-400 bg-slate-800 px-2 py-0.5 rounded border border-slate-700">
              {selectedApp.path}
            </span>
          )}
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={() => setActiveTab('preview')}
            className={`px-3 py-1.5 rounded-lg text-xs font-medium transition-colors ${
              activeTab === 'preview' ? 'bg-slate-800 text-slate-100 border border-slate-700' : 'text-slate-400 hover:text-slate-200'
            }`}
          >
            <span className="flex items-center gap-1.5">
              <Eye className="w-3.5 h-3.5" />
              <span>Live Preview</span>
            </span>
          </button>
          <button
            onClick={() => setActiveTab('source')}
            className={`px-3 py-1.5 rounded-lg text-xs font-medium transition-colors ${
              activeTab === 'source' ? 'bg-slate-800 text-slate-100 border border-slate-700' : 'text-slate-400 hover:text-slate-200'
            }`}
          >
            <span className="flex items-center gap-1.5">
              <Code2 className="w-3.5 h-3.5" />
              <span>React Source</span>
            </span>
          </button>
          <button
            onClick={() => setActiveTab('data')}
            className={`px-3 py-1.5 rounded-lg text-xs font-medium transition-colors ${
              activeTab === 'data' ? 'bg-slate-800 text-slate-100 border border-slate-700' : 'text-slate-400 hover:text-slate-200'
            }`}
          >
            <span className="flex items-center gap-1.5">
              <Database className="w-3.5 h-3.5" />
              <span>S3 Data Helpers</span>
            </span>
          </button>
        </div>
      </div>

      {/* Main Split: Left Apps List, Right Workspace */}
      <div className="flex flex-1 overflow-hidden">
        {/* Apps List Sidebar */}
        <div className="w-64 border-r border-slate-800 p-3 space-y-2 overflow-y-auto">
          <div className="text-[11px] font-semibold uppercase tracking-wider text-slate-500 px-2">
            Workspace Apps
          </div>
          {apps.map((a) => (
            <div
              key={a.path}
              onClick={() => setSelectedApp(a)}
              className={`p-2.5 rounded-lg border text-xs cursor-pointer transition-colors ${
                selectedApp?.path === a.path
                  ? 'bg-slate-800 border-brand-500/60 text-slate-100'
                  : 'bg-slate-900/40 border-slate-800 text-slate-400 hover:border-slate-700'
              }`}
            >
              <div className="font-semibold text-slate-200 truncate">{a.path}</div>
              <div className="text-[11px] text-slate-400 truncate mt-0.5">{a.summary}</div>
            </div>
          ))}
        </div>

        {/* Workspace Area */}
        <div className="flex-1 bg-slate-950 flex flex-col overflow-hidden">
          {activeTab === 'preview' && (
            <div className="flex-1 flex flex-col items-center justify-center p-8 bg-slate-950">
              <div className="w-full max-w-2xl bg-white rounded-xl shadow-2xl p-8 text-slate-900">
                <div className="flex items-center justify-between border-b pb-4 mb-4">
                  <div>
                    <h2 className="text-xl font-bold">Gantry Application Preview</h2>
                    <p className="text-sm text-slate-500">Live preview rendered with minted SDK preview token.</p>
                  </div>
                  <span className="text-xs bg-emerald-100 text-emerald-800 px-2.5 py-1 rounded-full font-semibold">
                    Guest Mode
                  </span>
                </div>
                <div className="p-6 bg-slate-50 rounded-lg border border-slate-200 space-y-4">
                  <div className="text-sm text-slate-700 font-medium">Interactive App Component:</div>
                  <div className="flex items-center gap-4">
                    <button className="bg-emerald-600 hover:bg-emerald-500 text-white font-bold px-4 py-2 rounded shadow transition-all">
                      Trigger Backend Script
                    </button>
                    <span className="text-xs text-slate-500">POST /w/demo/apps_u/execute_component/u/admin/demo_greeter</span>
                  </div>
                </div>
              </div>
            </div>
          )}

          {activeTab === 'source' && (
            <div className="flex-1 flex flex-col">
              <div className="px-4 py-2 bg-slate-900/60 border-b border-slate-800 text-xs font-mono text-slate-400 flex justify-between">
                <span>src/App.tsx</span>
                <span>Worker Bundle Target: ES2022</span>
              </div>
              <textarea
                value={appSource}
                onChange={(e) => setAppSource(e.target.value)}
                spellCheck={false}
                className="flex-1 w-full bg-slate-950 text-slate-100 font-mono text-xs p-4 resize-none focus:outline-none"
              />
            </div>
          )}

          {activeTab === 'data' && (
            <div className="p-6 overflow-y-auto space-y-6">
              <div>
                <h3 className="text-sm font-semibold text-slate-100 mb-1">S3 Data Helpers & Parquet Previews</h3>
                <p className="text-xs text-slate-400">
                  Built-in endpoints allow apps to stream and tabularly preview Parquet and CSV files from workspace S3 storage.
                </p>
              </div>

              <div className="p-4 rounded-xl border border-slate-800 bg-slate-900/40 space-y-3">
                <div className="font-mono text-xs text-brand-400">GET /w/&#123;workspace&#125;/apps_u/load_parquet_preview/f/data/customers.parquet</div>
                <div className="overflow-x-auto">
                  <table className="w-full text-left text-xs font-mono border-collapse">
                    <thead>
                      <tr className="border-b border-slate-800 text-slate-400">
                        <th className="p-2">id</th>
                        <th className="p-2">name</th>
                        <th className="p-2">tier</th>
                        <th className="p-2">spend_usd</th>
                      </tr>
                    </thead>
                    <tbody className="divide-y divide-slate-800/60 text-slate-300">
                      <tr>
                        <td className="p-2">101</td>
                        <td className="p-2">Acme Corp</td>
                        <td className="p-2">Enterprise</td>
                        <td className="p-2">$14,250.00</td>
                      </tr>
                      <tr>
                        <td className="p-2">102</td>
                        <td className="p-2">Starlight IO</td>
                        <td className="p-2">Growth</td>
                        <td className="p-2">$4,800.00</td>
                      </tr>
                    </tbody>
                  </table>
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
