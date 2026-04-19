let open = $state(false);

export const loginModal = {
  get open() {
    return open;
  },
  show() {
    open = true;
  },
  close() {
    open = false;
  }
};
