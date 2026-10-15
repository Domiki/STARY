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
    <circle cx="8" cy="8" r="2.2" />
    <path d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.4 3.4l1.4 1.4M11.2 11.2l1.4 1.4M12.6 3.4l-1.4 1.4M4.8 11.2l-1.4 1.4" />
  </svg>
)

export const StarIcon = (props: SVGProps<SVGSVGElement>) => (
  <svg {...base} {...props} fill="currentColor" stroke="none">
    <path d="M8 1 9.3 6.7 15 8 9.3 9.3 8 15 6.7 9.3 1 8 6.7 6.7 8 1Z" />
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
