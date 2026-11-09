import { create } from 'zustand'

interface AboutState {
  open: boolean
  setOpen: (open: boolean) => void
}

export const useAboutStore = create<AboutState>((set) => ({
  open: false,
  setOpen: (open) => set({ open }),
}))
