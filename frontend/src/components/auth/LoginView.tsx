import React, { useState } from 'react';
import { useAuth } from '../../context/AuthContext';
import { useToast } from '../../context/ToastContext';
import { ShieldCheck, KeyRound, ArrowRight, Sparkles, Layers } from 'lucide-react';

export const LoginView: React.FC = () => {
  const { login, loginDemo } = useAuth();
  const { addToast } = useToast();

  const [email, setEmail] = useState('admin@windmill.dev');
  const [password, setPassword] = useState('changeme');
  const [isSubmitting, setIsSubmitting] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsSubmitting(true);
    try {
      await login(email, password);
      addToast({ kind: 'success', title: 'Authenticated', message: `Signed in as ${email}` });
    } catch (err: any) {
      addToast({ kind: 'error', title: 'Login failed', message: err.message || 'Check your credentials.' });
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleFillAdmin = () => {
    setEmail('admin@windmill.dev');
    setPassword('changeme');
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

        {/* Login Card */}
        <div className="bg-slate-900/80 border border-slate-800 rounded-2xl p-6 shadow-2xl backdrop-blur space-y-5">
          <form onSubmit={handleSubmit} className="space-y-4">
            <div className="space-y-1.5">
              <label className="text-xs font-medium text-slate-300">Email Address</label>
              <input
                type="email"
                required
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                placeholder="name@company.com"
                className="w-full bg-slate-950 border border-slate-700/80 rounded-xl px-3.5 py-2 text-xs text-slate-100 placeholder-slate-500 focus:outline-none focus:border-brand-500 transition-colors"
              />
            </div>

            <div className="space-y-1.5">
              <div className="flex justify-between items-center text-xs">
                <label className="font-medium text-slate-300">Password</label>
              </div>
              <input
                type="password"
                required
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                placeholder="••••••••••••"
                className="w-full bg-slate-950 border border-slate-700/80 rounded-xl px-3.5 py-2 text-xs text-slate-100 placeholder-slate-500 focus:outline-none focus:border-brand-500 transition-colors"
              />
            </div>

            <button
              type="submit"
              disabled={isSubmitting}
              className="w-full bg-brand-600 hover:bg-brand-500 text-slate-950 font-bold py-2.5 rounded-xl text-xs flex items-center justify-center gap-2 shadow-lg shadow-brand-600/20 transition-all disabled:opacity-50 mt-2"
            >
              <span>{isSubmitting ? 'Signing In...' : 'Sign In'}</span>
              <ArrowRight className="w-3.5 h-3.5" />
            </button>
          </form>

          {/* Quick Seeded Admin Helper */}
          <div className="pt-3 border-t border-slate-800/80">
            <div className="bg-slate-950/60 border border-slate-800/80 rounded-xl p-3 flex items-center justify-between text-xs">
              <div>
                <div className="text-[11px] text-slate-400">Default Seeded Admin:</div>
                <div className="font-mono text-[11px] text-brand-400 font-semibold mt-0.5">
                  admin@windmill.dev / changeme
                </div>
              </div>
              <button
                type="button"
                onClick={handleFillAdmin}
                className="px-2.5 py-1 rounded bg-slate-800 hover:bg-slate-700 text-slate-300 text-[10px] font-semibold border border-slate-700 transition-colors"
              >
                Autofill
              </button>
            </div>
          </div>

          {/* Demo Mode Bypass */}
          <div className="text-center pt-1">
            <button
              type="button"
              onClick={loginDemo}
              className="text-xs text-slate-400 hover:text-slate-200 inline-flex items-center gap-1.5 transition-colors"
            >
              <Sparkles className="w-3.5 h-3.5 text-brand-400" />
              <span>Explore Clean-Room UI in Demo Mode</span>
            </button>
          </div>
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
