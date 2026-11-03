import React from 'react';
import { useAuth } from '../../context/AuthContext';
import { useToast } from '../../context/ToastContext';
import { ShieldCheck } from 'lucide-react';

/**
 * Gantry signs people in through Identity-Aware Proxy, or as a mock user in local development
 * (specs/identity-aware-proxy.md, specs/single-organisation.md §1). There are no passwords.
 */
export const LoginView: React.FC = () => {
  const { loginAsMockUser, signInOptions, iapError } = useAuth();
  const { addToast } = useToast();

  const handleMockUser = async (mockEmail: string) => {
    try {
      await loginAsMockUser(mockEmail);
    } catch (err: any) {
      addToast({ kind: 'error', title: 'Login failed', message: err.message });
    }
  };

  return (
    <div className="min-h-screen w-screen bg-slate-950 flex flex-col justify-center items-center p-4 selection:bg-brand-600 selection:text-white">
      <div className="w-full max-w-md space-y-6">
        {/* Brand Header */}
        <div className="text-center space-y-2">
          <div className="inline-flex w-12 h-12 rounded-2xl bg-brand-500/10 border border-brand-500/30 items-center justify-center shadow-lg shadow-brand-500/10 mb-2">
            <span className="text-brand-500 font-black text-2xl">G</span>
          </div>
          <h1 className="text-2xl font-black tracking-tight text-slate-100">GANTRY</h1>
          <p className="text-xs text-slate-400">Clean-Room Workflow & Developer Platform</p>
        </div>

        <div className="bg-slate-900/80 border border-slate-800 rounded-2xl p-6 shadow-2xl backdrop-blur space-y-4 text-xs">
          {/* Behind Identity-Aware Proxy there is nothing to sign in with: only a failure lands here. */}
          {signInOptions.iap ? (
            <>
              <h2 className="text-sm font-semibold text-slate-100">Could not sign in through Identity-Aware Proxy</h2>
              <p className="text-red-400 break-words">{iapError ?? 'The proxy did not identify you.'}</p>
              <button
                type="button"
                onClick={() => window.location.reload()}
                className="w-full bg-brand-600 hover:bg-brand-500 text-slate-950 font-bold py-2.5 rounded-xl"
              >
                Try again
              </button>
            </>
          ) : signInOptions.mock_users.length > 0 ? (
            <div className="space-y-2">
              <div className="text-[11px] text-slate-400">Local development: mock users</div>
              {signInOptions.mock_users.map((mockEmail) => (
                <button
                  key={mockEmail}
                  type="button"
                  onClick={() => handleMockUser(mockEmail)}
                  className="w-full py-2 rounded-xl text-xs font-semibold text-slate-200 bg-slate-950 border border-slate-700/80 hover:border-brand-500 transition-colors"
                >
                  Continue as {mockEmail}
                </button>
              ))}
            </div>
          ) : (
            <>
              <h2 className="text-sm font-semibold text-slate-100">No way to sign in</h2>
              <p className="text-slate-400">
                Gantry signs people in through Identity-Aware Proxy (set <code>IAP_AUDIENCE</code>). For local
                development, list mock users in <code>GANTRY_MOCK_USERS</code>.
              </p>
            </>
          )}
        </div>

        {/* Provenance Footer */}
        <div className="flex items-center justify-center gap-1.5 text-xs text-slate-500">
          <ShieldCheck className="w-4 h-4 text-emerald-500" />
          <span>Clean-Room Frontend Implementation (PROVENANCE.md)</span>
        </div>
      </div>
    </div>
  );
};
