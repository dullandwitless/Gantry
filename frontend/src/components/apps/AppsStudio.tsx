import React, { useState, useEffect } from 'react';
import { useWorkspace } from '../../context/WorkspaceContext';
import { useToast } from '../../context/ToastContext';
import { api } from '../../api/client';
import { AppExecutionMode, AppWithLastVersion, ListableApp } from '../../types';
import { AppWindow, Code2, Info } from 'lucide-react';

const EXECUTION_MODES: Record<AppExecutionMode, string> = {
  viewer: 'Runnables run as the user viewing the app',
  publisher: 'Runnables run as the app’s publisher',
  anonymous: 'Public: anyone with the link can use it',
};

/** Source files of a full-code app (`value.files`, path to content), if it has any. */
function appFiles(value: unknown): Record<string, string> | null {
  if (typeof value !== 'object' || value === null) return null;
  const files = (value as { files?: unknown }).files;
  if (typeof files !== 'object' || files === null) return null;
  const entries = Object.entries(files).filter(
    (entry): entry is [string, string] => typeof entry[1] === 'string',
  );
  return entries.length > 0 ? Object.fromEntries(entries) : null;
}

export const AppsStudio: React.FC = () => {
  const { currentWorkspace } = useWorkspace();
  const { addToast } = useToast();

  const [apps, setApps] = useState<ListableApp[]>([]);
  // Tied to its workspace, so switching workspace never fetches a path from the old one.
  const [selected, setSelected] = useState<{ workspace: string; path: string } | null>(null);
  const [app, setApp] = useState<AppWithLastVersion | null>(null);
  const [activeTab, setActiveTab] = useState<'overview' | 'definition'>('overview');
  const [openFile, setOpenFile] = useState<string | null>(null);

  useEffect(() => {
    if (!currentWorkspace) return;
    let cancelled = false;
    setApps([]);
    setSelected(null);
    api.listApps(currentWorkspace).then(
      (list) => {
        if (cancelled) return;
        setApps(list);
        setSelected(list[0] ? { workspace: currentWorkspace, path: list[0].path } : null);
      },
      (e: Error) => {
        if (!cancelled) addToast({ kind: 'error', title: 'Failed to load apps', message: e.message });
      },
    );
    return () => {
      cancelled = true;
    };
    // addToast is recreated on every render; reloading must depend only on the workspace.
  }, [currentWorkspace]);

  useEffect(() => {
    setApp(null);
    setOpenFile(null);
    if (!selected || selected.workspace !== currentWorkspace) return;
    let cancelled = false;
    api.getApp(selected.workspace, selected.path).then(
      (data) => {
        if (cancelled) return;
        setApp(data);
        const files = appFiles(data.value);
        setOpenFile(files ? Object.keys(files).sort()[0] : null);
      },
      (e: Error) => {
        if (!cancelled) addToast({ kind: 'error', title: 'Failed to load app', message: e.message });
      },
    );
    return () => {
      cancelled = true;
    };
  }, [currentWorkspace, selected]);

  const selectedPath = selected?.path ?? null;
  const files = app ? appFiles(app.value) : null;

  return (
    <div className="flex flex-col h-[calc(100vh-3.5rem)] overflow-hidden">
      <div className="h-12 border-b border-slate-800 bg-slate-900/40 px-4 flex items-center justify-between">
        <div className="flex items-center gap-3">
          <AppWindow className="w-4 h-4 text-brand-500" />
          <span className="font-semibold text-xs text-slate-200">Apps</span>
          {selectedPath && (
            <span className="text-xs font-mono text-slate-400 bg-slate-800 px-2 py-0.5 rounded border border-slate-700">
              {selectedPath}
            </span>
          )}
        </div>

        {app && (
          <div className="flex items-center gap-2">
            <TabButton active={activeTab === 'overview'} onClick={() => setActiveTab('overview')}>
              <Info className="w-3.5 h-3.5" />
              <span>Overview</span>
            </TabButton>
            <TabButton active={activeTab === 'definition'} onClick={() => setActiveTab('definition')}>
              <Code2 className="w-3.5 h-3.5" />
              <span>{files ? 'Source' : 'Definition'}</span>
            </TabButton>
          </div>
        )}
      </div>

      <div className="flex flex-1 overflow-hidden">
        <div className="w-64 border-r border-slate-800 p-3 space-y-2 overflow-y-auto">
          <div className="text-[11px] font-semibold uppercase tracking-wider text-slate-500 px-2">
            Workspace Apps
          </div>
          {apps.length === 0 && (
            <div className="px-2 text-xs text-slate-500">No apps in this workspace yet.</div>
          )}
          {apps.map((a) => (
            <div
              key={a.path}
              onClick={() => setSelected({ workspace: currentWorkspace, path: a.path })}
              className={`p-2.5 rounded-lg border text-xs cursor-pointer transition-colors ${
                selectedPath === a.path
                  ? 'bg-slate-800 border-brand-500/60 text-slate-100'
                  : 'bg-slate-900/40 border-slate-800 text-slate-400 hover:border-slate-700'
              }`}
            >
              <div className="font-semibold text-slate-200 truncate">{a.path}</div>
              {a.summary && <div className="text-[11px] text-slate-400 truncate mt-0.5">{a.summary}</div>}
            </div>
          ))}
        </div>

        <div className="flex-1 bg-slate-950 flex flex-col overflow-hidden">
          {!app && (
            <div className="flex-1 flex items-center justify-center text-xs text-slate-500">
              {selectedPath ? 'Loading…' : 'Select an app to see its details.'}
            </div>
          )}

          {app && activeTab === 'overview' && (
            <dl className="p-6 grid grid-cols-[10rem_1fr] gap-x-6 gap-y-3 text-xs content-start">
              <Field label="Summary">{app.summary || '—'}</Field>
              <Field label="Kind">{app.raw_app ? 'Full-code app' : 'Low-code app'}</Field>
              <Field label="Execution">{EXECUTION_MODES[app.policy.execution_mode] ?? app.policy.execution_mode}</Field>
              <Field label="Custom path">
                {app.custom_path ? <span className="font-mono">{app.custom_path}</span> : '—'}
              </Field>
              <Field label="Versions">{app.versions.length}</Field>
              <Field label="Created">
                {new Date(app.created_at).toLocaleString()} by {app.created_by}
              </Field>
            </dl>
          )}

          {app && activeTab === 'definition' && files && (
            <div className="flex flex-1 overflow-hidden">
              <div className="w-56 border-r border-slate-800 p-2 overflow-y-auto">
                {Object.keys(files)
                  .sort()
                  .map((name) => (
                    <div
                      key={name}
                      onClick={() => setOpenFile(name)}
                      className={`px-2 py-1 rounded font-mono text-[11px] cursor-pointer truncate ${
                        openFile === name ? 'bg-slate-800 text-slate-100' : 'text-slate-400 hover:text-slate-200'
                      }`}
                    >
                      {name}
                    </div>
                  ))}
              </div>
              <pre className="flex-1 overflow-auto p-4 font-mono text-xs text-slate-100">
                {openFile ? files[openFile] : ''}
              </pre>
            </div>
          )}

          {app && activeTab === 'definition' && !files && (
            <pre className="flex-1 overflow-auto p-4 font-mono text-xs text-slate-100">
              {JSON.stringify(app.value, null, 2)}
            </pre>
          )}
        </div>
      </div>
    </div>
  );
};

const TabButton: React.FC<{ active: boolean; onClick: () => void; children: React.ReactNode }> = ({
  active,
  onClick,
  children,
}) => (
  <button
    onClick={onClick}
    className={`px-3 py-1.5 rounded-lg text-xs font-medium transition-colors ${
      active ? 'bg-slate-800 text-slate-100 border border-slate-700' : 'text-slate-400 hover:text-slate-200'
    }`}
  >
    <span className="flex items-center gap-1.5">{children}</span>
  </button>
);

const Field: React.FC<{ label: string; children: React.ReactNode }> = ({ label, children }) => (
  <>
    <dt className="text-slate-500">{label}</dt>
    <dd className="text-slate-200">{children}</dd>
  </>
);
