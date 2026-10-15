import type { Histogram, ImageView } from './types'

const HEADER_LEN_BYTES = 4

/** A JSON header followed by bytes: the length of the header (4 bytes), the header, the bytes. */
export function parsePacket<Header>(buffer: ArrayBuffer): { header: Header; body: Uint8Array<ArrayBuffer> } {
  const headerLength = new DataView(buffer).getUint32(0, true)
  const headerEnd = HEADER_LEN_BYTES + headerLength
  const header: Header = JSON.parse(
    new TextDecoder().decode(new Uint8Array(buffer, HEADER_LEN_BYTES, headerLength)),
  )
  return { header, body: new Uint8Array(buffer, headerEnd) }
}

interface ImageHeader {
  width: number
  height: number
  histogram: Histogram
}

export function parseImagePacket(buffer: ArrayBuffer): ImageView {
  const { header, body } = parsePacket<ImageHeader>(buffer)
  const jpeg = new Blob([body], { type: 'image/jpeg' })
  return { ...header, url: URL.createObjectURL(jpeg) }
}
