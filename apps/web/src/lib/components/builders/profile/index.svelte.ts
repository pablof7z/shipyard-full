import type NDK from '@nostr-dev-kit/ndk';
import type { NDKUser, NDKUserProfile } from '@nostr-dev-kit/ndk';

const inFlightRequests = new Map<string, Promise<NDKUserProfile | null>>();

export interface ProfileFetcherState {
  profile: NDKUserProfile | null;
  user: NDKUser | null;
  loading: boolean;
}

export interface ProfileFetcherConfig {
  user: NDKUser | string | null | undefined;
}

export function createProfileFetcher(
  config: () => ProfileFetcherConfig,
  getNdk: () => NDK
): ProfileFetcherState {
  const state = $state<{
    profile: NDKUserProfile | null;
    user: NDKUser | null;
    loading: boolean;
  }>({
    profile: null,
    user: null,
    loading: false
  });
  let requestId = 0;

  async function fetchProfile(payload: NDKUser | string) {
    const activeRequestId = ++requestId;
    state.loading = true;

    try {
      const ndk = getNdk();
      const ndkUser = typeof payload === 'string' ? await ndk.fetchUser(payload) : payload;
      if (activeRequestId !== requestId) return;
      if (!ndkUser) {
        state.profile = null;
        state.user = null;
        return;
      }

      if (!ndkUser.ndk) ndkUser.ndk = ndk;

      const pubkey = ndkUser.pubkey;
      if (ndkUser.profile) {
        state.profile = ndkUser.profile;
        state.user = ndkUser;
        return;
      }

      let fetchPromise = inFlightRequests.get(pubkey);
      if (!fetchPromise) {
        fetchPromise = ndkUser
          .fetchProfile({ closeOnEose: true, groupable: true, groupableDelay: 250 })
          .finally(() => inFlightRequests.delete(pubkey));
        inFlightRequests.set(pubkey, fetchPromise);
      }

      const fetchedProfile = (await fetchPromise) || null;
      if (activeRequestId !== requestId) return;
      state.profile = fetchedProfile;
      state.user = ndkUser;
    } catch {
      if (activeRequestId !== requestId) return;
      state.profile = null;
      state.user = null;
    } finally {
      if (activeRequestId === requestId) {
        state.loading = false;
      }
    }
  }

  $effect(() => {
    const { user } = config();
    if (user) {
      fetchProfile(user);
    } else {
      requestId += 1;
      state.profile = null;
      state.user = null;
      state.loading = false;
    }
  });

  return {
    get profile() {
      return state.profile;
    },
    get user() {
      return state.user;
    },
    get loading() {
      return state.loading;
    }
  };
}
