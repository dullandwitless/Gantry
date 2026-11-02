import React, { createContext, useContext, useState, useEffect } from 'react';
import { api, SignInOptions, UserProfile } from '../api/client';

interface AuthContextType {
  user: UserProfile | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  /** How this instance signs users in (specs/identity-aware-proxy.md). */
  signInOptions: SignInOptions;
  /** Why signing in through Identity-Aware Proxy failed, when it did. */
  iapError: string | null;
  login: (email: string, pass: string) => Promise<void>;
  loginAsMockUser: (email: string) => Promise<void>;
  logout: () => Promise<void>;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

export const AuthProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [user, setUser] = useState<UserProfile | null>(null);
  const [isLoading, setIsLoading] = useState<boolean>(true);
  const [signInOptions, setSignInOptions] = useState<SignInOptions>({ iap: false, mock_users: [] });
  const [iapError, setIapError] = useState<string | null>(null);

  useEffect(() => {
    (async () => {
      const options = await api.getSignInOptions();
      setSignInOptions(options);
      // Behind IAP the session must belong to whoever the proxy admitted, so it is
      // exchanged on every load, replacing any earlier one.
      if (options.iap) {
        try {
          await api.iapLogin();
        } catch (e: any) {
          api.setToken(null);
          setIapError(e.message);
          return;
        }
      }
      setUser(await api.whoami());
    })().finally(() => setIsLoading(false));
  }, []);

  /** Runs a sign-in that stores a session token, then loads the account it belongs to. */
  const signIn = async (authenticate: () => Promise<unknown>) => {
    await authenticate();
    const profile = await api.whoami();
    if (!profile) {
      await api.logout();
      throw new Error('Signed in, but the server did not return the account. Try again.');
    }
    setUser(profile);
  };

  const login = (email: string, pass: string) => signIn(() => api.login(email, pass));

  const loginAsMockUser = (email: string) => signIn(() => api.mockLogin(email));

  const logout = async () => {
    await api.logout();
    setUser(null);
  };

  return (
    <AuthContext.Provider
      value={{
        user,
        isAuthenticated: user !== null,
        isLoading,
        signInOptions,
        iapError,
        login,
        loginAsMockUser,
        logout,
      }}
    >
      {children}
    </AuthContext.Provider>
  );
};

export const useAuth = () => {
  const context = useContext(AuthContext);
  if (!context) throw new Error('useAuth must be used within AuthProvider');
  return context;
};
