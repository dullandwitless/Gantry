import React, { createContext, useContext, useState } from 'react';

export type ToastKind = 'info' | 'success' | 'warning' | 'error';

export interface Toast {
  id: string;
  kind: ToastKind;
  title: string;
  message?: string;
}

interface ToastContextType {
  toasts: Toast[];
  addToast: (toast: Omit<Toast, 'id'>) => void;
  removeToast: (id: string) => void;
}

const ToastContext = createContext<ToastContextType | undefined>(undefined);

export const ToastProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [toasts, setToasts] = useState<Toast[]>([]);

  const addToast = (toast: Omit<Toast, 'id'>) => {
    const id = Math.random().toString(36).substring(2, 9);
    setToasts(prev => [...prev, { ...toast, id }]);
    setTimeout(() => {
      removeToast(id);
    }, 5000);
  };

  const removeToast = (id: string) => {
    setToasts(prev => prev.filter(t => t.id !== id));
  };

  return (
    <ToastContext.Provider value={{ toasts, addToast, removeToast }}>
      {children}
      <div className="fixed bottom-4 right-4 z-50 flex flex-col gap-2 max-w-sm w-full">
        {toasts.map(t => (
          <div
            key={t.id}
            className={`p-3 rounded-lg border shadow-lg flex justify-between items-start text-sm transition-all ${
              t.kind === 'success' ? 'bg-emerald-950/90 border-emerald-800 text-emerald-200' :
              t.kind === 'error' ? 'bg-rose-950/90 border-rose-800 text-rose-200' :
              t.kind === 'warning' ? 'bg-amber-950/90 border-amber-800 text-amber-200' :
              'bg-slate-900/90 border-slate-700 text-slate-200'
            }`}
          >
            <div>
              <div className="font-semibold">{t.title}</div>
              {t.message && <div className="text-xs opacity-90 mt-0.5">{t.message}</div>}
            </div>
            <button
              onClick={() => removeToast(t.id)}
              className="ml-2 text-slate-400 hover:text-white"
            >
              ✕
            </button>
          </div>
        ))}
      </div>
    </ToastContext.Provider>
  );
};

export const useToast = () => {
  const context = useContext(ToastContext);
  if (!context) throw new Error('useToast must be used within ToastProvider');
  return context;
};
