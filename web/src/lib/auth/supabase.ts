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

/**
 * Drop THIS browser tab's copy of the session without revoking it server-side.
 *
 * Used right after a desktop OAuth handoff: the app now holds the same
 * refresh-token family this tab does. Two clients rotating one refresh token
 * trip GoTrue's reuse-detection ("refresh_token_already_used"), which revokes
 * the whole family → both the browser and the app get signed out. So this tab
 * must stop touching that family. We must NOT call `supabase.auth.signOut()`:
 * even `{ scope: 'local' }` issues `POST /logout?scope=local`, which revokes
 * the shared session on the server and signs the desktop app out as soon as
 * its access token expires. Instead we kill the in-memory auto-refresh timer
 * and delete the persisted copy by hand, the tokens we handed the app stay
 * valid.
 */
export async function dropLocalSession(): Promise<void> {
  try {
    await supabase.auth.stopAutoRefresh();
  } catch {
    // best-effort
  }
  try {
    // Mirrors supabase-js's default storageKey: `sb-<ref>-auth-token`.
    const ref = new URL(config.supabase.url).hostname.split('.')[0];
    window.localStorage.removeItem(`sb-${ref}-auth-token`);
  } catch {
    // no localStorage / malformed URL: nothing to drop
  }
}

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
    const { error } = await supabase.auth.signInWithOAuth({
      provider: 'google',
      options: { redirectTo }
    });
    if (error) throw error;
  },

  async signInWithGithub(redirectTo) {
    const { error } = await supabase.auth.signInWithOAuth({
      provider: 'github',
      options: { redirectTo }
    });
    if (error) throw error;
  },

  async signInWithEmail(email, redirectTo, captchaToken) {
    const { error } = await supabase.auth.signInWithOtp({
      email,
      options: {
        emailRedirectTo: redirectTo,
        // Only sent when Turnstile is configured; GoTrue ignores it when
        // captcha protection is off, and requires it when it's on.
        ...(captchaToken ? { captchaToken } : {})
      }
    });
    if (error) throw error;
  },

  async signOut() {
    await supabase.auth.signOut();
  },

  async getAccessToken() {
    const { data } = await supabase.auth.getSession();
    return data.session?.access_token ?? null;
  }
};
