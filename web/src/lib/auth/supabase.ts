import { createClient, type Session } from '@supabase/supabase-js';
import { config } from '../config';
import type { AccountSession } from '../types';
import type { AuthProvider } from './index';

export const supabase = createClient(
  config.supabase.url || 'https://placeholder.supabase.co',
  config.supabase.anonKey || 'placeholder',
  {
    auth: {
      persistSession: true,
      autoRefreshToken: true,
      detectSessionInUrl: true
    }
  }
);

function toAccountSession(s: Session | null): AccountSession | null {
  if (!s?.user) return null;
  const meta = s.user.user_metadata ?? {};
  return {
    userId: s.user.id,
    email: s.user.email ?? '',
    displayName: meta.full_name ?? meta.name ?? null,
    avatarUrl: meta.avatar_url ?? meta.picture ?? null
  };
}

export const supabaseAuth: AuthProvider = {
  async getSession() {
    const { data } = await supabase.auth.getSession();
    return toAccountSession(data.session);
  },

  onAuthChange(cb) {
    const { data } = supabase.auth.onAuthStateChange((_event, session) => {
      cb(toAccountSession(session));
    });
    return () => data.subscription.unsubscribe();
  },

  async signInWithGoogle(redirectTo) {
    await supabase.auth.signInWithOAuth({
      provider: 'google',
      options: { redirectTo }
    });
  },

  async signInWithEmail(email, redirectTo) {
    await supabase.auth.signInWithOtp({
      email,
      options: { emailRedirectTo: redirectTo }
    });
  },

  async signOut() {
    await supabase.auth.signOut();
  },

  async getAccessToken() {
    const { data } = await supabase.auth.getSession();
    return data.session?.access_token ?? null;
  }
};
