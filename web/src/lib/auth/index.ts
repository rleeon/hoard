import type { AccountSession } from '../types';

/**
 * AuthProvider, swap-in interface. Today: Supabase. Tomorrow: anything
 * that can produce a JWT and a session. Components and routes must
 * depend on this interface, never on the impl module directly.
 */
export interface AuthProvider {
  getSession(): Promise<AccountSession | null>;
  onAuthChange(cb: (session: AccountSession | null) => void): () => void;
  signInWithGoogle(redirectTo: string): Promise<void>;
  signInWithGithub(redirectTo: string): Promise<void>;
  signInWithEmail(email: string, redirectTo: string, captchaToken?: string): Promise<void>;
  signOut(): Promise<void>;
  getAccessToken(): Promise<string | null>;
}

import { supabaseAuth } from './supabase';

export const auth: AuthProvider = supabaseAuth;
