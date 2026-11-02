import React, { useState } from 'react';
import { api } from '../../api/client';
import { useToast } from '../../context/ToastContext';
import { Modal, primaryButtonClass } from '../common/Modal';
import { NewPasswordFields, emptyNewPassword, newPasswordProblem } from './NewPasswordFields';

interface Props {
  onClose: () => void;
}

/** Sets the signed-in user's password (`POST /users/setpassword`). */
export const ChangePasswordModal: React.FC<Props> = ({ onClose }) => {
  const { addToast } = useToast();
  const [value, setValue] = useState(emptyNewPassword);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const problem = newPasswordProblem(value);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (problem) return;
    setIsSubmitting(true);
    try {
      await api.setOwnPassword(value.password);
      addToast({
        kind: 'success',
        title: 'Password changed',
        message: 'Your other sessions have been signed out.',
      });
      onClose();
    } catch (err: any) {
      addToast({ kind: 'error', title: 'Could not change the password', message: err.message });
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <Modal title="Change password" onClose={onClose}>
      <form onSubmit={handleSubmit} className="space-y-4">
        <NewPasswordFields value={value} onChange={setValue} autoFocus />
        <p className="text-[11px] text-slate-400">
          Changing your password signs you out everywhere else.
        </p>
        <div className="flex justify-end">
          <button type="submit" disabled={!!problem || isSubmitting} className={primaryButtonClass}>
            {isSubmitting ? 'Saving…' : 'Change password'}
          </button>
        </div>
      </form>
    </Modal>
  );
};
