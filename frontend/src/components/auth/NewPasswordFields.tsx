import React from 'react';
import { inputClass } from '../common/Modal';

export interface NewPassword {
  password: string;
  confirm: string;
}

export const emptyNewPassword: NewPassword = { password: '', confirm: '' };

/** Why `value` cannot be submitted yet, or `null` when it can. */
export const newPasswordProblem = (value: NewPassword): string | null => {
  if (!value.password) return 'Enter a password.';
  if (value.password !== value.confirm) return 'The passwords do not match.';
  return null;
};

interface Props {
  value: NewPassword;
  onChange: (value: NewPassword) => void;
  autoFocus?: boolean;
}

/** A new password entered twice. */
export const NewPasswordFields: React.FC<Props> = ({ value, onChange, autoFocus }) => {
  const mismatch = value.confirm !== '' && value.password !== value.confirm;
  return (
    <>
      <label className="block space-y-1.5">
        <span className="text-xs font-medium text-slate-300">New password</span>
        <input
          type="password"
          autoComplete="new-password"
          autoFocus={autoFocus}
          value={value.password}
          onChange={(e) => onChange({ ...value, password: e.target.value })}
          className={inputClass}
        />
      </label>
      <label className="block space-y-1.5">
        <span className="text-xs font-medium text-slate-300">Repeat new password</span>
        <input
          type="password"
          autoComplete="new-password"
          value={value.confirm}
          onChange={(e) => onChange({ ...value, confirm: e.target.value })}
          aria-invalid={mismatch}
          className={inputClass}
        />
      </label>
      {mismatch && <p className="text-[11px] text-rose-400">The passwords do not match.</p>}
    </>
  );
};
