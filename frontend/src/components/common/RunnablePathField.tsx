import React from 'react';
import { ListedRunnable } from '../../types';
import { RunnableSelection } from './useRunnablePicker';

/** Option value for "new item"; a real path is never empty. */
const NEW_OPTION = '';

interface Props {
  /** "script" or "flow", used in labels. */
  noun: string;
  items: ListedRunnable[];
  selection: RunnableSelection | null;
  onSelect: (selection: RunnableSelection) => void;
  /** The path typed for a new item. */
  newPath: string;
  onNewPathChange: (path: string) => void;
}

const fieldClass =
  'bg-slate-800/80 border border-slate-700/60 rounded px-2.5 py-1 text-xs font-mono text-slate-200 focus:outline-none focus:border-brand-500';

/** Picks an existing script or flow, or names a new one. Never fetches by itself. */
export const RunnablePathField: React.FC<Props> = ({
  noun, items, selection, onSelect, newPath, onNewPathChange,
}) => {
  if (selection === null) {
    return <span className="text-xs text-slate-500">Loading {noun}s…</span>;
  }

  return (
    <div className="flex items-center gap-2">
      <select
        aria-label={`Open ${noun}`}
        value={selection.kind === 'existing' ? selection.path : NEW_OPTION}
        onChange={(e) =>
          onSelect(e.target.value === NEW_OPTION ? { kind: 'new' } : { kind: 'existing', path: e.target.value })
        }
        className={`${fieldClass} w-64`}
      >
        <option value={NEW_OPTION}>New {noun}</option>
        {items.map((item) => (
          <option key={item.path} value={item.path}>
            {item.path}
          </option>
        ))}
      </select>
      {selection.kind === 'new' && (
        <input
          type="text"
          aria-label={`Path of the new ${noun}`}
          value={newPath}
          onChange={(e) => onNewPathChange(e.target.value)}
          placeholder={`u/<username>/my_${noun}`}
          className={`${fieldClass} w-56`}
        />
      )}
    </div>
  );
};
