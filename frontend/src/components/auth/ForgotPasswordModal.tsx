import React, { useState } from 'react';
import { api } from '../../api/client';
import { Modal, inputClass, primaryButtonClass } from '../common/Modal';

interface Props {
  initialEmail: string;
  onClose: () => void;
}

/** Requests a password reset email (`POST /auth/request_password_reset`). */
export const ForgotPasswordModal: React.FC<Props> = ({ initialEmail, onClose }) => {
  const [email, setEmail] = useState(initialEmail);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [result, setResult] = useState<{ ok: boolean; message: string } | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsSubmitting(true);
    try {
      setResult({ ok: true, message: await api.requestPasswordReset(email.trim()) });
    } catch (err: any) {
      setResult({ ok: false, message: err.message });
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <Modal title="Reset your password" onClose={onClose}>
      {result?.ok ? (
        <div className="space-y-4">
          <p className="text-xs text-slate-300">{result.message}</p>
          <div className="flex justify-end">
            <button type="button" onClick={onClose} className={primaryButtonClass}>
              Done
            </button>
          </div>
        </div>
      ) : (
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
          <p className="text-[11px] text-slate-400">
            If an account uses this address, a link to choose a new password is sent to it. The
            link expires after one hour.
          </p>
          {result && <p className="text-[11px] text-rose-400">{result.message}</p>}
          <div className="flex justify-end">
            <button type="submit" disabled={isSubmitting} className={primaryButtonClass}>
              {isSubmitting ? 'Sending…' : 'Send reset link'}
            </button>
          </div>
        </form>
      )}
    </Modal>
  );
};
