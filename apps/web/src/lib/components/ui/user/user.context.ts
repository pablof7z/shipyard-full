import type NDK from '@nostr-dev-kit/ndk';
import type { NDKUser, NDKUserProfile } from '@nostr-dev-kit/ndk';

export interface UserContext {
  ndk: NDK;
  user?: NDKUser;
  ndkUser: NDKUser | null;
  profile?: NDKUserProfile | null;
  onclick?: (event: MouseEvent) => void;
}

export const USER_CONTEXT_KEY = Symbol('user');
