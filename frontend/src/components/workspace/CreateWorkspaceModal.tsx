import React, { useState } from 'react';
import { useWorkspace } from '../../context/WorkspaceContext';
import { useToast } from '../../context/ToastContext';
import { Layers, ArrowRight } from 'lucide-react';

interface Props {
  isOpen: boolean;
}

export const CreateWorkspaceModal: React.FC<Props> = ({ isOpen }) => {
  const { createNewWorkspace } = useWorkspace();
  const { addToast } = useToast();

  const [id, setId] = useState('main');
  const [name, setName] = useState('Main Workspace');
  const [isSubmitting, setIsSubmitting] = useState(false);

  if (!isOpen) return null;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsSubmitting(true);
    try {
      await createNewWorkspace(id.trim().toLowerCase(), name.trim());
      addToast({ kind: 'success', title: 'Workspace created', message: `Welcome to ${name}!` });
    } catch (err: any) {
      addToast({ kind: 'error', title: 'Failed to create workspace', message: err.message });
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 bg-slate-950/90 backdrop-blur-md flex items-center justify-center p-4">
      <div className="max-w-md w-full bg-slate-900 border border-slate-800 rounded-2xl p-6 shadow-2xl space-y-5">
        <div className="text-center space-y-2">
          <div className="w-12 h-12 rounded-2xl bg-brand-500/10 border border-brand-500/30 flex items-center justify-center mx-auto text-brand-500 mb-2">
            <Layers className="w-6 h-6" />
          </div>
          <h2 className="text-lg font-bold text-slate-100">Set Up the Workspace</h2>
          <p className="text-xs text-slate-400">
            Gantry has one workspace for scripts, flows, credentials and runs. Everyone who has signed
            in joins it.
          </p>
        </div>

        <form onSubmit={handleSubmit} className="space-y-4 text-xs">
          <div className="space-y-1">
            <label className="font-medium text-slate-300">Workspace Identifier (Slug)</label>
            <input
              type="text"
              required
              pattern="^[a-z0-9_-]+$"
              value={id}
              onChange={(e) => setId(e.target.value)}
              placeholder="e.g. main"
              className="w-full bg-slate-950 border border-slate-700/80 rounded-xl px-3 py-2 text-xs font-mono text-slate-100 placeholder-slate-500 focus:outline-none focus:border-brand-500"
            />
            <span className="text-[10px] text-slate-500 font-mono">Allowed: lowercase letters, numbers, dash, underscore</span>
          </div>

          <div className="space-y-1">
            <label className="font-medium text-slate-300">Display Name</label>
            <input
              type="text"
              required
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="e.g. Main Workspace"
              className="w-full bg-slate-950 border border-slate-700/80 rounded-xl px-3 py-2 text-xs text-slate-100 placeholder-slate-500 focus:outline-none focus:border-brand-500"
            />
          </div>

          <button
            type="submit"
            disabled={isSubmitting}
            className="w-full bg-brand-600 hover:bg-brand-500 text-slate-950 font-bold py-2.5 rounded-xl text-xs flex items-center justify-center gap-2 shadow-lg shadow-brand-600/20 transition-all disabled:opacity-50 mt-4"
          >
            <span>{isSubmitting ? 'Creating...' : 'Create Workspace'}</span>
            <ArrowRight className="w-3.5 h-3.5" />
          </button>
        </form>
      </div>
    </div>
  );
};
