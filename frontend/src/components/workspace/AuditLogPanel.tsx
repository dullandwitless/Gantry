import React, { useCallback, useEffect, useRef, useState } from 'react';
import { api } from '../../api/client';
import { useToast } from '../../context/ToastContext';
import { useWorkspace } from '../../context/WorkspaceContext';
import { AuditLogEntry, AuditLogFilters } from '../../types';
import { inputClass } from '../common/Modal';

const PAGE_SIZE = 100;

const ACTION_KINDS = ['Create', 'Update', 'Delete', 'Execute'] as const;

/** `datetime-local` input value (local time, no zone) to an ISO timestamp, or undefined. */
const toIso = (local: string): string | undefined =>
  local ? new Date(local).toISOString() : undefined;

/** The workspace's audit log (specs/audit-logs.md, section 4). */
export const AuditLogPanel: React.FC = () => {
  const { currentWorkspace } = useWorkspace();
  const { addToast } = useToast();

  const [draft, setDraft] = useState({ username: '', operation: '', action_kind: '', after: '', before: '' });
  const [filters, setFilters] = useState<AuditLogFilters>({});
  const [entries, setEntries] = useState<AuditLogEntry[] | null>(null);
  const [hasMore, setHasMore] = useState(false);
  const [isLoadingMore, setIsLoadingMore] = useState(false);
  const [expanded, setExpanded] = useState<number | null>(null);

  const addToastRef = useRef(addToast);
  addToastRef.current = addToast;
  // Discards a response that arrives after the filters or workspace changed.
  const generation = useRef(0);

  const load = useCallback(
    async (beforeId?: number) => {
      const current = ++generation.current;
      try {
        const page = await api.listAuditLogs(currentWorkspace, filters, PAGE_SIZE, beforeId);
        if (current !== generation.current) return;
        setEntries((prev) => (beforeId === undefined ? page : [...(prev ?? []), ...page]));
        setHasMore(page.length === PAGE_SIZE);
      } catch (e: any) {
        if (current !== generation.current) return;
        if (beforeId === undefined) setEntries([]);
        addToastRef.current({ kind: 'error', title: 'Failed to load the audit log', message: e.message });
      }
    },
    [currentWorkspace, filters],
  );

  useEffect(() => {
    setEntries(null);
    setExpanded(null);
    if (currentWorkspace) void load();
  }, [currentWorkspace, load]);

  const applyFilters = (e: React.FormEvent) => {
    e.preventDefault();
    setFilters({
      username: draft.username.trim() || undefined,
      operation: draft.operation.trim() || undefined,
      action_kind: draft.action_kind || undefined,
      after: toIso(draft.after),
      before: toIso(draft.before),
    });
  };

  const loadMore = async () => {
    if (!entries?.length) return;
    setIsLoadingMore(true);
    await load(entries[entries.length - 1].id);
    setIsLoadingMore(false);
  };

  return (
    <div className="space-y-4">
      <div>
        <h2 className="text-sm font-semibold text-slate-200">Audit log</h2>
        <p className="text-[11px] text-slate-400 mt-1">
          Workspace admins see everyone's actions; other members see their own.
        </p>
      </div>

      <form onSubmit={applyFilters} className="grid grid-cols-2 md:grid-cols-6 gap-2 items-end">
        <input
          aria-label="Username"
          placeholder="Username"
          value={draft.username}
          onChange={(e) => setDraft({ ...draft, username: e.target.value })}
          className={inputClass}
        />
        <input
          aria-label="Operation"
          placeholder="Operation (prefix)"
          value={draft.operation}
          onChange={(e) => setDraft({ ...draft, operation: e.target.value })}
          className={inputClass}
        />
        <select
          aria-label="Action kind"
          value={draft.action_kind}
          onChange={(e) => setDraft({ ...draft, action_kind: e.target.value })}
          className={inputClass}
        >
          <option value="">Any action</option>
          {ACTION_KINDS.map((k) => (
            <option key={k} value={k}>
              {k}
            </option>
          ))}
        </select>
        <input
          type="datetime-local"
          aria-label="After"
          title="After"
          value={draft.after}
          onChange={(e) => setDraft({ ...draft, after: e.target.value })}
          className={inputClass}
        />
        <input
          type="datetime-local"
          aria-label="Before"
          title="Before"
          value={draft.before}
          onChange={(e) => setDraft({ ...draft, before: e.target.value })}
          className={inputClass}
        />
        <button
          type="submit"
          className="bg-slate-800 hover:bg-slate-700 text-slate-200 border border-slate-700 rounded-lg px-3 py-2 text-xs"
        >
          Filter
        </button>
      </form>

      {entries === null ? (
        <div className="text-xs text-slate-500">Loading…</div>
      ) : entries.length === 0 ? (
        <div className="text-xs text-slate-500 italic">No audit entries match.</div>
      ) : (
        <div className="border border-slate-800 rounded-xl overflow-x-auto bg-slate-900/40">
          <table className="w-full text-left text-xs">
            <thead className="bg-slate-900/80 border-b border-slate-800 text-slate-400">
              <tr>
                <th className="p-3">Time</th>
                <th className="p-3">User</th>
                <th className="p-3">Operation</th>
                <th className="p-3">Action</th>
                <th className="p-3">Resource</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-800/60 text-slate-300">
              {entries.map((entry) => (
                <React.Fragment key={entry.id}>
                  <tr
                    onClick={() => setExpanded(expanded === entry.id ? null : entry.id)}
                    className="cursor-pointer hover:bg-slate-800/40"
                    aria-expanded={expanded === entry.id}
                  >
                    <td className="p-3 whitespace-nowrap">{new Date(entry.timestamp).toLocaleString()}</td>
                    <td className="p-3 font-mono">{entry.username}</td>
                    <td className="p-3 font-mono">{entry.operation}</td>
                    <td className="p-3">{entry.action_kind}</td>
                    <td className="p-3 font-mono break-all">{entry.resource || '–'}</td>
                  </tr>
                  {expanded === entry.id && (
                    <tr>
                      <td colSpan={5} className="p-3 bg-slate-950/60">
                        <pre className="text-[11px] text-slate-300 overflow-x-auto">
                          {entry.parameters ? JSON.stringify(entry.parameters, null, 2) : 'No parameters.'}
                        </pre>
                      </td>
                    </tr>
                  )}
                </React.Fragment>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {hasMore && (
        <button
          type="button"
          onClick={loadMore}
          disabled={isLoadingMore}
          className="text-xs text-brand-400 hover:text-brand-300 disabled:opacity-50"
        >
          {isLoadingMore ? 'Loading…' : 'Load more'}
        </button>
      )}
    </div>
  );
};
