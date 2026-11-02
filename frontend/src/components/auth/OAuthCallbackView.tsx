import React, { useEffect, useRef, useState } from 'react';
import { useAuth } from '../../context/AuthContext';
import { primaryButtonClass } from '../common/Modal';

/** Where sign-in providers send the browser back (specs/sso-oauth-login.md, section 4). */
export const OAUTH_CALLBACK_PREFIX = '/user/login_callback/';

/** The provider named by the current URL, when it is a sign-in callback. */
export function oauthCallbackProvider(pathname: string): string | null {
  if (!pathname.startsWith(OAUTH_CALLBACK_PREFIX)) return null;
  const provider = decodeURIComponent(pathname.slice(OAUTH_CALLBACK_PREFIX.length)).replace(/\/$/, '');
  return provider || null;
}

interface Props {
  provider: string;
  /** Leaves the page: into the app once signed in, else back to sign-in. */
  onDone: () => void;
}

/** Completes a provider sign-in with the `code` and `state` it put in the URL. */
export const OAuthCallbackView: React.FC<Props> = ({ provider, onDone }) => {
  const { loginWithProvider } = useAuth();
  const [error, setError] = useState<string | null>(null);
  // A code can be exchanged once; never send it twice (React may run effects twice in development).
  const started = useRef(false);

  useEffect(() => {
    if (started.current) return;
    started.current = true;

    const params = new URLSearchParams(window.location.search);
    const code = params.get('code');
    const state = params.get('state');
    if (!code || !state) {
      setError(params.get('error_description') || params.get('error') || 'The provider did not complete the sign-in.');
      return;
    }
    loginWithProvider(provider, code, state).then(onDone, (e: Error) => setError(e.message));
    // Runs once per visit to the callback URL.
  }, []);

  return (
    <div className="min-h-screen w-screen bg-slate-950 flex justify-center items-center p-4">
      <div className="w-full max-w-sm bg-slate-900/80 border border-slate-800 rounded-2xl p-6 shadow-2xl space-y-4 text-xs">
        {error === null ? (
          <div className="flex items-center gap-3 text-slate-300">
            <div className="w-5 h-5 rounded-full border-2 border-brand-500 border-t-transparent animate-spin" />
            <span>Signing in with {provider}…</span>
          </div>
        ) : (
          <>
            <h1 className="text-sm font-semibold text-slate-100">Sign-in with {provider} failed</h1>
            <p className="text-red-400 break-words">{error}</p>
            <button type="button" onClick={onDone} className={primaryButtonClass}>
              Back to sign-in
            </button>
          </>
        )}
      </div>
    </div>
  );
};
