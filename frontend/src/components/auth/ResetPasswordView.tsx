import React, { useState } from 'react';
import { api } from '../../api/client';
import { primaryButtonClass } from '../common/Modal';
import { NewPasswordFields, emptyNewPassword, newPasswordProblem } from './NewPasswordFields';

/** Path of the link in the password reset email. */
export const RESET_PASSWORD_PATH = '/user/reset-password';

interface Props {
  token: string;
  /** Leaves the page, back to sign-in. */
  onDone: () => void;
}

/** Sets a new password from a reset link (`POST /auth/reset_password`). */
export const ResetPasswordView: React.FC<Props> = ({ token, onDone }) => {
  const [value, setValue] = useState(emptyNewPassword);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [done, setDone] = useState(false);
  const problem = newPasswordProblem(value);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (problem) return;
    setIsSubmitting(true);
    setError(null);
    try {
      await api.resetPassword(token, value.password);
      setDone(true);
    } catch (err: any) {
      setError(err.message);
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="min-h-screen w-screen bg-slate-950 flex justify-center items-center p-4">
      <div className="w-full max-w-sm bg-slate-900/80 border border-slate-800 rounded-2xl p-6 shadow-2xl space-y-4">
        <h1 className="text-base font-bold text-slate-100">Choose a new password</h1>
        {done ? (
          <>
            <p className="text-xs text-slate-300">Your password has been changed.</p>
            <button type="button" onClick={onDone} className={`${primaryButtonClass} w-full`}>
              Sign in
            </button>
          </>
        ) : !token ? (
          <>
            <p className="text-xs text-rose-400">This link has no reset token. Request a new one.</p>
            <button type="button" onClick={onDone} className={`${primaryButtonClass} w-full`}>
              Back to sign-in
            </button>
          </>
        ) : (
          <form onSubmit={handleSubmit} className="space-y-4">
            <NewPasswordFields value={value} onChange={setValue} autoFocus />
            {error && <p className="text-[11px] text-rose-400">{error}</p>}
            <button
              type="submit"
              disabled={!!problem || isSubmitting}
              className={`${primaryButtonClass} w-full`}
            >
              {isSubmitting ? 'Saving…' : 'Set password'}
            </button>
            <button
              type="button"
              onClick={onDone}
              className="w-full text-xs text-slate-400 hover:text-slate-200"
            >
              Back to sign-in
            </button>
          </form>
        )}
      </div>
    </div>
  );
};
