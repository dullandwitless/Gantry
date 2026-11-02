import React, { createContext, useContext, useState, useEffect } from 'react';
import { api, UserProfile } from '../api/client';

interface AuthContextType {
  user: UserProfile | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  login: (email: string, pass: string) => Promise<void>;
  logout: () => Promise<void>;
  loginDemo: () => void;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

export const AuthProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [user, setUser] = useState<UserProfile | null>(null);
  const [isAuthenticated, setIsAuthenticated] = useState<boolean>(false);
  const [isLoading, setIsLoading] = useState<boolean>(true);

  useEffect(() => {
    checkAuth();
  }, []);

  const checkAuth = async () => {
    try {
      const profile = await api.whoami();
      if (profile) {
        setUser(profile);
        setIsAuthenticated(true);
      } else {
        // Check if demo session flag was set
        const demoMode = localStorage.getItem('gantry_demo_mode');
        if (demoMode === 'true') {
          setUser({
            email: 'admin@windmill.dev',
            username: 'admin',
            name: 'Demo Admin',
            super_admin: true,
          });
          setIsAuthenticated(true);
        }
      }
    } catch (e) {
      console.warn('Auth check failed:', e);
    } finally {
      setIsLoading(false);
    }
  };

  const login = async (email: string, pass: string) => {
    localStorage.removeItem('gantry_demo_mode');
    await api.login(email, pass);
    const profile = await api.whoami();
    setUser(profile || {
      email,
      username: email.split('@')[0],
      super_admin: true,
    });
    setIsAuthenticated(true);
  };

  const logout = async () => {
    localStorage.removeItem('gantry_demo_mode');
    await api.logout();
    setUser(null);
    setIsAuthenticated(false);
  };

  const loginDemo = () => {
    localStorage.setItem('gantry_demo_mode', 'true');
    setUser({
      email: 'admin@windmill.dev',
      username: 'admin',
      name: 'Demo Admin',
      super_admin: true,
    });
    setIsAuthenticated(true);
  };

  return (
    <AuthContext.Provider
      value={{
        user,
        isAuthenticated,
        isLoading,
        login,
        logout,
        loginDemo,
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
