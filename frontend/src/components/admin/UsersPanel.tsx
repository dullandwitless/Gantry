import React, { useCallback, useEffect, useState } from 'react';
import { UserPlus } from 'lucide-react';
import { api } from '../../api/client';
import { useToast } from '../../context/ToastContext';
import { GlobalUser } from '../../types';
import { Modal, inputClass, primaryButtonClass } from '../common/Modal';

/**
 * Instance users for superadmins (specs/users-and-email.md, section 6). An account added here
 * signs in through Identity-Aware Proxy; it has no password (specs/single-organisation.md §1).
 */
export const UsersPanel: React.FC = () => {
  const { addToast } = useToast();
  const [users, setUsers] = useState<GlobalUser[] | null>(null);
  const [isAddOpen, setIsAddOpen] = useState(false);

  const load = useCallback(() => {
    api.listGlobalUsers().then(setUsers, (e: Error) => {
      setUsers([]);
      addToast({ kind: 'error', title: 'Failed to load users', message: e.message });
    });
    // addToast is recreated on every render; loading must not depend on it.
  }, []);

  useEffect(load, [load]);

  return (
    <div className="space-y-4">
      <div className="flex justify-between items-center">
        <h2 className="text-sm font-semibold text-slate-200">Instance users</h2>
        <button
          type="button"
          onClick={() => setIsAddOpen(true)}
          className={`${primaryButtonClass} flex items-center gap-1.5`}
        >
          <UserPlus className="w-3.5 h-3.5" />
          <span>Add user</span>
        </button>
      </div>

      {users === null ? (
        <div className="text-xs text-slate-500">Loading users…</div>
      ) : (
        <div className="border border-slate-800 rounded-xl overflow-x-auto bg-slate-900/40">
          <table className="w-full text-left text-xs">
            <thead className="bg-slate-900/80 border-b border-slate-800 text-slate-400">
              <tr>
                <th className="p-3">Email</th>
                <th className="p-3">Name</th>
                <th className="p-3">Username</th>
                <th className="p-3">Role</th>
                <th className="p-3">Sign-in</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-800/60 text-slate-300">
              {users.map((u) => (
                <tr key={u.email} className={u.disabled ? 'opacity-50' : undefined}>
                  <td className="p-3 font-mono text-slate-200">{u.email}</td>
                  <td className="p-3">{u.name || '–'}</td>
                  <td className="p-3 font-mono">{u.username || '–'}</td>
                  <td className="p-3">{u.super_admin ? 'Superadmin' : u.devops ? 'Devops' : 'User'}</td>
                  <td className="p-3">
                    {u.login_type === 'pending_oauth' ? 'Not signed in yet' : u.login_type}
                    {u.disabled && ' (disabled)'}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {isAddOpen && (
        <AddUserModal
          onClose={() => setIsAddOpen(false)}
          onAdded={() => {
            setIsAddOpen(false);
            load();
          }}
        />
      )}
    </div>
  );
};

const AddUserModal: React.FC<{ onClose: () => void; onAdded: () => void }> = ({
  onClose,
  onAdded,
}) => {
  const { addToast } = useToast();
  const [email, setEmail] = useState('');
  const [name, setName] = useState('');
  const [superAdmin, setSuperAdmin] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const problem = !email.trim() ? 'Enter an email address.' : null;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (problem) return;
    setIsSubmitting(true);
    try {
      await api.createUser({
        email: email.trim(),
        super_admin: superAdmin,
        name: name.trim() || undefined,
      });
      addToast({ kind: 'success', title: 'User added', message: email.trim() });
      onAdded();
    } catch (err: any) {
      addToast({ kind: 'error', title: 'Could not add the user', message: err.message });
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <Modal title="Add user" onClose={onClose}>
      <form onSubmit={handleSubmit} className="space-y-4">
        <label className="block space-y-1.5">
          <span className="text-xs font-medium text-slate-300">Email address</span>
          <input
            type="email"
            required
            autoFocus
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            className={inputClass}
          />
        </label>
        <label className="block space-y-1.5">
          <span className="text-xs font-medium text-slate-300">Name (optional)</span>
          <input
            type="text"
            maxLength={255}
            value={name}
            onChange={(e) => setName(e.target.value)}
            className={inputClass}
          />
        </label>
        <label className="flex items-center gap-2 text-xs text-slate-300">
          <input type="checkbox" checked={superAdmin} onChange={(e) => setSuperAdmin(e.target.checked)} />
          Superadmin
        </label>
        <p className="text-[11px] text-slate-400">
          They sign in through Identity-Aware Proxy with this address and join the workspace then.
        </p>
        <div className="flex justify-end">
          <button type="submit" disabled={!!problem || isSubmitting} className={primaryButtonClass}>
            {isSubmitting ? 'Adding…' : 'Add user'}
          </button>
        </div>
      </form>
    </Modal>
  );
};
