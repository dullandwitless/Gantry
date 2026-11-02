import React, { createContext, useContext, useState, useEffect } from 'react';
import { api, UserProfile } from '../api/client';

interface AuthContextType {
  user: UserProfile | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  login: (email: string, pass: string) => Promise<void>;
  logout: () => Promise<void>;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

export const AuthProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [user, setUser] = useState<UserProfile | null>(null);
  const [isLoading, setIsLoading] = useState<boolean>(true);

  useEffect(() => {
    api
      .whoami()
      .then(setUser)
      .finally(() => setIsLoading(false));
  }, []);

  const login = async (email: string, pass: string) => {
    await api.login(email, pass);
    const profile = await api.whoami();
    if (!profile) {
      await api.logout();
      throw new Error('Signed in, but the server did not return the account. Try again.');
    }
    setUser(profile);
  };

  const logout = async () => {
    await api.logout();
    setUser(null);
  };

  return (
    <AuthContext.Provider
      value={{ user, isAuthenticated: user !== null, isLoading, login, logout }}
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
