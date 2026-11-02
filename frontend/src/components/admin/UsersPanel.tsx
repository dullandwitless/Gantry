import React, { useCallback, useEffect, useState } from 'react';
import { KeyRound, UserPlus } from 'lucide-react';
import { api } from '../../api/client';
import { useToast } from '../../context/ToastContext';
import { GlobalUser } from '../../types';
import { Modal, inputClass, primaryButtonClass } from '../common/Modal';
import {
  NewPasswordFields,
  emptyNewPassword,
  newPasswordProblem,
} from '../auth/NewPasswordFields';

/** Instance users for superadmins (specs/users-and-email.md, section 6). */
export const UsersPanel: React.FC = () => {
  const { addToast } = useToast();
  const [users, setUsers] = useState<GlobalUser[] | null>(null);
  const [isAddOpen, setIsAddOpen] = useState(false);
  const [passwordFor, setPasswordFor] = useState<string | null>(null);

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
                <th className="p-3" />
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
                    {u.login_type === 'pending_oauth' ? 'Pending SSO' : u.login_type}
                    {u.disabled && ' (disabled)'}
                  </td>
                  <td className="p-3 text-right">
                    <button
                      type="button"
                      onClick={() => setPasswordFor(u.email)}
                      title={`Set the password of ${u.email}`}
                      aria-label={`Set the password of ${u.email}`}
                      className="p-1.5 rounded text-slate-400 hover:text-slate-200 hover:bg-slate-800"
                    >
                      <KeyRound className="w-3.5 h-3.5" />
                    </button>
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
      {passwordFor !== null && (
        <SetPasswordModal
          email={passwordFor}
          onClose={() => setPasswordFor(null)}
          onSet={() => {
            setPasswordFor(null);
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
  const [password, setPassword] = useState(emptyNewPassword);
  const [superAdmin, setSuperAdmin] = useState(false);
  const [sendEmail, setSendEmail] = useState(true);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const problem = !email.trim() ? 'Enter an email address.' : newPasswordProblem(password);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (problem) return;
    setIsSubmitting(true);
    try {
      await api.createUser({
        email: email.trim(),
        password: password.password,
        super_admin: superAdmin,
        name: name.trim() || undefined,
        skip_email: !sendEmail,
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
        <NewPasswordFields value={password} onChange={setPassword} />
        <label className="flex items-center gap-2 text-xs text-slate-300">
          <input type="checkbox" checked={superAdmin} onChange={(e) => setSuperAdmin(e.target.checked)} />
          Superadmin
        </label>
        <label className="flex items-center gap-2 text-xs text-slate-300">
          <input type="checkbox" checked={sendEmail} onChange={(e) => setSendEmail(e.target.checked)} />
          Email them that the account exists (when email is configured; the password is not
          sent)
        </label>
        <div className="flex justify-end">
          <button type="submit" disabled={!!problem || isSubmitting} className={primaryButtonClass}>
            {isSubmitting ? 'Adding…' : 'Add user'}
          </button>
        </div>
      </form>
    </Modal>
  );
};

const SetPasswordModal: React.FC<{ email: string; onClose: () => void; onSet: () => void }> = ({
  email,
  onClose,
  onSet,
}) => {
  const { addToast } = useToast();
  const [value, setValue] = useState(emptyNewPassword);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const problem = newPasswordProblem(value);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (problem) return;
    setIsSubmitting(true);
    try {
      await api.setPasswordOf(email, value.password);
      addToast({ kind: 'success', title: 'Password set', message: `${email} has been signed out.` });
      onSet();
    } catch (err: any) {
      addToast({ kind: 'error', title: 'Could not set the password', message: err.message });
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <Modal title={`Set the password of ${email}`} onClose={onClose}>
      <form onSubmit={handleSubmit} className="space-y-4">
        <NewPasswordFields value={value} onChange={setValue} autoFocus />
        <p className="text-[11px] text-slate-400">
          This signs them out everywhere and makes password their sign-in method.
        </p>
        <div className="flex justify-end">
          <button type="submit" disabled={!!problem || isSubmitting} className={primaryButtonClass}>
            {isSubmitting ? 'Saving…' : 'Set password'}
          </button>
        </div>
      </form>
    </Modal>
  );
};
