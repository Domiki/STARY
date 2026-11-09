import type { SVGProps } from 'react'

const base: SVGProps<SVGSVGElement> = {
  width: 16,
  height: 16,
  viewBox: '0 0 16 16',
  fill: 'none',
  stroke: 'currentColor',
  strokeWidth: 1.5,
  strokeLinecap: 'round',
  strokeLinejoin: 'round',
}

export const ChevronIcon = ({ open, ...props }: SVGProps<SVGSVGElement> & { open: boolean }) => (
  <svg {...base} {...props} style={{ transform: open ? 'rotate(90deg)' : undefined }}>
    <path d="M6 3.5 10.5 8 6 12.5" />
  </svg>
)

export const PlusIcon = (props: SVGProps<SVGSVGElement>) => (
  <svg {...base} {...props}>
    <path d="M8 3v10M3 8h10" />
  </svg>
)

export const CloseIcon = (props: SVGProps<SVGSVGElement>) => (
  <svg {...base} {...props}>
    <path d="m4 4 8 8M12 4l-8 8" />
  </svg>
)

export const WarningIcon = (props: SVGProps<SVGSVGElement>) => (
  <svg {...base} {...props}>
    <path d="M8 2.5 14 13H2L8 2.5ZM8 6.5v3M8 11.2v.1" />
  </svg>
)

export const GearIcon = (props: SVGProps<SVGSVGElement>) => (
  <svg {...base} {...props}>
    <path d="M7.11 3.18 L7.22 1.14 L8.78 1.14 L8.89 3.18 L10.78 3.96 L12.30 2.60 L13.40 3.70 L12.04 5.22 L12.82 7.11 L14.86 7.22 L14.86 8.78 L12.82 8.89 L12.04 10.78 L13.40 12.30 L12.30 13.40 L10.78 12.04 L8.89 12.82 L8.78 14.86 L7.22 14.86 L7.11 12.82 L5.22 12.04 L3.70 13.40 L2.60 12.30 L3.96 10.78 L3.18 8.89 L1.14 8.78 L1.14 7.22 L3.18 7.11 L3.96 5.22 L2.60 3.70 L3.70 2.60 L5.22 3.96Z" />
    <circle cx="8" cy="8" r="2" />
  </svg>
)

export const RepresentativeIcon = (props: SVGProps<SVGSVGElement>) => (
  <svg {...base} {...props} fill="currentColor" stroke="none">
    <path d="m8 1.5 1.9 4.2 4.6.5-3.4 3.1 1 4.5L8 11.4l-4.1 2.4 1-4.5L1.5 6.2l4.6-.5L8 1.5Z" />
  </svg>
)

export const ZoomInIcon = (props: SVGProps<SVGSVGElement>) => (
  <svg {...base} {...props}>
    <circle cx="7" cy="7" r="4.5" />
    <path d="m10.5 10.5 3.5 3.5M7 5v4M5 7h4" />
  </svg>
)

export const ZoomOutIcon = (props: SVGProps<SVGSVGElement>) => (
  <svg {...base} {...props}>
    <circle cx="7" cy="7" r="4.5" />
    <path d="m10.5 10.5 3.5 3.5M5 7h4" />
  </svg>
)

export const FitIcon = (props: SVGProps<SVGSVGElement>) => (
  <svg {...base} {...props}>
    <path d="M2.5 6V2.5H6M10 2.5h3.5V6M13.5 10v3.5H10M6 13.5H2.5V10" />
  </svg>
)

export const SessionPlusIcon = (props: SVGProps<SVGSVGElement>) => (
  <svg {...base} {...props}>
    <rect x="2" y="3" width="9" height="9" rx="1.5" />
    <path d="M5 1.5h8.5V10M10 11.5h4M12 9.5v4" />
  </svg>
)

export const UndoIcon = (props: SVGProps<SVGSVGElement>) => (
  <svg {...base} {...props}>
    <path d="M5.5 3 2.5 6l3 3M2.5 6h6.2a3.8 3.8 0 0 1 0 7.6H6" />
  </svg>
)

export const RedoIcon = (props: SVGProps<SVGSVGElement>) => (
  <svg {...base} {...props}>
    <path d="m10.5 3 3 3-3 3M13.5 6H7.3a3.8 3.8 0 0 0 0 7.6H10" />
  </svg>
)
